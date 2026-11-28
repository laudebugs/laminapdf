// Select over the objects a page already drew (Paper A05): which are read, which are picked, what
// a press, a drag and a marquee do to the pick, and the one `EditContent` op each change makes.
// The content controller owns the pointer and the layer; this owns the picks. Each change reads
// the page again, because an edit renames the objects it touches, and the pick follows its
// objects by their place in the painting order.

import type { ContentEdit, Op, PageObject } from "@laminapdf/sdk";
import { laterCountMessage, laterMessage } from "@laminapdf/ui/editing";
import { handleFrame, movedFrame } from "./drag.ts";
import { type Box, boxAround, boxOf, HANDLES, type HandleName, type Matrix, type Point, reshaped } from "./geometry.ts";
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

/** The name of a kind of pick in the undo steps and announcements. */
function kindName(kind: PickKind): string {
  return laterMessage(`content.original.kind.${kind}`);
}

/** Select over what the page already drew, for one open document. */
export class Originals {
  readonly #host: OriginalsHost;
  readonly #objects = new Map<number, readonly PageObject[]>();
  readonly #originals = new Map<number, readonly Original[]>();
  /** The crop each image was last given, as insets (left, top, right, bottom): the page can't say. */
  readonly #crops = new Map<string, readonly [number, number, number, number]>();
  #pick: Picked | undefined;
  #hover: { readonly page: number; readonly box: Box } | undefined;
  #drag: Drag | undefined;
  #frame: Matrix | undefined;
  #marquee: { readonly page: number; readonly box: Box } | undefined;
  /** The page's objects for a pick of ordinals to follow once the page is read again. */
  #follow: readonly number[] | undefined;

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

  /** The page's objects in painting order, as the page draws them (every one, whether Select lists it or not). */
  allOn(page: number): readonly PageObject[] {
    return this.#objects.get(page) ?? [];
  }

  /** The crop `original` was last given in this session, if any. */
  cropOf(original: Original): readonly [number, number, number, number] | undefined {
    return this.#crops.get(`${original.page}:${ordinalOf(original.ref)}`);
  }

  /** Reads `pages` again; the pick keeps the objects it had, or those named by the change that asked. */
  async read(pages: readonly number[]): Promise<void> {
    const keep = this.#follow ?? this.#pick?.objects.map((o) => ordinalOf(o.ref));
    this.#follow = undefined;
    await Promise.all(
      pages.map(async (page) => {
        const objects = await this.#host.contentObjects(page).catch((error: unknown) => {
          console.error("contentObjects failed", error);
          return undefined;
        });
        if (objects === undefined) return;
        this.#objects.set(page, objects);
        this.#originals.set(page, originalsOn(page, objects, this.#host.itemsOn(page)));
      }),
    );
    const page = this.#pick?.page;
    if (page !== undefined && keep !== undefined && pages.includes(page))
      this.#pick = followed(page, keep, this.on(page));
    if (this.#drag === undefined) this.#frame = undefined;
    this.#host.changed();
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
   * the pick) and starts moving the pick; over nothing, starts a marquee. Whether the press started a drag.
   */
  down(page: number, point: Point, shift: boolean): boolean {
    const hit = hitOriginal(this.on(page), point);
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
      const far = Math.hypot(point.x - drag.from.x, point.y - drag.from.y) >= CLICK;
      drag.moved = drag.moved || far;
      const others = this.on(pick.page)
        .filter((o) => !pick.objects.includes(o))
        .map((o) => boxFrame(o.box));
      this.#frame = movedFrame(
        boxFrame(pick.box),
        drag.from,
        point,
        this.#host.pageSize(pick.page),
        others,
        noSnap,
      ).frame;
    } else if (pick !== undefined && drag.kind === "handle") {
      this.#frame = handleFrame(boxFrame(pick.box), false, drag.handle, drag.from, point, shift);
    }
    this.#host.changed();
  }

  /** The press was let go, or taken away (`cancel`). */
  up(point: Point, cancel: boolean): void {
    const drag = this.#drag;
    const frame = this.#frame;
    this.#drag = undefined;
    this.#frame = undefined;
    const marquee = this.#marquee;
    this.#marquee = undefined;
    if (drag === undefined || cancel) {
      this.#host.changed();
      return;
    }
    if (drag.kind === "marquee") this.#finishMarquee(drag, point, marquee !== undefined);
    else if (frame !== undefined && drag.kind === "handle") {
      void this.setFrame(frame, drag.handle === "turn" ? "turn" : "resize");
    } else if (frame !== undefined && drag.kind === "move" && drag.moved) void this.setFrame(frame, "move");
    // A click on one of several picked objects, with no drag, picks just that one.
    else if (drag.kind === "move" && (this.#pick?.objects.length ?? 0) > 1) this.choose(drag.hit.page, [drag.hit]);
    else this.#host.changed();
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

  /** The frame of the pick as a drag has it now. */
  frame(): Matrix | undefined {
    const pick = this.#pick;
    return this.#frame ?? (pick === undefined ? undefined : boxFrame(pick.box));
  }

  /** The handles the layer shows, and whether the pick is shown without any (an object lamina can't change safely). */
  handles(): { readonly names: readonly HandleName[]; readonly refusal: string | undefined } {
    const refusal = this.#pick === undefined ? undefined : pickRefusal(this.#pick);
    return { names: HANDLES, refusal };
  }

  /** What the layer draws of hover and marquee. */
  view(): { readonly hover?: { page: number; box: Box }; readonly marquee?: { page: number; box: Box } } {
    return {
      ...(this.#hover === undefined ? {} : { hover: this.#hover }),
      ...(this.#marquee === undefined ? {} : { marquee: this.#marquee }),
    };
  }

  // Changes.

  /** The label of a step on the pick. */
  #label(step: "move" | "resize" | "turn" | "delete" | "arrange", pick: Picked): string {
    return laterMessage(`content.original.step.${step}`, { kind: this.#kindOf(pick) });
  }

  /** The pick's kind in a step's name: "image", "text block", "4 shapes". */
  #kindOf(pick: Picked): string {
    const kind = pickKind(pick);
    return pick.objects.length > 1 ? `${pick.objects.length} ${kindName(kind)}` : kindName(kind);
  }

  /** Makes `edits` on the pick's page as one op and one step; whether it was made. */
  async #edit(pick: Picked, edits: readonly ContentEdit[], label: string, follow: readonly number[]): Promise<boolean> {
    if (edits.length === 0) return false;
    this.#follow = follow;
    const op = { op: "editContent", args: { page: this.#host.pageId(pick.page), edits: [...edits] } } as unknown as Op;
    const made = await this.#host.apply([op], label, [pick.page]);
    this.#follow = undefined;
    return made;
  }

  /** Moves, resizes or turns the pick so its frame is `frame`. */
  async setFrame(frame: Matrix, step: "move" | "resize" | "turn" = "move"): Promise<boolean> {
    const pick = this.#pick;
    if (pick === undefined || pickRefusal(pick) !== undefined) return false;
    const edits = transformEdits(pick, boxFrame(pick.box), frame);
    // A turned group's box is the box around it, so what it was turned from isn't kept: the page is read again.
    return this.#edit(
      pick,
      edits,
      this.#label(step, pick),
      pick.objects.map((o) => ordinalOf(o.ref)),
    );
  }

  /** Moves the pick by (`dx`, `dy`) points: the arrow keys. */
  async nudge(dx: number, dy: number): Promise<boolean> {
    const frame = this.frame();
    const pick = this.#pick;
    if (frame === undefined || pick === undefined) return false;
    return this.setFrame([frame[0], frame[1], frame[2], frame[3], frame[4] + dx, frame[5] + dy]);
  }

  /** Sets X, Y, width or height from the inspector, as one edit. */
  async setBox(change: Partial<Record<"x" | "y" | "width" | "height" | "rotate", number>>): Promise<boolean> {
    const pick = this.#pick;
    if (pick === undefined) return false;
    const frame = reshaped(boxFrame(pick.box), change);
    const step =
      change.rotate !== undefined
        ? "turn"
        : change.width !== undefined || change.height !== undefined
          ? "resize"
          : "move";
    return this.setFrame(frame, step);
  }

  /** Deletes the pick. */
  async delete(): Promise<boolean> {
    const pick = this.#pick;
    if (pick === undefined || pickRefusal(pick) !== undefined) return false;
    const made = await this.#edit(pick, deleteEdits(pick), this.#label("delete", pick), []);
    if (made) this.clear();
    return made;
  }

  /** Restacks the pick. */
  async arrange(how: Arrange): Promise<boolean> {
    const pick = this.#pick;
    if (pick === undefined || pickRefusal(pick) !== undefined) return false;
    const all = this.allOn(pick.page);
    const moving = pick.objects.map((o) => ordinalOf(o.ref));
    const after = restacked(
      all.map((o) => ordinalOf(o.object)),
      moving,
      how,
    );
    const follow = moving.map((n) => after.indexOf(n));
    return this.#edit(pick, reorderEdits(pick, all, how), this.#label("arrange", pick), follow);
  }

  /** Cuts the one picked image to `insets` (left, top, right, bottom) from its box. */
  async crop(insets: readonly [number, number, number, number]): Promise<boolean> {
    const pick = this.#pick;
    const [image] = pick?.objects ?? [];
    if (pick === undefined || image === undefined || pick.objects.length !== 1 || image.kind !== "image") return false;
    const [left, top, right, bottom] = insets.map((v) => Math.max(0, v));
    const [x0, y0, x1, y1] = image.box;
    const rect: Box = [
      x0 + (left ?? 0),
      y0 + (top ?? 0),
      Math.max(x0 + (left ?? 0) + 1, x1 - (right ?? 0)),
      Math.max(y0 + (top ?? 0) + 1, y1 - (bottom ?? 0)),
    ];
    this.#crops.set(`${image.page}:${ordinalOf(image.ref)}`, [left ?? 0, top ?? 0, right ?? 0, bottom ?? 0]);
    return this.#edit(pick, [cropEdit(image, rect)], laterMessage("content.original.step.crop"), [
      ordinalOf(image.ref),
    ]);
  }

  /** Puts the picture in blob `blob` in the place of the one picked image; its page is read again. */
  async replaceImage(blob: string): Promise<boolean> {
    const pick = this.#pick;
    const [image] = pick?.objects ?? [];
    if (pick === undefined || image === undefined || image.kind !== "image") return false;
    const edit = { replaceImage: { object: image.ref, blob } } as unknown as ContentEdit;
    return this.#edit(pick, [edit], laterMessage("content.original.step.replace"), [ordinalOf(image.ref)]);
  }

  /** The announcement of what is picked: "4 shapes selected, part of the original page". */
  announcement(): string {
    const pick = this.#pick;
    if (pick === undefined) return "";
    const kind = kindName(pickKind(pick));
    return pick.objects.length === 1
      ? laterMessage("content.original.selected.one", { kind: kind.charAt(0).toLocaleUpperCase() + kind.slice(1) })
      : laterMessage("content.original.selected.many", { count: pick.objects.length, kind });
  }

  /** The count and size summary the inspector's title uses. */
  title(pick: Picked): string {
    const kind = pickKind(pick);
    if (pick.objects.length > 1 && kind === "shape")
      return laterCountMessage(pick.objects.length, {
        one: "content.original.shapes.one",
        many: "content.original.shapes.many",
      });
    if (pick.objects.length > 1) return laterMessage("content.original.mixed", { count: pick.objects.length });
    return laterMessage(
      kind === "image"
        ? "content.original.image"
        : kind === "text"
          ? "content.original.text"
          : "content.original.shapes.one",
    );
  }

  /** The inspector's position and size for a pick. A pick is boxed upright, so its turn reads 0. */
  box(pick: Picked): ReturnType<typeof boxOf> {
    const frame = this.frame() ?? boxFrame(pick.box);
    return { ...boxOf(frame), rotate: 0 };
  }
}
