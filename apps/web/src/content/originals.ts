// Select over the objects a page already drew (Paper A05): which are read, which are picked, what
// a press, a drag and a marquee do to the pick, and the one `EditContent` op each change makes.
// The content controller owns the pointer and the layer; this owns the picks. Each change reads
// the page again, because an edit renames the objects it touches, and the pick follows its
// objects by their place in the painting order. What the page can't say (how an image was cut) is
// kept per step of the history, so Undo and Redo bring it back with the page.

import type { ContentEdit, Op, PageObject } from "@laminapdf/sdk";
import { laterCountMessage, laterMessage } from "@laminapdf/ui/editing";
import { handleFrame, movedFrame } from "./drag.ts";
import {
  type Box,
  boxAround,
  boxOf,
  HANDLES,
  type HandleName,
  type Matrix,
  moved,
  type Point,
  reshaped,
} from "./geometry.ts";
import type { Item } from "./objects.ts";
import {
  type Arrange,
  boxFrame,
  cropEdit,
  deleteEdits,
  followed,
  hitOriginal,
  inMarquee,
  type Original,
  ordinalOf,
  originalsOn,
  type Picked,
  type PickKind,
  pickKind,
  pickOf,
  pickRefusal,
  reorderEdits,
  restacked,
  transformEdits,
} from "./original.ts";

/** What Select over existing objects needs of Edit mode. */
export interface OriginalsHost {
  /** The objects page `page` draws. */
  readonly contentObjects: (page: number) => Promise<PageObject[]>;
  /** The objects Lamina PDF added to `page`. */
  readonly itemsOn: (page: number) => readonly Item[];
  readonly pageId: (page: number) => string;
  readonly pageSize: (page: number) => { readonly width: number; readonly height: number } | undefined;
  /** Applies `ops` as one step named `label` and reads `pages` again; whether it was made. */
  readonly apply: (ops: readonly Op[], label: string, pages: readonly number[]) => Promise<boolean>;
  /** Runs `work` once every change before it is made and its page read again. */
  readonly enqueue: <T>(work: () => Promise<T>) => Promise<T>;
  /** As `apply`, for work already running in the queue. */
  readonly applyNow: (ops: readonly Op[], label: string, pages: readonly number[]) => Promise<boolean>;
  /** Where the edit history stands: it changes with every step, Undo and Redo. */
  readonly position: () => number;
  /** The pick or the drag changed: draw it, tell the inspector. */
  readonly changed: () => void;
  /** Takes the selection off an added object. */
  readonly clearAdded: () => void;
}

/** A pointer drag over a pick. */
type Drag =
  | { readonly kind: "move"; readonly from: Point; readonly hit: Original; moved: boolean }
  | { readonly kind: "handle"; readonly handle: HandleName | "turn"; readonly from: Point }
  | { readonly kind: "marquee"; readonly page: number; readonly from: Point; readonly shift: boolean };

/** How far a pointer travels before a press becomes a drag, in points. */
const CLICK = 3;

/** How long arrow-key presses gather into one step, in milliseconds. */
const NUDGE_MS = 400;

/** A page whose size isn't known: nothing is cut to it. */
const HUGE = { width: Number.POSITIVE_INFINITY, height: Number.POSITIVE_INFINITY };

/** An image's crop as insets from its box: left, top, right, bottom. */
export type Insets = readonly [number, number, number, number];

/** What the page can't say, by the page and ordinal of the object it is about. */
interface Memory {
  readonly crops: ReadonlyMap<string, Insets>;
  /** The picture that was last replaced. */
  readonly replaced: string | undefined;
}

/** An object's key in what is remembered: its page and ordinal. */
const keyOf = (page: number, ordinal: number): string => `${page}:${ordinal}`;

/** What one change makes: its edits, the step's name, the ordinals the pick takes, and how it moves what is remembered. */
interface Change {
  readonly edits: readonly ContentEdit[];
  readonly label: string;
  readonly follow: readonly number[];
  /** Where each ordinal goes; undefined for one that goes. Every ordinal stays when left out. */
  readonly remap?: (ordinal: number) => number | undefined;
  /** What the change itself adds to what is remembered. */
  readonly note?: { readonly crop?: { readonly ordinal: number; readonly insets: Insets }; readonly replaced?: number };
}

/** A change that makes no edit. */
const NOTHING: Change = { edits: [], label: "", follow: [] };

/** The name of a kind of pick, counted: "image", "4 shapes". */
function kindName(kind: PickKind, count: number): string {
  return laterCountMessage(
    count,
    { one: `content.original.kind.${kind}`, many: `content.original.kinds.${kind}` },
    { count },
  );
}

/** Where each ordinal goes once `deleted` ones are gone. */
function afterDeleting(deleted: readonly number[]): (ordinal: number) => number | undefined {
  return (n) => (deleted.includes(n) ? undefined : n - deleted.filter((d) => d < n).length);
}

/** Select over what the page already drew, for one open document. */
export class Originals {
  readonly #host: OriginalsHost;
  readonly #objects = new Map<number, readonly PageObject[]>();
  readonly #originals = new Map<number, readonly Original[]>();
  /** What the page can't say, as each step of the history left it. */
  readonly #memory = new Map<number, Memory>();
  #crops: ReadonlyMap<string, Insets> = new Map();
  #replaced: string | undefined;
  #pick: Picked | undefined;
  #hover: { readonly page: number; readonly box: Box } | undefined;
  #drag: Drag | undefined;
  #frame: Matrix | undefined;
  #marquee: { readonly page: number; readonly box: Box } | undefined;
  #nudge: { dx: number; dy: number; timer: ReturnType<typeof setTimeout> } | undefined;
  /** The frame a nudge that is being made shows until the page has it. */
  #flight: Matrix | undefined;
  /** The ordinals a pick takes once the page is read again, when a change moves them. */
  #follow: readonly number[] | undefined;
  #applying = false;

  constructor(host: OriginalsHost) {
    this.#host = host;
  }

  get pick(): Picked | undefined {
    return this.#pick;
  }

  get dragging(): boolean {
    return this.#drag !== undefined;
  }

  /** The page the drag in progress is on. */
  dragPage(): number | undefined {
    return this.#drag?.kind === "marquee" ? this.#drag.page : this.#pick?.page;
  }

  /** The objects of `page` as Select lists them, once read. */
  on(page: number): readonly Original[] {
    return this.#originals.get(page) ?? [];
  }

  /** Whether `page` has been read. */
  has(page: number): boolean {
    return this.#originals.has(page);
  }

  /** The page's objects in painting order, every one, whether Select lists it or not. */
  allOn(page: number): readonly PageObject[] {
    return this.#objects.get(page) ?? [];
  }

  /** The crop `original` was last given, if any. */
  cropOf(original: Original): Insets | undefined {
    return this.#crops.get(keyOf(original.page, ordinalOf(original.ref)));
  }

  /** The picture that was last replaced on `page`, if it is still there. */
  replacedOn(page: number): Original | undefined {
    const at = this.#replaced;
    return at === undefined
      ? undefined
      : this.on(page).find((o) => o.kind === "image" && keyOf(page, ordinalOf(o.ref)) === at);
  }

  /** Reads `pages` again; the pick keeps the objects it has, or those named by the change that asked. */
  async read(pages: readonly number[]): Promise<void> {
    const follow = this.#follow;
    this.#follow = undefined;
    await Promise.all(pages.map((page) => this.#readPage(page)));
    // What the pick is now, not what it was before the page was read: a newer pick stays.
    const keep = follow ?? this.#pick?.objects.map((o) => ordinalOf(o.ref));
    const page = this.#pick?.page;
    if (page !== undefined && keep !== undefined && pages.includes(page)) {
      this.#pick = followed(page, keep, this.on(page));
    }
    this.#restoreMemory();
    if (this.#drag === undefined && this.#nudge === undefined) this.#frame = undefined;
    this.#host.changed();
  }

  async #readPage(page: number): Promise<void> {
    const objects = await this.#host.contentObjects(page).catch((error: unknown) => {
      console.error("contentObjects failed", error);
      return undefined;
    });
    if (objects === undefined) return;
    this.#objects.set(page, objects);
    this.#originals.set(page, originalsOn(page, objects, this.#host.itemsOn(page), this.#host.pageSize(page) ?? HUGE));
  }

  /**
   * What the page can't say, as the history's present step left it (Undo and Redo move it). A step
   * this class didn't make (a text change, an added object) keeps what the nearest step before it
   * remembered.
   */
  #restoreMemory(): void {
    // A change of this class remembers its own step once it is made.
    if (this.#applying) return;
    let at: Memory | undefined;
    for (let position = this.#host.position(); position >= 0 && at === undefined; position--) {
      at = this.#memory.get(position);
    }
    this.#crops = at?.crops ?? new Map();
    this.#replaced = at?.replaced;
  }

  /** Keeps what the page can't say for the step the history is at now, once `change` has moved the ordinals of `page`. */
  #remember(change: Change, page: number): void {
    const remap = change.remap ?? ((n: number) => n);
    const mine = `${page}:`;
    const moves = (key: string): string | undefined => {
      if (!key.startsWith(mine)) return key;
      const to = remap(Number(key.slice(mine.length)));
      return to === undefined ? undefined : keyOf(page, to);
    };
    const crops = new Map<string, Insets>();
    for (const [key, insets] of this.#crops) {
      const to = moves(key);
      if (to !== undefined) crops.set(to, insets);
    }
    const { crop, replaced } = change.note ?? {};
    if (crop !== undefined) crops.set(keyOf(page, crop.ordinal), crop.insets);
    const was = this.#replaced === undefined ? undefined : moves(this.#replaced);
    this.#crops = crops;
    this.#replaced = replaced === undefined ? was : keyOf(page, replaced);
    const position = this.#host.position();
    // A step that branches the history leaves what it remembered above it behind.
    for (const later of [...this.#memory.keys()]) if (later > position) this.#memory.delete(later);
    this.#memory.set(position, { crops, replaced: this.#replaced });
  }

  /** Clears the pick; true if there was one. */
  clear(): boolean {
    const had = this.#pick !== undefined;
    this.#pick = undefined;
    this.#frame = undefined;
    if (had) this.#host.changed();
    return had;
  }

  /** Picks `objects` of `page` (or clears the pick). */
  choose(page: number, objects: readonly Original[]): void {
    this.#pick = pickOf(page, objects);
    this.#frame = undefined;
    if (this.#pick !== undefined) this.#host.clearAdded();
    this.#host.changed();
  }

  // Pointer input.

  /** The pointer moved over `page` with nothing held: outlines what a press would pick. */
  hoverAt(page: number, point: Point): void {
    const found = hitOriginal(this.on(page), point);
    const same = found?.box === this.#hover?.box;
    this.#hover = found === undefined ? undefined : { page, box: found.box };
    if (!same) this.#host.changed();
  }

  /** Takes the hover outline away. */
  unhover(): void {
    if (this.#hover === undefined) return;
    this.#hover = undefined;
    this.#host.changed();
  }

  /**
   * A press at `point` on `page`: picks the object under it (Shift adds it to, or takes it out of,
   * the pick; Alt takes a background the press would otherwise go through) and starts moving the
   * pick; over nothing, starts a marquee. Whether the press started a drag.
   */
  down(page: number, point: Point, shift: boolean, alt = false): boolean {
    const hit = hitOriginal(this.on(page), point, alt);
    this.#hover = undefined;
    if (hit === undefined) {
      this.#drag = { kind: "marquee", page, from: point, shift };
      return true;
    }
    const held = this.#pick?.page === page ? this.#pick.objects : [];
    const inPick = held.some((o) => o.ref === hit.ref);
    if (shift) {
      this.choose(page, inPick ? held.filter((o) => o.ref !== hit.ref) : [...held, hit]);
      return false;
    }
    if (!inPick) this.choose(page, [hit]);
    this.#drag = { kind: "move", from: point, hit, moved: false };
    return true;
  }

  /** A turn or resize handle of the pick was pressed. */
  startHandle(handle: HandleName | "turn", from: Point): void {
    this.#drag = { kind: "handle", handle, from };
  }

  /** The pointer moved with a drag held. */
  move(point: Point, noSnap: boolean, shift: boolean): void {
    const drag = this.#drag;
    const pick = this.#pick;
    if (drag === undefined) return;
    if (drag.kind === "marquee") {
      this.#marquee = { page: drag.page, box: boxAround([drag.from, point]) };
    } else if (pick !== undefined && drag.kind === "move") {
      drag.moved = drag.moved || Math.hypot(point.x - drag.from.x, point.y - drag.from.y) >= CLICK;
      const others = this.on(pick.page)
        .filter((o) => !pick.objects.includes(o))
        .map((o) => boxFrame(o.box));
      const size = this.#host.pageSize(pick.page);
      this.#frame = movedFrame(boxFrame(pick.box), drag.from, point, size, others, noSnap).frame;
    } else if (pick !== undefined && drag.kind === "handle") {
      this.#frame = handleFrame(boxFrame(pick.box), false, drag.handle, drag.from, point, shift);
    }
    this.#host.changed();
  }

  /** The press was let go, or taken away (`cancel`). */
  up(point: Point, cancel: boolean): void {
    const drag = this.#drag;
    const frame = this.#frame;
    const marquee = this.#marquee;
    this.#drag = undefined;
    this.#frame = undefined;
    this.#marquee = undefined;
    if (drag === undefined || cancel) {
      this.#host.changed();
    } else if (drag.kind === "marquee") {
      this.#finishMarquee(drag, point, marquee !== undefined);
    } else if (frame !== undefined && drag.kind === "handle") {
      void this.setFrame(frame, drag.handle === "turn" ? "turn" : "resize");
    } else if (frame !== undefined && drag.kind === "move" && drag.moved) {
      void this.setFrame(frame, "move");
    } else if (drag.kind === "move" && (this.#pick?.objects.length ?? 0) > 1) {
      // A click on one of several picked objects, with no drag, picks just that one.
      this.choose(drag.hit.page, [drag.hit]);
    } else {
      this.#host.changed();
    }
  }

  #finishMarquee(drag: Extract<Drag, { kind: "marquee" }>, point: Point, dragged: boolean): void {
    if (!dragged) {
      if (!drag.shift) this.clear();
      else this.#host.changed();
      return;
    }
    const taken = inMarquee(this.on(drag.page), drag.from, point);
    const held = drag.shift && this.#pick?.page === drag.page ? this.#pick.objects : [];
    this.choose(drag.page, [...held, ...taken.filter((o) => !held.includes(o))]);
  }

  // What the layer draws.

  /** The frame of the pick as a drag or a gathering nudge has it now. */
  frame(): Matrix | undefined {
    const pick = this.#pick;
    if (pick === undefined) return undefined;
    const nudge = this.#nudge;
    const base = this.#frame ?? this.#flight ?? boxFrame(pick.box);
    return nudge === undefined ? base : moved(base, nudge.dx, nudge.dy);
  }

  /** The handles the layer shows, and why the pick is shown without any when an object can't be changed safely. */
  handles(): { readonly names: readonly HandleName[]; readonly refusal: string | undefined } {
    return { names: HANDLES, refusal: this.#pick === undefined ? undefined : pickRefusal(this.#pick) };
  }

  /** What the layer draws of hover and marquee. */
  view(): { readonly hover?: { page: number; box: Box }; readonly marquee?: { page: number; box: Box } } {
    return {
      ...(this.#hover === undefined ? {} : { hover: this.#hover }),
      ...(this.#marquee === undefined ? {} : { marquee: this.#marquee }),
    };
  }

  // Changes.

  #label(step: "move" | "resize" | "turn" | "delete" | "arrange", pick: Picked): string {
    return laterMessage(`content.original.step.${step}`, { kind: kindName(pickKind(pick), pick.objects.length) });
  }

  /**
   * Makes the change `build` gives for the pick as one op and one step; whether it was made. It is
   * built once the changes before it are done, from the pick as they left it, so a run of changes
   * never names an object by what it was before the last one.
   */
  #edit(build: (pick: Picked) => Change): Promise<boolean> {
    // The pick and its names are read when the change's turn comes, not when it was asked for.
    return this.#host.enqueue(async () => {
      const pick = this.#pick;
      if (pick === undefined || pickRefusal(pick) !== undefined) return false;
      const change = build(pick);
      if (change.edits.length === 0) return false;
      this.#follow = change.follow;
      const op: Op = { op: "editContent", args: { page: this.#host.pageId(pick.page), edits: [...change.edits] } };
      this.#applying = true;
      const made = await this.#host.applyNow([op], change.label, [pick.page]).finally(() => {
        this.#applying = false;
        this.#follow = undefined;
      });
      if (made) this.#remember(change, pick.page);
      return made;
    });
  }

  /** The change that moves the pick into `frame`. */
  #moveTo(pick: Picked, frame: Matrix, step: "move" | "resize" | "turn"): Change {
    return {
      edits: transformEdits(pick, boxFrame(pick.box), frame),
      label: this.#label(step, pick),
      follow: pick.objects.map((o) => ordinalOf(o.ref)),
    };
  }

  /** Moves, resizes or turns the pick so its frame is `frame`. */
  setFrame(frame: Matrix, step: "move" | "resize" | "turn" = "move"): Promise<boolean> {
    return this.#edit((pick) => this.#moveTo(pick, frame, step));
  }

  /** Moves the pick by (`dx`, `dy`) points: the arrow keys, which gather for a moment into one step. */
  nudge(dx: number, dy: number): void {
    if (this.#pick === undefined) return;
    if (this.#nudge !== undefined) clearTimeout(this.#nudge.timer);
    const total = { dx: (this.#nudge?.dx ?? 0) + dx, dy: (this.#nudge?.dy ?? 0) + dy };
    this.#nudge = { ...total, timer: setTimeout(() => void this.flush(), NUDGE_MS) };
    this.#host.changed();
  }

  /** Makes the gathering nudge one step now (before Undo, a save, a new pick). */
  async flush(): Promise<void> {
    const nudge = this.#nudge;
    if (nudge === undefined) return;
    clearTimeout(nudge.timer);
    // Taken before the step is awaited, so a nudge asked for meanwhile gathers on its own and no
    // press moves the pick twice. The pick shows the moved frame until the step is made.
    this.#nudge = undefined;
    const pick = this.#pick;
    this.#flight = pick === undefined ? undefined : moved(boxFrame(pick.box), nudge.dx, nudge.dy);
    await this.#edit((now) => this.#moveTo(now, moved(boxFrame(now.box), nudge.dx, nudge.dy), "move"));
    this.#flight = undefined;
    this.#host.changed();
  }

  /** Sets X, Y, width, height or the turn from the inspector, as one edit on the pick as it is by then. */
  setBox(change: Partial<Record<"x" | "y" | "width" | "height" | "rotate", number>>): Promise<boolean> {
    const resizing = change.width !== undefined || change.height !== undefined;
    const step = change.rotate !== undefined ? "turn" : resizing ? "resize" : "move";
    return this.#edit((pick) => this.#moveTo(pick, reshaped(boxFrame(pick.box), change), step));
  }

  /** Deletes the pick. */
  async delete(): Promise<boolean> {
    const made = await this.#edit((pick) => ({
      edits: deleteEdits(pick),
      label: this.#label("delete", pick),
      follow: [],
      remap: afterDeleting(pick.objects.map((o) => ordinalOf(o.ref))),
    }));
    if (made) this.clear();
    return made;
  }

  /** Restacks the pick. */
  arrange(how: Arrange): Promise<boolean> {
    return this.#edit((pick) => {
      const all = this.allOn(pick.page);
      const moving = pick.objects.map((o) => ordinalOf(o.ref));
      const after = restacked(
        all.map((o) => ordinalOf(o.object)),
        moving,
        how,
      );
      return {
        edits: reorderEdits(pick, all, how),
        label: this.#label("arrange", pick),
        follow: moving.map((n) => after.indexOf(n)),
        remap: (n) => Math.max(after.indexOf(n), 0),
      };
    });
  }

  /** Cuts the one picked image to `insets` from its box. */
  crop(insets: Insets): Promise<boolean> {
    return this.#edit((pick) => {
      const [image] = pick.objects;
      if (image === undefined || pick.objects.length !== 1 || image.kind !== "image") return NOTHING;
      const [left, top, right, bottom] = insets.map((v) => Math.max(0, v)) as unknown as Insets;
      const [x0, y0, x1, y1] = image.box;
      const rect: Box = [x0 + left, y0 + top, Math.max(x0 + left + 1, x1 - right), Math.max(y0 + top + 1, y1 - bottom)];
      const ordinal = ordinalOf(image.ref);
      return {
        edits: [cropEdit(image, rect)],
        label: laterMessage("content.original.step.crop"),
        follow: [ordinal],
        note: { crop: { ordinal, insets: [left, top, right, bottom] } },
      };
    });
  }

  /** Puts the picture in blob `blob` in the place of the one picked image; its page is read again. */
  replaceImage(blob: string): Promise<boolean> {
    return this.#edit((pick) => {
      const [image] = pick.objects;
      if (image === undefined || image.kind !== "image") return NOTHING;
      const ordinal = ordinalOf(image.ref);
      return {
        edits: [{ replaceImage: { object: image.ref, blob } } as ContentEdit],
        label: laterMessage("content.original.step.replace"),
        follow: [ordinal],
        note: { replaced: ordinal },
      };
    });
  }

  /** The announcement of what is picked: "4 shapes selected, part of the original page". */
  announcement(): string {
    const pick = this.#pick;
    return pick === undefined ? "" : laterMessage("content.original.selected", { what: this.title(pick) });
  }

  /** What a pick is called: "Image", "Text block", "4 shapes". */
  title(pick: Picked): string {
    const kind = pickKind(pick);
    return pick.objects.length === 1
      ? laterMessage(`content.original.name.${kind}`)
      : kindName(kind, pick.objects.length);
  }

  /** The inspector's position and size for a pick. A pick is boxed upright, so its turn reads 0. */
  box(pick: Picked): ReturnType<typeof boxOf> {
    const frame = this.frame() ?? boxFrame(pick.box);
    return { ...boxOf(frame), rotate: 0 };
  }
}
