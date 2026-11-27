// Edit mode's state over one open document (Paper A01, A02): the tool on, the objects Lamina PDF
// added to the pages in view, the selection and its handles, the text being typed, and every
// change as one op and one undo step. Pointer input comes through two interaction modes (select
// and place) and the layer's handles; the inspector's changes are `inspector-props.ts`'s. Nothing
// here reads a clock but the ops' dates, so a drag moves only the overlay and the page redraws
// once, when the edit is made (E6f).

import type { ContentChanges, Op, TextBlock } from "@laminapdf/sdk";
import type { Platform, ShellLayout, ToastQueue, ToolModule } from "@laminapdf/ui";
import { type ContentShape, type ContentTool, laterMessage, type ToolRegistry } from "@laminapdf/ui/editing";
import type { Viewer } from "@laminapdf/viewer";
import { DocumentFonts } from "../fonts/document-fonts.ts";
import { type FontChoice, FontFileError, FontRegistry } from "../fonts/registry.ts";
import { pointOn, unturn } from "../page-points.ts";
import type { Engine } from "../session.ts";
import {
  type Box,
  boxAround,
  contains,
  corners,
  dragBox,
  type Guide,
  HANDLES,
  type HandleName,
  lineEnd,
  type Matrix,
  moved,
  nudgeOf,
  type Point,
  resized,
  sizeOf,
  snapMove,
  TEXT_HANDLES,
  turnedTo,
} from "./geometry.ts";
import { fittedSize, type ReadImage, readImage, stemOf } from "./images.ts";
import { CONTENT_INSPECTOR } from "./inspector-id.ts";
import { ContentLayer, type Draft, type LayerHost } from "./layer.ts";
import { type Item, itemOf, matrixFor, nameOf } from "./objects.ts";
import {
  addImageOp,
  addShapeOp,
  addTextOp,
  colour,
  deleteOp,
  modifiedNow,
  newId,
  pathOf,
  type ShapeKind,
  type ShapeLook,
  type TextLook,
  updateOp,
} from "./ops.ts";
import { css, cssFamily, INK, type Look, lookOfBlock, SHAPE_DEFAULT, TEXT_DEFAULT } from "./text-look.ts";
import { heightFor } from "./text-measure.ts";

/** Edit mode's two interaction modes: selecting what was added, and placing something new. */
declare module "@laminapdf/ui" {
  interface InteractionModes {
    contentSelect: true;
    contentPlace: true;
  }
}

/** Smaller than this, in points, a drag is a click. */
const CLICK = 3;
/** A click places a shape this big, in points. */
const DEFAULT_SHAPE = 72;
/** A click places a text box at least this wide, in points. */
const NARROWEST_TEXT = 120;
/** How long arrow-key nudges gather before they are one step, in ms. */
const NUDGE_MS = 400;

/** The fonts of a session: the registry names them, and each document takes their bytes once. */
const registry = new FontRegistry();

/** The box text is being typed into. */
export interface Typing {
  readonly page: number;
  /** The object being edited; undefined for a new box. */
  readonly id: string | undefined;
  readonly rect: Box;
  readonly text: string;
  readonly look: Look;
}

/** Edit mode's state. */
export interface ContentState {
  readonly tool: ContentTool;
  readonly shape: ContentShape;
  /** The selected object. */
  readonly selected: Item | undefined;
  readonly typing: Typing | undefined;
  /** What the next new text is set in. */
  readonly look: Look;
  /** What the next shape is drawn with. */
  readonly shapeLook: ShapeLook;
  readonly keepRatio: boolean;
  /** The symbol picker is open. */
  readonly symbols: boolean;
  /** A file that wasn't a picture lamina can place. */
  readonly unsupported: string | undefined;
  /** What this session knows of pictures it placed, bumped when it changes. */
  readonly known: number;
}

/** What Edit mode needs of the app. */
export interface ContentHost {
  readonly platform: Platform;
  readonly toasts: ToastQueue;
}

/** The hint under the toolbar for `tool` (and `shape`, when it is the Shape tool). Pure, so a render that reads it is redone when they change. */
export function hintFor(tool: ContentTool, shape: ContentShape): string {
  return laterMessage(tool === "shape" ? `content.hint.${shape}` : (`content.hint.${tool}` as "content.hint.text"));
}

/** A pointer drag in progress. */
type Drag =
  | { readonly kind: "move"; readonly item: Item; readonly from: Point }
  | { readonly kind: "handle"; readonly item: Item; readonly handle: HandleName | "turn"; readonly from: Point }
  | { readonly kind: "text"; readonly page: number; readonly from: Point }
  | { readonly kind: "shape"; readonly page: number; readonly from: Point; readonly shape: ShapeKind };

/** The picture facts a session keeps for the inspector. */
export interface PictureFacts {
  readonly name: string;
  readonly width: number;
  readonly height: number;
  readonly type: "image/png" | "image/jpeg";
  readonly smaller: boolean;
}

/** Edit mode's state over one open document. */
export class ContentController {
  readonly #engine: Engine;
  readonly #viewer: Viewer;
  readonly #tools: ToolRegistry;
  readonly #layout: ShellLayout;
  readonly #host: ContentHost;
  readonly #fonts: DocumentFonts;
  readonly #layer: ContentLayer;
  readonly #listeners = new Set<() => void>();
  readonly #items = new Map<number, readonly Item[]>();
  readonly #facts = new Map<string, PictureFacts>();
  readonly #objectFonts = new Map<string, string>();
  #state: ContentState;
  #on = false;
  #stops: (() => void)[] = [];
  #drag: Drag | undefined;
  #frame: Matrix | undefined;
  #guides: { page: number; guides: readonly Guide[] } | undefined;
  #draft: Draft | undefined;
  #polygon: { page: number; points: Point[]; hover: Point | undefined } | undefined;
  #nudge: { item: Item; dx: number; dy: number; timer: ReturnType<typeof setTimeout> } | undefined;
  #textCopy: Promise<void> | undefined;
  #measured = 0;

  constructor(engine: Engine, viewer: Viewer, tools: ToolRegistry, layout: ShellLayout, host: ContentHost) {
    this.#engine = engine;
    this.#viewer = viewer;
    this.#tools = tools;
    this.#layout = layout;
    this.#host = host;
    this.#fonts = new DocumentFonts(registry, (bytes) => engine.addBlob(bytes));
    this.#state = {
      tool: "select",
      shape: "rect",
      selected: undefined,
      typing: undefined,
      look: TEXT_DEFAULT,
      shapeLook: SHAPE_DEFAULT,
      keepRatio: true,
      symbols: false,
      unsupported: undefined,
      known: 0,
    };
    this.#layer = new ContentLayer(viewer, this.#layerHost());
  }

  getState = (): ContentState => this.#state;

  subscribe = (listener: () => void): (() => void) => {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  };

  /** The fonts the picker offers. */
  fonts(): readonly FontChoice[] {
    return registry.list();
  }

  /** The font registry, for adding a font file and naming what a font costs. */
  get registry(): FontRegistry {
    return registry;
  }

  /** What the session knows of a picture it placed. */
  factsOf(id: string): PictureFacts | undefined {
    return this.#facts.get(id);
  }

  /** The registry font object `id` was set in this session, if it was. */
  fontOf(id: string): string | undefined {
    return this.#objectFonts.get(id);
  }

  /** The objects added to `page`, once read. */
  itemsOn(page: number): readonly Item[] {
    return this.#items.get(page) ?? [];
  }

  /** Page `page`'s size in points as the document has it. */
  sizeOf(page: number): { readonly width: number; readonly height: number } | undefined {
    const info = this.#engine.getState().pages[page];
    return info === undefined ? undefined : { width: info.width, height: info.height };
  }

  // Entering and leaving.

  /** Edit mode was entered: the layer, the tools and the keys go on, and the pages in view are read. */
  enter(): void {
    if (this.#on) return;
    this.#on = true;
    this.#layer.attach();
    const select = this.#tool("contentSelect", "");
    const place = this.#tool("contentPlace", "crosshair");
    this.#stops.push(this.#tools.register(select), this.#tools.register(place));
    this.#tools.use("contentSelect");
    const key = (event: KeyboardEvent) => this.#key(event);
    const drop = (event: DragEvent) => this.#drop(event);
    const over = (event: DragEvent) => {
      if ([...(event.dataTransfer?.types ?? [])].includes("Files")) event.preventDefault();
    };
    const paste = (event: ClipboardEvent) => this.#paste(event);
    const element = this.#viewer.element;
    document.addEventListener("keydown", key);
    element.addEventListener("drop", drop);
    element.addEventListener("dragover", over);
    document.addEventListener("paste", paste);
    this.#stops.push(
      () => document.removeEventListener("keydown", key),
      () => element.removeEventListener("drop", drop),
      () => element.removeEventListener("dragover", over),
      () => document.removeEventListener("paste", paste),
      this.#viewer.subscribe(() => this.#prefetch()),
      this.#viewer.edits.subscribe(() => this.#refreshSoon()),
    );
    // Text goes through the text-laying copy of the document, as Comment mode's text boxes do.
    this.#textCopy ??= this.#engine
      .view()
      .then(({ client }) => client.editInTextCopy({ text: true }))
      .catch((error: unknown) => {
        this.#textCopy = undefined;
        console.error("Edit mode couldn't open the text copy", error);
      });
    this.#prefetch();
    this.#render();
  }

  /** Edit mode was left: typing ends, the selection goes and everything it put over the page is taken off. */
  leave(): void {
    if (!this.#on) return;
    void this.#endTyping();
    this.#flushNudge();
    this.#on = false;
    this.#tools.use("select");
    for (const stop of this.#stops.splice(0)) stop();
    this.#layer.detach();
    this.#drag = undefined;
    this.#frame = undefined;
    this.#draft = undefined;
    this.#polygon = undefined;
    this.#guides = undefined;
    this.#layout.closeInspector();
    this.#set({ tool: "select", selected: undefined, symbols: false });
  }

  // Tools.

  /** Turns `tool` on (a tool already on goes off, back to Select). */
  useTool(tool: ContentTool, shape?: ContentShape): void {
    if (shape !== undefined) this.#set({ shape });
    const next = tool === this.#state.tool && shape === undefined && tool !== "select" ? "select" : tool;
    void this.#endTyping();
    this.#polygon = undefined;
    this.#draft = undefined;
    this.#set({ tool: next });
    this.#tools.use(next === "select" || next === "image" ? "contentSelect" : "contentPlace");
    if (next === "image") {
      void this.pickImage().finally(() => this.useTool("select"));
    }
    this.#render();
  }

  /** Whether a tool has the text copy it needs. */
  ready(): Promise<void> {
    return this.#textCopy ?? Promise.resolve();
  }

  #tool(name: "contentSelect" | "contentPlace", cursor: string): ToolModule {
    return {
      name,
      cursor,
      selectsText: false,
      pointerDown: (event) => this.#down(event),
      pointerMove: (event) => this.#move(event),
      pointerUp: (event) => this.#up(event),
    };
  }

  // Pictures.

  /** Asks for a picture and places it on the page in view. */
  async pickImage(replace?: Item): Promise<void> {
    const [picked] = await this.#host.platform.openFiles({ multiple: false, accept: "images" });
    if (picked === undefined) return;
    const file =
      picked.handle instanceof File
        ? picked.handle
        : new File([new Uint8Array(await picked.read(0, picked.size))], picked.name);
    await this.placeFiles([file], undefined, replace);
  }

  /** Places the first picture of `files`, centred on `at` or on the page in view; `replace` swaps it into an image instead. */
  async placeFiles(files: readonly File[], at?: { page: number; point: Point }, replace?: Item): Promise<void> {
    const [file] = files;
    if (file === undefined) return;
    const image = await readImage(file);
    if (image === undefined) {
      this.#set({ unsupported: file.name });
      return;
    }
    const blob = await this.#engine.addBlob(image.bytes);
    if (replace !== undefined) {
      this.#remember(replace.id, image, false);
      await this.#apply(
        [updateOp(this.#pageId(replace.page), replace.id, { blob })],
        laterMessage("content.step.replace"),
        [replace.page],
      );
      return;
    }
    const page = at?.page ?? this.#viewer.getState().currentPage;
    const size = this.sizeOf(page);
    const pageId = this.#pageId(page);
    if (size === undefined || pageId === undefined) return;
    const fit = fittedSize(image, size);
    const centre = at?.point ?? { x: size.width / 2, y: size.height / 2 };
    const x = Math.min(Math.max(centre.x - fit.width / 2, 0), Math.max(0, size.width - fit.width));
    const y = Math.min(Math.max(centre.y - fit.height / 2, 0), Math.max(0, size.height - fit.height));
    const id = newId();
    this.#remember(id, image, fit.smaller);
    const frame: Matrix = [fit.width, 0, 0, fit.height, x, y];
    const kind = laterMessage("content.object.image").toLocaleLowerCase();
    if (
      await this.#apply(
        [addImageOp(pageId, id, blob, frame, stemOf(image.name))],
        laterMessage("content.step.add", { kind }),
        [page],
      )
    ) {
      this.select(page, id);
    }
  }

  #remember(id: string, image: ReadImage, smaller: boolean): void {
    this.#facts.set(id, { name: image.name, width: image.width, height: image.height, type: image.type, smaller });
    this.#set({ known: this.#state.known + 1 });
  }

  /** Closes the refused-file dialog. */
  dismissUnsupported(): void {
    this.#set({ unsupported: undefined });
  }

  // The selection.

  /** Selects object `id` on `page`; with no `id`, clears the selection. */
  select(page: number, id?: string): void {
    this.#flushNudge();
    const item = id === undefined ? undefined : this.itemsOn(page).find((i) => i.id === id);
    this.#frame = undefined;
    this.#guides = undefined;
    this.#set({ selected: item });
    if (item === undefined) {
      if (this.#layout.getState().inspector?.id === CONTENT_INSPECTOR) this.#layout.closeInspector();
    } else {
      this.#layout.openInspector({ id: CONTENT_INSPECTOR, selection: true });
      if (this.#state.tool !== "select") this.useTool("select");
    }
    this.#render();
  }

  /** Clears the selection. */
  clearSelection(): void {
    this.select(0);
  }

  /** The selection's frame, as a drag has it now. */
  frameOfSelection(): Matrix | undefined {
    return this.#frame ?? this.#state.selected?.frame;
  }

  // Changing objects.

  /** Applies `ops` as one step named `label`, then reads `pages` again; whether it was made. */
  async apply(ops: readonly Op[], label: string, pages: readonly number[]): Promise<boolean> {
    return this.#apply(ops, label, pages);
  }

  async #apply(ops: readonly Op[], label: string, pages: readonly number[]): Promise<boolean> {
    try {
      await this.#viewer.edits.apply(ops, label);
    } catch (error) {
      // The session's failure toast says why; a refused font is named here.
      if (error instanceof Error && /content-/.test(error.message)) {
        this.#host.toasts.add({ message: laterMessage("content.failed", { reason: error.message }) });
      }
      return false;
    }
    await this.#refresh(pages);
    return true;
  }

  #pageId(page: number): string {
    return this.#engine.getState().pages[page]?.id ?? "";
  }

  /** The id of `page`, for an op. */
  pageIdOf(page: number): string {
    return this.#pageId(page);
  }

  /** The name of `item`'s kind, lower-cased, for a step's label. */
  kindOf(item: Item): string {
    return laterMessage(`content.${nameOf(item)}` as "content.object.text").toLocaleLowerCase();
  }

  /** Changes object `item` to the frame `frame` as one step named for `step`. */
  async setFrame(
    item: Item,
    frame: Matrix,
    step: "move" | "resize" | "turn" = "move",
    extra: Pick<ContentChanges, "text"> = {},
  ): Promise<boolean> {
    const label = laterMessage(`content.step.${step}`, { kind: this.kindOf(item) });
    const op = updateOp(this.#pageId(item.page), item.id, { matrix: matrixFor(item, frame), ...extra });
    return this.#apply([op], label, [item.page]);
  }

  /** Moves the selection by (`dx`, `dy`) points: the keys and Position fields' way. */
  nudgeSelection(dx: number, dy: number): void {
    const item = this.#state.selected;
    if (item === undefined) return;
    if (this.#nudge !== undefined) clearTimeout(this.#nudge.timer);
    const total = { dx: (this.#nudge?.dx ?? 0) + dx, dy: (this.#nudge?.dy ?? 0) + dy };
    this.#nudge = { item, ...total, timer: setTimeout(() => this.#flushNudge(), NUDGE_MS) };
    this.#frame = moved(item.frame, total.dx, total.dy);
    this.#render();
  }

  #flushNudge(): void {
    const nudge = this.#nudge;
    if (nudge === undefined) return;
    clearTimeout(nudge.timer);
    this.#nudge = undefined;
    const frame = moved(nudge.item.frame, nudge.dx, nudge.dy);
    void this.setFrame(nudge.item, frame).then(() => {
      this.#frame = undefined;
      this.#render();
    });
  }

  /** Deletes the selection. */
  deleteSelection(): void {
    const item = this.#state.selected;
    if (item === undefined) return;
    this.#flushNudge();
    const label = laterMessage("content.step.delete", { kind: this.kindOf(item) });
    void this.#apply([deleteOp(this.#pageId(item.page), item.id)], label, [item.page]);
  }

  /** Puts a copy of the selection 12 points down and right, and selects it. */
  duplicateSelection(): void {
    const item = this.#state.selected;
    const spec = item?.added.object;
    if (item === undefined || spec === undefined) return;
    const id = newId();
    const pageId = this.#pageId(item.page);
    const [a, b, c, d, e, f] = item.added.matrix;
    const offset: Matrix = [a, b, c, d, e + 12, f + 12];
    const op: Op =
      "text" in spec
        ? addTextOp(
            pageId,
            id,
            [
              item.frame[4] + 12,
              item.frame[5] + 12,
              item.frame[4] + 12 + spec.text.width,
              item.frame[5] + 12 + spec.text.height,
            ],
            spec.text.runs.map((r) => r.text).join(""),
            {
              fonts: spec.text.fonts ?? [],
              size: spec.text.size ?? 12,
              color: INK,
              align: (spec.text.align ?? "left") as TextLook["align"],
              bold: spec.text.runs.every((r) => r.bold === true),
              italic: spec.text.runs.every((r) => r.italic === true),
            },
          )
        : ({
            op: "addContentObject",
            args: {
              id,
              page: pageId,
              object: spec,
              matrix: offset,
              opacity: item.added.opacity,
              layer: item.added.layer,
              ...(item.added.alt === undefined ? {} : { alt: item.added.alt }),
              modified: modifiedNow(),
            },
          } as unknown as Op);
    const label = laterMessage("content.step.duplicate", { kind: this.kindOf(item) });
    void this.#apply([op], label, [item.page]).then((made) => {
      if (made) this.select(item.page, id);
    });
  }

  /** Moves the selection among the objects on its layer. */
  arrangeSelection(how: "front" | "forward" | "backward" | "back"): void {
    const item = this.#state.selected;
    if (item === undefined) return;
    const layer = this.itemsOn(item.page).filter((i) => i.added.layer === item.added.layer);
    const index = layer.findIndex((i) => i.id === item.id);
    const order = {
      front: -1,
      back: 0,
      forward: Math.min(index + 1, layer.length - 1),
      backward: Math.max(index - 1, 0),
    }[how];
    const label = laterMessage("content.step.arrange", { kind: this.kindOf(item) });
    void this.#apply([updateOp(this.#pageId(item.page), item.id, { order })], label, [item.page]);
  }

  // Text.

  /** Turns the look of new text (or of the box being typed) into `look`. */
  setLook(change: Partial<Look>): void {
    const typing = this.#state.typing;
    if (typing !== undefined) this.#set({ typing: { ...typing, look: { ...typing.look, ...change } } });
    this.#set({ look: { ...this.#state.look, ...change } });
    this.#render();
  }

  /** Opens a box on `page` in `rect` for typing a new text. */
  openText(page: number, rect: Box): void {
    this.select(page);
    this.#layout.openInspector({ id: CONTENT_INSPECTOR, selection: true });
    this.#set({ typing: { page, id: undefined, rect, text: "", look: this.#state.look } });
    this.#render();
    requestAnimationFrame(() => this.#layer.focusEditor());
  }

  /** Opens the selected text box for typing. */
  editSelectedText(): void {
    const item = this.#state.selected;
    const spec = item?.added.object;
    if (item === undefined || spec === undefined || !("text" in spec)) return;
    const look = lookOfBlock(spec.text, this.#objectFonts.get(item.id));
    const rect = boxAround(corners(item.frame));
    const text = spec.text.runs.map((r) => r.text).join("");
    this.#set({ typing: { page: item.page, id: item.id, rect, text, look } });
    this.#render();
    requestAnimationFrame(() => this.#layer.focusEditor());
  }

  /** What typing the box now has, kept as it is typed. */
  #typed(text: string): void {
    const typing = this.#state.typing;
    if (typing !== undefined && typing.text !== text) this.#state = { ...this.#state, typing: { ...typing, text } };
  }

  /** Whether a box is being typed into. */
  get typing(): boolean {
    return this.#state.typing !== undefined;
  }

  /** Ends typing: the text becomes an object, or changes the one being edited; an empty new box goes without an op. */
  async #endTyping(): Promise<void> {
    const typing = this.#state.typing;
    if (typing === undefined) return;
    const text = this.#layer.typed() || typing.text;
    this.#set({ typing: undefined });
    this.#render();
    if (text.trim() === "" || this.sizeOf(typing.page) === undefined) {
      // Nothing typed: no op, and no inspector for a box that never was.
      if (typing.id === undefined) this.select(0);
      return;
    }
    const plan = await this.#fonts
      .forText(typing.look.fontId, text)
      .catch(() => ({ fonts: [typing.look.fontId], missing: [] as string[] }));
    this.#set({ known: this.#state.known + 1 });
    if (plan.missing.length > 0) {
      this.#host.toasts.add({ message: laterMessage("content.font.missing", { characters: plan.missing.join(" ") }) });
      this.#set({ typing });
      this.#render();
      return;
    }
    const look: TextLook = { ...typing.look, fonts: plan.fonts };
    const [x0, , x1] = typing.rect;
    const width = Math.max(x1 - x0, 24);
    const measured = heightFor(text, width, look.size, cssFamily(typing.look.fontId), look.bold, look.italic);
    const height = Math.max(this.#layer.typedHeight(typing.page), measured);
    const id = typing.id ?? newId();
    this.#objectFonts.set(id, typing.look.fontId);
    if (typing.id === undefined) await this.#addText(typing, id, text, look, height);
    else await this.#editText(typing, text, look, height);
  }

  async #addText(typing: Typing, id: string, text: string, look: TextLook, height: number): Promise<void> {
    const [x0, y0, x1] = typing.rect;
    const rect: Box = [x0, y0, x0 + Math.max(x1 - x0, 24), y0 + height];
    const kind = laterMessage("content.object.text").toLocaleLowerCase();
    const op = addTextOp(this.#pageId(typing.page), id, rect, text, look);
    if (await this.#apply([op], laterMessage("content.step.add", { kind }), [typing.page]))
      this.select(typing.page, id);
  }

  async #editText(typing: Typing, text: string, look: TextLook, height: number): Promise<void> {
    const item = this.itemsOn(typing.page).find((i) => i.id === typing.id);
    const spec = item?.added.object;
    if (item === undefined || spec === undefined || !("text" in spec)) return;
    const block: TextBlock = {
      ...spec.text,
      runs: [{ text, ...(look.bold ? { bold: true } : {}), ...(look.italic ? { italic: true } : {}) }],
      fonts: [...look.fonts],
      size: look.size,
      align: look.align,
      color: colour(look.color),
      height: Math.max(height, spec.text.height),
    };
    await this.#apply(
      [updateOp(this.#pageId(typing.page), item.id, { text: block })],
      laterMessage("content.step.edit"),
      [typing.page],
    );
  }

  /** Changes the look of the selected text as one step: its font (with the fallbacks its characters need), size, style, alignment or colour. */
  async changeText(change: Partial<Look>): Promise<void> {
    const item = this.#state.selected;
    const spec = item?.added.object;
    if (item === undefined || spec === undefined || !("text" in spec)) return;
    const block = spec.text;
    const text = block.runs.map((r) => r.text).join("");
    const look: Look = { ...lookOfBlock(block, this.#objectFonts.get(item.id)), ...change };
    const plan = await this.#fonts.forText(look.fontId, text);
    this.#set({ known: this.#state.known + 1 });
    if (plan.missing.length > 0) {
      this.#host.toasts.add({ message: laterMessage("content.font.missing", { characters: plan.missing.join(" ") }) });
      return;
    }
    const grown = heightFor(text, block.width, look.size, cssFamily(look.fontId), look.bold, look.italic);
    const next: TextBlock = {
      ...block,
      runs: block.runs.map((r) => ({
        text: r.text,
        ...(look.bold ? { bold: true } : {}),
        ...(look.italic ? { italic: true } : {}),
      })),
      fonts: [...plan.fonts],
      size: look.size,
      align: look.align,
      color: colour(look.color),
      height: Math.max(grown, change.size === undefined ? block.height : 0),
    };
    this.#objectFonts.set(item.id, look.fontId);
    this.#set({ look: { ...this.#state.look, ...change } });
    const label = laterMessage("content.step.change", { kind: this.kindOf(item) });
    // The box's top edge stays where it was, so the matrix's origin (its bottom-left) follows a new height.
    const [a, b, c, d, e, f] = item.added.matrix;
    const matrix: Matrix = [a, b, c, d, e, f - block.height + next.height];
    await this.#apply([updateOp(this.#pageId(item.page), item.id, { text: next, matrix })], label, [item.page]);
  }

  /** Changes the selected object's own fields (opacity, fill, outline, alternative text, or its picture) as one step. */
  change(changes: Omit<ContentChanges, "modified">): void {
    const item = this.#state.selected;
    if (item === undefined) return;
    const label = laterMessage("content.step.change", { kind: this.kindOf(item) });
    void this.#apply([updateOp(this.#pageId(item.page), item.id, changes)], label, [item.page]);
  }

  /** Sets what the next shape is drawn with. */
  setShapeLook(look: ShapeLook): void {
    this.#set({ shapeLook: look });
  }

  /** Whether Width and Height move together. */
  setKeepRatio(keepRatio: boolean): void {
    this.#set({ keepRatio });
  }

  /** Ends typing. */
  done(): void {
    void this.#endTyping();
  }

  /** Asks for a font file and adds it to the session's fonts; a file that can't be used says why. */
  async addFont(): Promise<void> {
    const [picked] = await this.#host.platform.openFiles({ multiple: false, accept: "any" });
    if (picked === undefined) return;
    const file =
      picked.handle instanceof File
        ? picked.handle
        : new File([new Uint8Array(await picked.read(0, picked.size))], picked.name);
    if ((await addFontFile(file, this.#host.toasts)) !== undefined) this.#set({ known: this.#state.known + 1 });
  }

  /** Opens or closes the symbol picker. */
  pickSymbol(open: boolean): void {
    this.#set({ symbols: open });
  }

  /** Puts `character` into the box being typed, or on the end of the selected text. */
  insertSymbol(character: string): void {
    if (this.#state.typing !== undefined) {
      this.#layer.insertAtCaret(character);
      return;
    }
    const item = this.#state.selected;
    const spec = item?.added.object;
    if (item === undefined || spec === undefined || !("text" in spec)) return;
    this.editSelectedText();
    requestAnimationFrame(() => this.#layer.insertAtCaret(character));
  }

  // Reading the pages.

  #prefetch(): void {
    const current = this.#viewer.getState().currentPage;
    for (const page of [current - 1, current, current + 1]) {
      if (page >= 0 && !this.#items.has(page) && this.sizeOf(page) !== undefined) void this.#refresh([page]);
    }
  }

  #refreshSoon(): void {
    // Undo and redo change the pages' objects without going through `apply`.
    const steps = this.#viewer.edits.getState().history;
    if (steps.position === this.#measured) return;
    this.#measured = steps.position;
    void this.#refresh([...this.#items.keys()]);
  }

  async #refresh(pages: readonly number[]): Promise<void> {
    await Promise.all(
      pages.map(async (page) => {
        const added = await this.#engine.addedObjects(page).catch((error: unknown) => {
          console.error("addedObjects failed", error);
          return undefined;
        });
        if (added === undefined) return;
        this.#items.set(
          page,
          added.flatMap((a) => itemOf(page, a) ?? []),
        );
      }),
    );
    const selected = this.#state.selected;
    const again = selected === undefined ? undefined : this.itemsOn(selected.page).find((i) => i.id === selected.id);
    this.#frame = undefined;
    this.#guides = undefined;
    this.#set({ selected: again });
    if (selected !== undefined && again === undefined && this.#layout.getState().inspector?.id === CONTENT_INSPECTOR) {
      this.#layout.closeInspector();
    }
    this.#render();
  }

  // Pointer input.

  /** The page-space point under a pointer, on `page`, whatever the view's turn. */
  #pagePoint(page: number, event: { readonly clientX: number; readonly clientY: number }): Point | undefined {
    const rect = this.#layer.rectOf(page);
    const size = this.sizeOf(page);
    if (rect === undefined || size === undefined || rect.width === 0 || rect.height === 0) return undefined;
    const [x, y] = unturn(
      (event.clientX - rect.left) / rect.width,
      (event.clientY - rect.top) / rect.height,
      this.#viewer.getState().rotation,
    );
    return { x: x * size.width, y: y * size.height };
  }

  #hit(page: number, at: Point): Item | undefined {
    const items = this.itemsOn(page);
    // Topmost first, a few points of slop around thin things (a line).
    for (let i = items.length - 1; i >= 0; i--) {
      const item = items[i];
      if (item === undefined) continue;
      const thin = Math.min(sizeOf(item.frame).width, sizeOf(item.frame).height) < 8;
      if (contains(item.frame, at, thin ? 5 : 0)) return item;
    }
    return undefined;
  }

  #down(event: PointerEvent): boolean {
    if (event.button !== 0 || event.pointerType === "touch") return false;
    const target = event.target as Element | null;
    if (target?.closest("[data-content-handle], .lamina-content-editor") != null) return false;
    const at = pointOn(this.#viewer, this.#engine, event);
    if (this.#state.typing !== undefined) void this.#endTyping();
    if (at === undefined) {
      this.select(0);
      return false;
    }
    const point = { x: at.x, y: at.y };
    // The tool took the press, which keeps focus where it was: Enter and Escape are for the page now, not a toolbar button.
    this.#viewer.element.focus({ preventScroll: true });
    switch (this.#state.tool) {
      case "shape":
        return this.#downShape(at.page, point);
      case "text":
        this.#drag = { kind: "text", page: at.page, from: point };
        return true;
      default: {
        // A page not read yet: read it, then select what the press was on.
        if (!this.#items.has(at.page)) {
          void this.#refresh([at.page]).then(() => {
            const found = this.#hit(at.page, point);
            if (found !== undefined) this.select(at.page, found.id);
          });
          return false;
        }
        const item = this.#hit(at.page, point);
        if (item === undefined) {
          this.select(0);
          return false;
        }
        this.select(at.page, item.id);
        this.#drag = { kind: "move", item, from: point };
        return true;
      }
    }
  }

  #downShape(page: number, point: Point): boolean {
    if (this.#state.shape === "polygon") {
      const polygon = this.#polygon?.page === page ? this.#polygon : undefined;
      const first = polygon?.points[0];
      if (polygon !== undefined && first !== undefined && polygon.points.length >= 3) {
        const scale = this.#layer.rectOf(page)?.width ?? 1;
        const size = this.sizeOf(page);
        const near = Math.hypot(first.x - point.x, first.y - point.y) < 8 * ((size?.width ?? 1) / scale);
        if (near) {
          void this.#closePolygon();
          return true;
        }
      }
      this.#polygon = { page, points: [...(polygon?.points ?? []), point], hover: point };
      this.#showPolygon();
      return true;
    }
    this.#drag = { kind: "shape", page, from: point, shape: this.#state.shape };
    return true;
  }

  #showPolygon(): void {
    const polygon = this.#polygon;
    this.#draft =
      polygon === undefined
        ? undefined
        : {
            page: polygon.page,
            kind: "polygon",
            points: polygon.hover === undefined ? polygon.points : [...polygon.points, polygon.hover],
            closed: false,
          };
    this.#render();
  }

  #move(event: PointerEvent): void {
    const drag = this.#drag;
    if (drag === undefined) {
      const polygon = this.#polygon;
      const point = polygon === undefined ? undefined : this.#pagePoint(polygon.page, event);
      if (polygon !== undefined && point !== undefined) {
        polygon.hover = point;
        this.#showPolygon();
      }
      return;
    }
    this.#drag = drag;
    const page = "item" in drag ? drag.item.page : drag.page;
    const now = this.#pagePoint(page, event);
    if (now === undefined) return;
    switch (drag.kind) {
      case "move":
        this.#dragMove(drag, now, event.altKey);
        break;
      case "handle":
        this.#dragHandle(drag, now, event.shiftKey);
        break;
      case "text":
        this.#draft = { page, kind: "box", box: dragBox(drag.from, now) };
        break;
      case "shape":
        this.#draft = this.#shapeDraft(drag, now, event.shiftKey);
        break;
    }
    this.#render();
  }

  #shapeDraft(drag: Extract<Drag, { kind: "shape" }>, now: Point, shift: boolean): Draft {
    if (drag.shape === "line")
      return { page: drag.page, kind: "line", from: drag.from, to: lineEnd(drag.from, now, shift) };
    return { page: drag.page, kind: drag.shape === "ellipse" ? "ellipse" : "box", box: dragBox(drag.from, now, shift) };
  }

  #dragMove(drag: Extract<Drag, { kind: "move" }>, now: Point, noSnap: boolean): void {
    const size = this.sizeOf(drag.item.page);
    let dx = now.x - drag.from.x;
    let dy = now.y - drag.from.y;
    this.#guides = undefined;
    if (size !== undefined && !noSnap) {
      const others = this.itemsOn(drag.item.page)
        .filter((i) => i.id !== drag.item.id)
        .map((i) => boxAround(corners(i.frame)));
      const snapped = snapMove(boxAround(corners(drag.item.frame)), dx, dy, size, others);
      dx = snapped.dx;
      dy = snapped.dy;
      if (snapped.guides.length > 0) this.#guides = { page: drag.item.page, guides: snapped.guides };
    }
    this.#frame = moved(drag.item.frame, dx, dy);
  }

  #dragHandle(drag: Extract<Drag, { kind: "handle" }>, now: Point, shift: boolean): void {
    const { item } = drag;
    if (drag.handle === "turn") {
      this.#frame = turnedTo(item.frame, now, shift);
      return;
    }
    const dx = now.x - drag.from.x;
    const dy = now.y - drag.from.y;
    if (item.kind === "text") {
      const { width } = sizeOf(item.frame);
      const next = Math.max(24, width + (drag.handle === "e" ? dx : -dx));
      const left = drag.handle === "w" ? item.frame[4] + (width - next) : item.frame[4];
      this.#frame = [next, 0, 0, item.frame[3], left, item.frame[5]];
      return;
    }
    this.#frame = resized(item.frame, drag.handle, dx, dy, shift);
  }

  #up(event: PointerEvent): void {
    const drag = this.#drag;
    this.#drag = undefined;
    this.#guides = undefined;
    if (drag === undefined) return;
    const page = "item" in drag ? drag.item.page : drag.page;
    const now = this.#pagePoint(page, event) ?? drag.from;
    this.#finish(drag, now, event.shiftKey);
  }

  #finish(drag: Drag, now: Point, shift: boolean): void {
    const frame = this.#frame;
    this.#frame = undefined;
    this.#draft = undefined;
    const far = Math.hypot(now.x - drag.from.x, now.y - drag.from.y) >= CLICK;
    switch (drag.kind) {
      case "move":
        if (frame !== undefined && far) void this.setFrame(drag.item, frame).then(() => this.#render());
        break;
      case "handle":
        if (frame !== undefined) void this.#finishHandle(drag.item, frame, drag.handle === "turn" ? "turn" : "resize");
        break;
      case "text":
        this.#finishText(drag, now, far);
        break;
      case "shape":
        void this.#finishShape(drag, now, far, shift);
        break;
    }
    this.#render();
  }

  async #finishHandle(item: Item, frame: Matrix, step: "resize" | "turn"): Promise<void> {
    const spec = item.added.object;
    if (item.kind === "text" && spec !== undefined && "text" in spec) {
      // A text box's sides set its width: the block takes the new width, and its left edge moves with the west handle.
      const block = { ...spec.text, width: sizeOf(frame).width };
      await this.setFrame(item, frame, step, { text: block });
    } else {
      await this.setFrame(item, frame, step);
    }
    this.#render();
  }

  #finishText(drag: Extract<Drag, { kind: "text" }>, now: Point, far: boolean): void {
    const size = this.sizeOf(drag.page);
    if (size === undefined) return;
    const left = far ? Math.min(drag.from.x, now.x) : drag.from.x;
    const top = far ? Math.min(drag.from.y, now.y) : drag.from.y;
    // A click makes the box as wide as the page's text, as far as the page goes.
    const width = far ? Math.abs(now.x - drag.from.x) : Math.max(NARROWEST_TEXT, size.width - 36 - left);
    const w = Math.max(width, NARROWEST_TEXT / 2);
    const rect: Box = [
      Math.min(left, size.width - w),
      top,
      Math.min(left, size.width - w) + w,
      top + this.#state.look.size * 1.4,
    ];
    this.openText(drag.page, rect);
  }

  async #finishShape(drag: Extract<Drag, { kind: "shape" }>, now: Point, far: boolean, shift: boolean): Promise<void> {
    const size = this.sizeOf(drag.page);
    const pageId = this.#pageId(drag.page);
    if (size === undefined) return;
    let from = drag.from;
    let to = far ? now : { x: from.x + DEFAULT_SHAPE, y: from.y + (drag.shape === "line" ? 0 : DEFAULT_SHAPE) };
    if (drag.shape === "line") to = lineEnd(from, to, shift);
    else if (far) {
      const [x0, y0, x1, y1] = dragBox(drag.from, now, shift);
      from = { x: x0, y: y0 };
      to = { x: x1, y: y1 };
    }
    const id = newId();
    const kind = laterMessage(`content.object.${drag.shape}`).toLocaleLowerCase();
    const op = addShapeOp(pageId, id, pathOf(drag.shape, from, to), this.#state.shapeLook);
    if (await this.#apply([op], laterMessage("content.step.add", { kind }), [drag.page])) this.select(drag.page, id);
  }

  async #closePolygon(): Promise<void> {
    const polygon = this.#polygon;
    this.#polygon = undefined;
    this.#draft = undefined;
    this.#render();
    if (polygon === undefined || polygon.points.length < 3) return;
    const id = newId();
    const first = polygon.points[0] as Point;
    const op = addShapeOp(
      this.#pageId(polygon.page),
      id,
      pathOf("polygon", first, first, polygon.points),
      this.#state.shapeLook,
    );
    const kind = laterMessage("content.object.polygon").toLocaleLowerCase();
    if (await this.#apply([op], laterMessage("content.step.add", { kind }), [polygon.page]))
      this.select(polygon.page, id);
  }

  // Keys, drops and pastes.

  #key(event: KeyboardEvent): void {
    const target = event.target as HTMLElement | null;
    if (target?.closest("input, textarea, select, [contenteditable=true], [role=dialog], [role=menu]") != null) return;
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    if (event.key === "Escape") this.#escape();
    else if (event.key === "Enter") this.#enter(event);
    else if (this.#state.selected !== undefined) this.#selectionKey(event);
  }

  /** Escape's steps: a polygon being drawn, then the tool on, then the selection. */
  #escape(): void {
    if (this.#polygon !== undefined) {
      this.#polygon = undefined;
      this.#draft = undefined;
      this.#render();
    } else if (this.#state.tool !== "select") this.useTool("select");
    else if (this.#state.selected !== undefined) this.select(0);
  }

  /** Enter closes a polygon, or starts typing into the selected text. */
  #enter(event: KeyboardEvent): void {
    if (this.#polygon !== undefined) void this.#closePolygon();
    else if (this.#state.selected?.kind === "text") this.editSelectedText();
    else return;
    event.preventDefault();
  }

  /** Delete removes the selection; the arrow keys nudge it. */
  #selectionKey(event: KeyboardEvent): void {
    if (event.key === "Delete" || event.key === "Backspace") {
      event.preventDefault();
      this.deleteSelection();
      return;
    }
    const step = nudgeOf(event.key, event.shiftKey);
    if (step === undefined) return;
    event.preventDefault();
    this.nudgeSelection(step.x, step.y);
  }

  #drop(event: DragEvent): void {
    const files = [...(event.dataTransfer?.files ?? [])];
    if (files.length === 0) return;
    event.preventDefault();
    const at = pointOn(this.#viewer, this.#engine, event);
    void this.placeFiles(files, at === undefined ? undefined : { page: at.page, point: { x: at.x, y: at.y } });
  }

  #paste(event: ClipboardEvent): void {
    const target = event.target as HTMLElement | null;
    if (target?.closest("input, textarea, [contenteditable=true]") != null) return;
    const files = [...(event.clipboardData?.files ?? [])].filter((f) => f.type.startsWith("image/"));
    if (files.length === 0) return;
    event.preventDefault();
    void this.placeFiles(files);
  }

  // Layer and state.

  #layerHost(): LayerHost {
    return {
      pageSize: (page) => this.sizeOf(page),
      onHandle: (handle, event) => this.#startHandle(handle, event),
      onType: (text) => this.#typed(text),
      onTypeEnd: (why, next) => {
        // The inspector and the symbol picker are part of typing: focus moving there keeps the box open.
        if (why === "away" && next?.closest(".paper-content-inspector, [role=dialog]") != null) return;
        void this.#endTyping();
      },
    };
  }

  #startHandle(handle: HandleName | "turn", event: PointerEvent): void {
    const item = this.#state.selected;
    if (item === undefined) return;
    const from = this.#pagePoint(item.page, event);
    if (from === undefined) return;
    this.#drag = { kind: "handle", item, handle, from };
    const move = (e: PointerEvent) => this.#move(e);
    const up = (e: PointerEvent) => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", up);
      this.#up(e);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", up);
  }

  #render(): void {
    const { selected, typing } = this.#state;
    const handles: readonly HandleName[] = selected?.kind === "text" ? TEXT_HANDLES : HANDLES;
    const locked = this.#viewer.edits.getState().blocked !== undefined;
    this.#layer.show({
      ...(selected === undefined || typing?.id === selected.id
        ? {}
        : {
            selection: {
              page: selected.page,
              frame: this.#frame ?? selected.frame,
              handles,
              turn: selected.kind !== "text",
              locked,
            },
          }),
      ...(this.#guides === undefined ? {} : { guides: this.#guides }),
      ...(this.#draft === undefined ? {} : { draft: this.#draft }),
      ...(typing === undefined
        ? {}
        : {
            editor: {
              page: typing.page,
              rect: typing.rect,
              text: typing.text,
              label: laterMessage(typing.id === undefined ? "content.typing.label" : "content.typing.edit", {
                page: typing.page + 1,
              }),
              style: {
                family: cssFamily(typing.look.fontId),
                size: typing.look.size,
                color: css(typing.look.color),
                bold: typing.look.bold,
                italic: typing.look.italic,
                align: typing.look.align,
              },
            },
          }),
    });
  }

  #set(change: Partial<ContentState>): void {
    this.#state = { ...this.#state, ...change };
    for (const listener of [...this.#listeners]) listener();
  }
}

/** A font file the user chose, added to the session's registry; the refusal's words when it can't be used. */
export async function addFontFile(file: File, toasts: ToastQueue): Promise<FontChoice | undefined> {
  try {
    return registry.add(file.name, new Uint8Array(await file.arrayBuffer()));
  } catch (error) {
    if (error instanceof FontFileError) {
      toasts.add({ message: laterMessage(`content.fontRefused.${error.problem}`, { file: file.name }) });
      return undefined;
    }
    throw error;
  }
}
