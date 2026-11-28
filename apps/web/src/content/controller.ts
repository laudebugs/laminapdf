// Edit mode's state over one open document (Paper A01, A02): the tool on, the objects Lamina PDF
// added to the pages in view, the selection and its handles, the text being typed, and every
// change as one op and one undo step. Pointer input comes through two interaction modes (select
// and place) and the layer's handles; the inspector's changes are `inspector-props.ts`'s. Nothing
// here reads a clock but the ops' dates, so a drag moves only the overlay and the page redraws
// once, when the edit is made.

import type { ContentChanges, Op, PathSpec, TextBlock } from "@laminapdf/sdk";
import type { Platform, ShellLayout, ToastQueue, ToolModule } from "@laminapdf/ui";
import { type ContentShape, type ContentTool, laterMessage, type ToolRegistry } from "@laminapdf/ui/editing";
import type { Viewer } from "@laminapdf/viewer";
import { DocumentFonts } from "../fonts/document-fonts.ts";
import { type FontChoice, FontFileError, FontRegistry } from "../fonts/registry.ts";
import { pointOn, turn, unturn } from "../page-points.ts";
import { pickedFile } from "../picked-file.ts";
import type { SaveFlow } from "../save-flow.ts";
import type { Engine } from "../session.ts";
import { handleFrame, movedFrame } from "./drag.ts";
import {
  type Box,
  boxAround,
  boxOf,
  contains,
  corners,
  dragBox,
  type Guide,
  HANDLES,
  type HandleName,
  lineEnd,
  MARGIN,
  type Matrix,
  moved,
  nudgeOf,
  type Point,
  reshaped,
  sizeOf,
  TEXT_HANDLES,
} from "./geometry.ts";
import { fittedSize, type ReadImage, readImage, stemOf } from "./images.ts";
import { CONTENT_INSPECTOR } from "./inspector-id.ts";
import { ContentLayer, type Draft, type LayerHost } from "./layer.ts";
import { type Item, itemOf, matrixFor, nameOf, resizedShape } from "./objects.ts";
import {
  type AltAnswer,
  addImageOp,
  addShapeOp,
  addTextOp,
  colour,
  deleteOp,
  modifiedNow,
  newId,
  pathOf,
  rgbOf,
  type ShapeKind,
  type ShapeLook,
  type TextLook,
  updateOp,
} from "./ops.ts";
import { type Arrange, hitOriginal, type Picked } from "./original.ts";
import { Originals } from "./originals.ts";
import type { TextPick, TextTyping } from "./text-edit.ts";
import { css, cssFamily, INK, type Look, lookOfBlock, SHAPE_DEFAULT, TEXT_DEFAULT } from "./text-look.ts";
import { heightFor } from "./text-measure.ts";
import { TextTool } from "./text-tool.ts";

/** Edit mode's two interaction modes: selecting what was added, and placing something new. */
declare module "@laminapdf/ui" {
  interface InteractionModes {
    contentSelect: true;
    contentPlace: true;
  }
}

/** The code of a picture refused on a PDF/UA file for want of alternative text. */
const NEEDS_ALT = "content-needs-alt-text";

/** The code of `error`, "" when it has none. */
function codeOf(error: unknown): string {
  const code = (error as { code?: unknown } | null)?.code;
  if (typeof code === "string") return code;
  const found = error instanceof Error ? /content-[a-z-]+/.exec(error.message) : null;
  return found?.[0] ?? "";
}

/** Smaller than this, in points, a drag is a click. */
const CLICK = 3;
/** A click places a shape this big, in points. */
const DEFAULT_SHAPE = 72;
/** A click places a text box at least this wide, in points. */
const NARROWEST_TEXT = 120;
/** How long arrow-key nudges gather before they are one step, in ms. */
const NUDGE_MS = 400;

/** How long edits are quiet before what the file drew is read again, in milliseconds. */
const READ_AFTER_MS = 800;

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
  /** The objects the page already drew that are picked (Select over what the file had). */
  readonly picked: Picked | undefined;
  /** The lines of the page's own text that Edit text has picked. */
  readonly textPick: TextPick | undefined;
  /** The line of the page's own text being typed into. */
  readonly textTyping: TextTyping | undefined;
  /** Where the context menu is open: the element it hangs from. */
  readonly menuAt: HTMLElement | undefined;
  /** Asked for: the focus goes to this field, once the inspector has it. */
  readonly fieldFocus: "alt" | "crop" | undefined;
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
  /** The picture (a name) a PDF/UA file wants alternative text for before it is placed. */
  readonly askingAlt: string | undefined;
  /** What this session knows of pictures it placed, bumped when it changes. */
  readonly known: number;
}

/** What Edit mode needs of the app. */
export interface ContentHost {
  readonly platform: Platform;
  readonly toasts: ToastQueue;
  /** Saving: what the file claims and what the edits broke, and where a person is sent to fix it. */
  readonly flow: SaveFlow;
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
  readonly #originals: Originals;
  readonly #text: TextTool;
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
  /** The changes in flight, in the order they were asked for. */
  #queue: Promise<void> = Promise.resolve();
  #measured = 0;
  /** Pages whose objects are to be read again once the edits stop, and the timer that does it. */
  readonly #later: number[] = [];
  #laterTimer: ReturnType<typeof setTimeout> | undefined;
  #answerAlt: ((alt: AltAnswer | undefined) => void) | undefined;
  /** What had the focus when the context menu opened, which gets it back. */
  #menuOpener: Element | null = null;
  /** Pages whose last read failed: what they held may have changed, so the next press reads them again. */
  readonly #stale = new Set<number>();

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
      picked: undefined,
      textPick: undefined,
      textTyping: undefined,
      menuAt: undefined,
      fieldFocus: undefined,
      typing: undefined,
      look: TEXT_DEFAULT,
      shapeLook: SHAPE_DEFAULT,
      keepRatio: true,
      symbols: false,
      unsupported: undefined,
      askingAlt: undefined,
      known: 0,
    };
    this.#layer = new ContentLayer(viewer, this.#layerHost());
    this.#originals = new Originals({
      contentObjects: (page) => engine.contentObjects(page),
      itemsOn: (page) => this.itemsOn(page),
      pageId: (page) => this.#pageId(page),
      pageSize: (page) => this.sizeOf(page),
      apply: (ops, label, pages) => this.#apply(ops, label, pages),
      enqueue: (work) => this.#enqueue(work),
      applyNow: async (ops, label, pages) => (await this.#applyNow(ops, label, pages)) === undefined,
      position: () => viewer.edits.getState().history.position,
      changed: () => this.#pickChanged(),
      clearAdded: () => {
        this.#set({ selected: undefined });
        this.#text.editor.clear();
      },
    });
    this.#text = new TextTool({
      viewer,
      engine,
      fonts: this.#fonts,
      pageId: (page) => this.#pageId(page),
      apply: (ops, label, pages) => this.#apply(ops, label, pages),
      applyNow: async (ops, label, pages) => (await this.#applyNow(ops, label, pages)) === undefined,
      enqueue: (work) => this.#enqueue(work),
      changed: () => this.#textChanged(),
      clearOthers: () => {
        this.#originals.clear();
        if (this.#state.selected !== undefined) this.#set({ selected: undefined });
      },
      say: (message) => this.#host.toasts.add({ message }),
      selectBlock: (page, box) => void this.#selectBlock(page, box),
    });
  }

  /** What Edit text does: the lines of the page's own text. */
  get text(): TextTool {
    return this.#text;
  }

  /** Saving, which knows the claims the file makes. */
  get flow(): SaveFlow {
    return this.#host.flow;
  }

  /** What Select over the objects the file already had does. */
  get originals(): Originals {
    return this.#originals;
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

  /** The fonts this document holds as blobs, which page marks share so a font is added once. */
  get documentFonts(): DocumentFonts {
    return this.#fonts;
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
    const hover = (event: PointerEvent) => this.#hover(event);
    const menu = (event: MouseEvent) => this.#contextMenu(event);
    const dblclick = (event: MouseEvent) => this.#doubleClick(event);
    element.addEventListener("dblclick", dblclick);
    element.addEventListener("pointermove", hover);
    const leave = () => {
      this.#originals.unhover();
      this.#text.editor.unhover();
    };
    element.addEventListener("pointerleave", leave);
    element.addEventListener("contextmenu", menu);
    document.addEventListener("keydown", key);
    element.addEventListener("drop", drop);
    element.addEventListener("dragover", over);
    document.addEventListener("paste", paste);
    this.#stops.push(
      () => document.removeEventListener("keydown", key),
      this.#afterChanges(),
      () => element.removeEventListener("pointermove", hover),
      () => element.removeEventListener("dblclick", dblclick),
      () => element.removeEventListener("pointerleave", leave),
      () => element.removeEventListener("contextmenu", menu),
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
    void this.#flushNudge();
    // Text typed into on the page is saved before what was read of it is forgotten.
    void this.#text.editor.commit("away").finally(() => this.#text.editor.reset());
    // A question still open is answered with a cancel, or it would return with the mode.
    this.answerAlt(undefined);
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
    this.closeMenu(false);
    this.#originals.clear();
    this.#set({ tool: "select", selected: undefined, symbols: false, textPick: undefined, textTyping: undefined });
  }

  // Tools.

  /** Turns `tool` on (a tool already on goes off, back to Select). */
  useTool(tool: ContentTool, shape?: ContentShape): void {
    if (shape !== undefined) this.#set({ shape });
    const next = tool === this.#state.tool && shape === undefined && tool !== "select" ? "select" : tool;
    void this.#endTyping();
    this.#polygon = undefined;
    this.#draft = undefined;
    if (this.#state.tool === "edit-text" && next !== "edit-text") {
      void this.#text.editor.commit("away").then(() => this.#text.editor.clear());
    }
    if (next === "edit-text") {
      this.clearSelection();
      this.#text.prefetch();
    }
    this.#set({ tool: next });
    this.#tools.use(next === "text" || next === "shape" ? "contentPlace" : "contentSelect");
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
      pointerCancel: () => this.#cancel(),
    };
  }

  // Pictures.

  /** Asks for a picture and places it on the page in view. */
  async pickImage(replace?: Item | "picked"): Promise<void> {
    const [picked] = await this.#host.platform.openFiles({ multiple: false, accept: "images" });
    if (picked === undefined) return;
    const file = await pickedFile(picked);
    await this.placeFiles([file], undefined, replace);
  }

  /** Places the first picture of `files`, centred on `at` or on the page in view; `replace` swaps it into an image instead. */
  async placeFiles(
    files: readonly File[],
    at?: { page: number; point: Point },
    replace?: Item | "picked",
  ): Promise<void> {
    const [file] = files;
    if (file === undefined) return;
    const image = await readImage(file);
    if (image === undefined) {
      this.#set({ unsupported: file.name });
      return;
    }
    const blob = await this.#engine.addBlob(image.bytes);
    if (replace !== undefined) {
      await (replace === "picked" ? this.#replacePicked(blob) : this.#replaceImage(replace, image, blob));
      return;
    }
    await this.#placeNew(image, blob, at);
  }

  /** Places the picture `image`, stored as `blob`, on a page as a new object, once PDF/UA alternative text is settled. */
  async #placeNew(image: ReadImage, blob: string, at: { page: number; point: Point } | undefined): Promise<void> {
    const page = at?.page ?? this.#viewer.getState().currentPage;
    const size = this.sizeOf(page);
    const pageId = this.#pageId(page);
    if (size === undefined || pageId === "") return;
    const fit = fittedSize(image, size);
    const centre = at?.point ?? { x: size.width / 2, y: size.height / 2 };
    const x = Math.min(Math.max(centre.x - fit.width / 2, 0), Math.max(0, size.width - fit.width));
    const y = Math.min(Math.max(centre.y - fit.height / 2, 0), Math.max(0, size.height - fit.height));
    const id = newId();
    const frame: Matrix = [fit.width, 0, 0, fit.height, x, y];
    const label = laterMessage("content.step.add", { kind: laterMessage("content.object.image").toLocaleLowerCase() });
    // A PDF/UA file takes no picture without alternative text: ask first, so no refused edit is shown as a failure.
    let alt: AltAnswer | undefined;
    if (await this.#claimsPdfUa()) {
      alt = await this.#askAlt(image.name);
      if (alt === undefined) return;
    }
    let code = await this.#tryApply([addImageOp(pageId, id, blob, frame, alt)], label, [page]);
    // The claim wasn't read (or the core reads it differently): the refusal is the fallback.
    if (code === NEEDS_ALT && alt === undefined) {
      alt = await this.#askAlt(image.name);
      if (alt === undefined) return;
      code = await this.#tryApply([addImageOp(pageId, id, blob, frame, alt)], label, [page]);
    }
    if (code === undefined) {
      this.#remember(id, image, fit.smaller);
      this.select(page, id);
    }
  }

  /** Whether the open document claims PDF/UA, which wants alternative text on every picture. */
  async #claimsPdfUa(): Promise<boolean> {
    try {
      const { client } = await this.#engine.view();
      return (await client.metadata()).pdfua !== undefined;
    } catch {
      return false;
    }
  }

  /** Puts `image` in the place of the picture `item`: shown whole, as large as fits in its box, centred and turned as it was. */
  async #replaceImage(item: Item, image: ReadImage, blob: string): Promise<void> {
    const box = boxOf(item.frame);
    const ratio = image.width / image.height;
    const width = box.width / box.height > ratio ? box.height * ratio : box.width;
    const height = width / ratio;
    const frame = reshaped(item.frame, {
      x: box.x + (box.width - width) / 2,
      y: box.y + (box.height - height) / 2,
      width,
      height,
    });
    const op = updateOp(this.#pageId(item.page), item.id, { blob, matrix: frame, crop: null });
    if (await this.#apply([op], laterMessage("content.step.replace"), [item.page]))
      this.#remember(item.id, image, false);
  }

  /** Puts the picture in `blob` in the place of the one picked image the file already had. */
  async #replacePicked(blob: string): Promise<void> {
    await this.#originals.replaceImage(blob);
  }

  /** Asks for alternative text for the picture `name`; undefined when the person cancels, "decorative" for decoration. */
  #askAlt(name: string): Promise<AltAnswer | undefined> {
    return new Promise((resolve) => {
      this.#answerAlt = resolve;
      this.#set({ askingAlt: stemOf(name) });
    });
  }

  /** The answer to the alternative-text question: text or decoration, or undefined to cancel. */
  answerAlt(alt: AltAnswer | undefined): void {
    const answer = this.#answerAlt;
    this.#answerAlt = undefined;
    this.#set({ askingAlt: undefined });
    answer?.(alt);
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
    void this.#flushNudge();
    const item = id === undefined ? undefined : this.itemsOn(page).find((i) => i.id === id);
    this.#frame = undefined;
    this.#guides = undefined;
    if (item !== undefined) {
      this.#originals.clear();
      this.#text.editor.clear();
    }
    this.#set({ selected: item });
    if (item === undefined) {
      if (this.#state.picked === undefined && this.#layout.getState().inspector?.id === CONTENT_INSPECTOR)
        this.#layout.closeInspector();
    } else {
      this.#layout.openInspector({ id: CONTENT_INSPECTOR, selection: true });
      if (this.#state.tool !== "select") this.useTool("select");
    }
    this.#render();
  }

  /** Clears the selection. */
  clearSelection(): void {
    this.#originals.clear();
    this.#text.editor.clear();
    this.select(0);
  }

  /** The pick changed: the state, the inspector and the layer follow it. */
  #pickChanged(): void {
    const picked = this.#originals.pick;
    if (picked !== this.#state.picked) this.#set({ picked });
    if (picked !== undefined) {
      this.#layout.openInspector({ id: CONTENT_INSPECTOR, selection: true });
      if (this.#state.tool !== "select") this.useTool("select");
    } else if (
      this.#state.selected === undefined &&
      this.#state.typing === undefined &&
      this.#state.textPick === undefined &&
      this.#layout.getState().inspector?.id === CONTENT_INSPECTOR
    ) {
      this.#layout.closeInspector();
    }
    this.#render();
  }

  /** The pick or the typing of Edit text changed: the state, the inspector and the layer follow it. */
  #textChanged(): void {
    const { pick, typing } = this.#text.editor;
    const before = this.#state.textTyping;
    if (pick !== this.#state.textPick || typing !== before) this.#set({ textPick: pick, textTyping: typing });
    if (this.#on && pick !== undefined) {
      this.#layout.openInspector({ id: CONTENT_INSPECTOR, selection: true });
    } else if (
      this.#on &&
      pick === undefined &&
      this.#state.selected === undefined &&
      this.#state.picked === undefined &&
      this.#state.typing === undefined &&
      this.#layout.getState().inspector?.id === CONTENT_INSPECTOR
    ) {
      this.#layout.closeInspector();
    }
    this.#render();
    // A line just opened (or reverted) puts the caret in its box.
    if (typing !== undefined && (before === undefined || before.version !== typing.version)) {
      requestAnimationFrame(() => this.#layer.focusRunEditor());
    }
  }

  /** Types into the lines of the block the pick is on (the Text block inspector's Edit text). */
  async editPickedText(): Promise<void> {
    const pick = this.#originals.pick;
    if (pick === undefined) return;
    const found = await this.#text.editBlock(pick.page, pick.box);
    if (found || this.#text.editor.pick !== undefined) this.#enterTextTool();
  }

  /** Edit text is the tool on, with what it has picked left as it is. */
  #enterTextTool(): void {
    this.#set({ tool: "edit-text" });
    this.#tools.use("contentSelect");
    this.#render();
  }

  /** Select takes the block of the page's own content around `box`, to move or delete it. */
  async #selectBlock(page: number, box: Box): Promise<void> {
    this.useTool("select");
    await this.#originals.read([page]);
    const centre = { x: (box[0] + box[2]) / 2, y: (box[1] + box[3]) / 2 };
    const found = hitOriginal(this.#originals.on(page), centre, true);
    if (found !== undefined) this.#originals.choose(page, [found]);
  }

  /** Types into the one picked line (Edit text in the palette and the inspector). */
  editText(): void {
    if (this.#originals.pick?.objects[0]?.kind === "text") void this.editPickedText();
    else if (this.#text.editor.pick !== undefined) {
      this.#enterTextTool();
      this.#text.editor.editPicked();
    } else if (this.#state.tool !== "edit-text") this.useTool("edit-text");
  }

  /** Finishes typing into the line: saves it. */
  finishText(): void {
    void this.#text.editor.commit("done");
  }

  /** Puts the line's original text back in its box. */
  revertText(): void {
    this.#text.editor.revert();
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
    return (await this.#tryApply(ops, label, pages)) === undefined;
  }

  /**
   * Applies `ops` after every change still in flight, so each is made from what the one before
   * left; undefined when they were made, else the refusal's code ("" when it had none).
   */
  #tryApply(ops: readonly Op[], label: string, pages: readonly number[]): Promise<string | undefined> {
    return this.#enqueue(() => this.#applyNow(ops, label, pages));
  }

  /** `#tryApply` without waiting its turn: for work already running in the queue. */
  async #applyNow(ops: readonly Op[], label: string, pages: readonly number[]): Promise<string | undefined> {
    try {
      await this.#viewer.edits.apply(ops, label);
    } catch (error) {
      const code = codeOf(error);
      // The session's failure toast says why; a refused font or picture is named here, and a picture
      // without alternative text is asked about by the caller.
      if (code.startsWith("content-") && code !== NEEDS_ALT) {
        const reason = error instanceof Error ? error.message : code;
        this.#host.toasts.add({ message: laterMessage("content.failed", { reason }) });
      }
      return code;
    }
    await this.#refresh(pages);
    return undefined;
  }

  /** Runs `work` once the changes before it are done; a failure of one doesn't stop the next. */
  #enqueue<T>(work: () => Promise<T>): Promise<T> {
    const run = this.#queue.then(work);
    this.#queue = run.then(
      () => undefined,
      () => undefined,
    );
    return run;
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

  /** Changes object `item` to the frame `frame` as one step named for `step`; a text box's new `width` goes with it. */
  async setFrame(
    item: Item,
    frame: Matrix,
    step: "move" | "resize" | "turn" = "move",
    width?: number,
  ): Promise<boolean> {
    const label = laterMessage(`content.step.${step}`, { kind: this.kindOf(item) });
    const pageId = this.#pageId(item.page);
    const rebuilt = step === "resize" ? resizedShape(item, frame) : undefined;
    if (rebuilt !== undefined) return this.#apply(this.#rebuildOps(item, rebuilt), label, [item.page]);
    const spec = item.added.object;
    const text = width !== undefined && spec !== undefined && "text" in spec ? { text: { ...spec.text, width } } : {};
    const op = updateOp(pageId, item.id, { matrix: matrixFor(item, frame, width), ...text });
    return this.#apply([op], label, [item.page]);
  }

  /**
   * A shape given a new path as one step: it is taken off and put back with the same ID, look and
   * place among the others, since no op changes a path. The outline keeps its width.
   */
  #rebuildOps(item: Item, path: PathSpec): Op[] {
    const spec = item.added.object;
    if (spec === undefined || !("shape" in spec)) return [];
    const pageId = this.#pageId(item.page);
    const layer = this.itemsOn(item.page).filter((i) => i.added.layer === item.added.layer);
    const index = layer.findIndex((i) => i.id === item.id);
    const add = {
      op: "addContentObject",
      args: {
        id: item.id,
        page: pageId,
        object: { shape: { ...spec.shape, path } },
        matrix: item.added.matrix,
        opacity: item.added.opacity,
        layer: item.added.layer,
        ...(item.added.alt === undefined ? {} : { alt: item.added.alt }),
        modified: modifiedNow(),
      },
    } as unknown as Op;
    const back = index >= 0 && index < layer.length - 1 ? [updateOp(pageId, item.id, { order: index })] : [];
    return [deleteOp(pageId, item.id), add, ...back];
  }

  /** Moves the selection by (`dx`, `dy`) points: the keys and Position fields' way. */
  nudgeSelection(dx: number, dy: number): void {
    const item = this.#state.selected;
    if (item === undefined) {
      this.#originals.nudge(dx, dy);
      return;
    }
    if (this.#nudge !== undefined) clearTimeout(this.#nudge.timer);
    const total = { dx: (this.#nudge?.dx ?? 0) + dx, dy: (this.#nudge?.dy ?? 0) + dy };
    this.#nudge = { item, ...total, timer: setTimeout(() => void this.#flushNudge(), NUDGE_MS) };
    this.#frame = moved(item.frame, total.dx, total.dy);
    this.#render();
  }

  /**
   * Undo and Redo, from the keys, the top bar or the palette, wait for every change still in
   * flight (a nudge gathering or being made), so they act on it, not on the step before. Returns
   * what puts the originals back.
   */
  #afterChanges(): () => void {
    const edits = this.#viewer.edits as unknown as Record<"undo" | "redo", () => Promise<unknown>>;
    const original = { undo: edits.undo, redo: edits.redo };
    for (const way of ["undo", "redo"] as const) {
      edits[way] = async () => {
        await this.#flushNudge();
        await this.#queue;
        return original[way].call(this.#viewer.edits);
      };
    }
    return () => {
      edits.undo = original.undo;
      edits.redo = original.redo;
    };
  }

  /**
   * Makes the pending nudge one step, moving the object from where the changes before it left it
   * (a nudge asked for while another was being made would otherwise start from the old place).
   */
  #flushNudge(): Promise<void> {
    const nudge = this.#nudge;
    if (nudge === undefined) return this.#originals.flush().then(() => this.#queue);
    clearTimeout(nudge.timer);
    this.#nudge = undefined;
    const { item } = nudge;
    const label = laterMessage("content.step.move", { kind: this.kindOf(item) });
    return this.#enqueue(async () => {
      const latest = this.itemsOn(item.page).find((i) => i.id === item.id) ?? item;
      const op = updateOp(this.#pageId(item.page), item.id, {
        matrix: matrixFor(latest, moved(latest.frame, nudge.dx, nudge.dy)),
      });
      await this.#applyNow([op], label, [item.page]);
      if (this.#nudge === undefined) {
        this.#frame = undefined;
        this.#render();
      }
    });
  }

  /** Deletes the selection. */
  deleteSelection(): void {
    const item = this.#state.selected;
    if (item === undefined && this.#originals.pick === undefined && this.#text.editor.pick !== undefined) {
      void this.#text.editor.deleteText();
      return;
    }
    if (item === undefined) {
      void this.#originals.delete();
      return;
    }
    void this.#flushNudge();
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
              color: rgbOf(spec.text.color) ?? INK,
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
    // The copy is set in the same font and shows the same file line as what it copies.
    const font = this.#objectFonts.get(item.id);
    if (font !== undefined) this.#objectFonts.set(id, font);
    const facts = this.#facts.get(item.id);
    if (facts !== undefined) this.#facts.set(id, facts);
    void this.#apply([op], label, [item.page]).then((made) => {
      if (made) this.select(item.page, id);
    });
  }

  /** Moves the selection among the objects on its layer. */
  arrangeSelection(how: Arrange): void {
    const item = this.#state.selected;
    if (item === undefined) {
      void this.#originals.arrange(how);
      return;
    }
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
    // Nothing changed: no step to undo.
    const before = lookOfBlock(spec.text, this.#objectFonts.get(item.id));
    const same =
      spec.text.runs.map((r) => r.text).join("") === text &&
      before.size === look.size &&
      before.align === look.align &&
      before.bold === look.bold &&
      before.italic === look.italic &&
      before.fontId === typing.look.fontId &&
      before.color.every((v, i) => Math.abs(v - (look.color[i] ?? 0)) < 1e-6);
    if (same) return;
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
    const file = await pickedFile(picked);
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
    if (this.#state.tool === "edit-text") this.#text.prefetch();
    const current = this.#viewer.getState().currentPage;
    for (const page of [current - 1, current, current + 1]) {
      if (page >= 0 && !this.#items.has(page) && this.sizeOf(page) !== undefined) void this.#refresh([page]);
    }
  }

  /** Marks what the file drew on `pages` as old and reads it again once nothing has changed for a moment. */
  #readLater(pages: readonly number[]): void {
    this.#originals.invalidate(pages);
    this.#later.push(...pages);
    if (this.#laterTimer !== undefined) clearTimeout(this.#laterTimer);
    this.#laterTimer = setTimeout(() => {
      this.#laterTimer = undefined;
      const due = [...new Set(this.#later.splice(0))];
      if (this.#on && due.length > 0) void this.#originals.read(due);
      if (this.#on && this.#state.tool === "edit-text" && due.length > 0) void this.#text.editor.read(due);
    }, READ_AFTER_MS);
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
        if (added === undefined) {
          // Keep what the page held, and the selection: the next press reads it again.
          this.#stale.add(page);
          return;
        }
        this.#stale.delete(page);
        this.#items.set(
          page,
          added.flatMap((a) => itemOf(page, a) ?? []),
        );
      }),
    );
    const selected = this.#state.selected;
    const again = selected === undefined ? undefined : this.itemsOn(selected.page).find((i) => i.id === selected.id);
    // A drag or a nudge in progress keeps its outline; what it made is read when it is done.
    if (this.#drag === undefined && this.#nudge === undefined) {
      this.#frame = undefined;
      this.#guides = undefined;
    }
    this.#set({ selected: again });
    // What the file drew is read on the same worker as everything else, so only a pick that must
    // follow its objects reads it now; otherwise it is read again once the edits stop.
    if (this.#originals.pick !== undefined) await this.#originals.read(pages);
    else this.#readLater(pages);
    if (this.#text.editor.pick !== undefined || this.#text.editor.typing !== undefined) {
      await this.#text.editor.read(pages);
    } else this.#text.editor.invalidate(pages);
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
      case "edit-text":
        this.#text.press(at.page, point, event.shiftKey);
        return false;
      default: {
        // A page not read yet: read it, then select what the press was on.
        if (this.#stale.has(at.page)) void this.#refresh([at.page]);
        if (!this.#items.has(at.page)) {
          void this.#refresh([at.page]).then(async () => {
            const found = this.#hit(at.page, point);
            if (found !== undefined) {
              this.select(at.page, found.id);
              return;
            }
            await this.#originals.read([at.page]);
            const original = hitOriginal(this.#originals.on(at.page), point);
            if (original !== undefined) this.#originals.choose(at.page, [original]);
          });
          return false;
        }
        const item = this.#hit(at.page, point);
        if (item === undefined) {
          this.select(0);
          if (this.#originals.has(at.page)) return this.#originals.down(at.page, point, event.shiftKey, event.altKey);
          // What the file drew is old: read it, then pick what the press was on.
          void this.#originals.read([at.page]).then(() => {
            const original = hitOriginal(this.#originals.on(at.page), point, event.altKey);
            if (original !== undefined) this.#originals.choose(at.page, [original]);
          });
          return false;
        }
        this.#originals.clear();
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
    if (drag === undefined && this.#originals.dragging) {
      const page = this.#originals.dragPage();
      const now = page === undefined ? undefined : this.#pagePoint(page, event);
      if (now !== undefined) this.#originals.move(now, event.altKey, event.shiftKey);
      return;
    }
    if (drag === undefined) {
      const polygon = this.#polygon;
      const point = polygon === undefined ? undefined : this.#pagePoint(polygon.page, event);
      if (polygon !== undefined && point !== undefined) {
        polygon.hover = point;
        this.#showPolygon();
      }
      return;
    }
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
    const others = this.itemsOn(drag.item.page)
      .filter((i) => i.id !== drag.item.id)
      .map((i) => i.frame);
    const made = movedFrame(drag.item.frame, drag.from, now, this.sizeOf(drag.item.page), others, noSnap);
    this.#frame = made.frame;
    this.#guides = made.guides.length === 0 ? undefined : { page: drag.item.page, guides: made.guides };
  }

  #dragHandle(drag: Extract<Drag, { kind: "handle" }>, now: Point, shift: boolean): void {
    const { item } = drag;
    this.#frame = handleFrame(item.frame, item.kind === "text", drag.handle, drag.from, now, shift);
  }

  /** A pointer that was cancelled (a palm, a lost capture) drops what it was doing; nothing is made. */
  #cancel(): void {
    if (this.#originals.dragging) this.#originals.up({ x: 0, y: 0 }, true);
    this.#drag = undefined;
    this.#frame = undefined;
    this.#guides = undefined;
    this.#draft = undefined;
    this.#render();
  }

  #up(event: PointerEvent): void {
    const drag = this.#drag;
    this.#drag = undefined;
    this.#guides = undefined;
    if (drag === undefined && this.#originals.dragging) {
      const page = this.#originals.dragPage();
      this.#originals.up((page === undefined ? undefined : this.#pagePoint(page, event)) ?? { x: 0, y: 0 }, false);
      return;
    }
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
    // A text box's sides set its width: the block takes the new width, and its left edge moves with the west handle.
    await this.setFrame(item, frame, step, item.kind === "text" ? sizeOf(frame).width : undefined);
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
    if (this.#state.tool === "edit-text" && this.#text.key(event)) {
      event.preventDefault();
      return;
    }
    if (event.key === "Escape") this.#escape();
    else if (event.key === "[" || event.key === "]") this.#cycle(event.key === "]" ? 1 : -1, event);
    else if (event.key === "Enter") this.#enter(event);
    else if (this.#menuKey(event)) event.preventDefault();
    else if (
      this.#state.selected !== undefined ||
      this.#originals.pick !== undefined ||
      this.#text.editor.pick !== undefined
    )
      this.#selectionKey(event);
  }

  /**
   * `[` and `]` step through the objects on the page, from nothing picked too: the ones added
   * first, then the ones the file had, in painting order, skipping what can't be changed. The keys'
   * way to select what the pointer would click.
   */
  #cycle(step: 1 | -1, event: KeyboardEvent): void {
    const page = this.#state.selected?.page ?? this.#originals.pick?.page ?? this.#viewer.getState().currentPage;
    const stops: readonly (() => void)[] = [
      ...this.itemsOn(page).map((item) => () => this.select(page, item.id)),
      ...this.#originals
        .on(page)
        .filter((o) => o.readOnly === undefined && !o.background)
        .map((o) => () => this.#originals.choose(page, [o])),
    ];
    if (stops.length === 0) return;
    const items = this.itemsOn(page);
    const held = this.#originals.pick?.objects[0];
    const heldAt = items.findIndex((i) => i.id === this.#state.selected?.id);
    const selectable = this.#originals.on(page).filter((o) => o.readOnly === undefined && !o.background);
    const at = heldAt >= 0 ? heldAt : held === undefined ? -1 : items.length + selectable.indexOf(held);
    const next = at < 0 ? (step > 0 ? 0 : stops.length - 1) : (at + step + stops.length) % stops.length;
    event.preventDefault();
    stops[next]?.();
  }

  // The context menu and what Select says of the objects the file had.

  /** The hover outline: what a press at the pointer would pick, while nothing is held. */
  #hover(event: PointerEvent): void {
    if (event.buttons !== 0 || this.#state.typing !== undefined) return;
    if (this.#state.tool === "edit-text") {
      const over = pointOn(this.#viewer, this.#engine, event);
      if (over === undefined) this.#text.editor.unhover();
      else this.#text.hover(over.page, { x: over.x, y: over.y });
      return;
    }
    if (this.#state.tool !== "select") return;
    const at = pointOn(this.#viewer, this.#engine, event);
    if (at === undefined || !this.#originals.has(at.page) || this.#hit(at.page, { x: at.x, y: at.y }) !== undefined) {
      this.#originals.unhover();
      return;
    }
    this.#originals.hoverAt(at.page, { x: at.x, y: at.y });
  }

  /** A double-click with Select: types into the text under it, whether the page's own or a box added to it. */
  #doubleClick(event: MouseEvent): void {
    if (this.#state.tool !== "select" || this.#state.typing !== undefined) return;
    const at = pointOn(this.#viewer, this.#engine, event);
    if (at === undefined) return;
    const point = { x: at.x, y: at.y };
    const item = this.#hit(at.page, point);
    if (item?.kind === "text") {
      this.select(at.page, item.id);
      this.editSelectedText();
    } else if (item === undefined) {
      void this.#text.doubleClick(at.page, point).then((found) => {
        if (found) this.#enterTextTool();
      });
    }
  }

  /** A right-click on the page: picks what is under it, if it isn't picked, and opens the menu there. */
  #contextMenu(event: MouseEvent): void {
    if (this.#state.tool !== "select" || this.#state.typing !== undefined) return;
    const at = pointOn(this.#viewer, this.#engine, event);
    if (at === undefined) return;
    const point = { x: at.x, y: at.y };
    const item = this.#hit(at.page, point);
    if (item !== undefined) this.select(at.page, item.id);
    else {
      const found = hitOriginal(this.#originals.on(at.page), point);
      const held = this.#originals.pick?.objects.some((o) => o === found) === true;
      if (found !== undefined && !held) this.#originals.choose(at.page, [found]);
    }
    if (this.#state.selected === undefined && this.#originals.pick === undefined) return;
    event.preventDefault();
    this.#openMenuAt(event.clientX, event.clientY);
  }

  /** Shift+F10 or the menu key: opens the menu at the selection. */
  #menuKey(event: KeyboardEvent): boolean {
    if (event.key !== "ContextMenu" && !(event.key === "F10" && event.shiftKey)) return false;
    const item = this.#state.selected;
    const box = item === undefined ? this.#originals.pick?.box : boxAround(corners(item.frame));
    const page = item?.page ?? this.#originals.pick?.page;
    const rect = page === undefined ? undefined : this.#layer.rectOf(page);
    const size = page === undefined ? undefined : this.sizeOf(page);
    if (box === undefined || rect === undefined || size === undefined) return false;
    const [x, y] = turn(
      (box[0] + box[2]) / 2 / size.width,
      (box[1] + box[3]) / 2 / size.height,
      this.#viewer.getState().rotation,
    );
    this.#openMenuAt(rect.left + x * rect.width, rect.top + y * rect.height);
    return true;
  }

  #openMenuAt(x: number, y: number): void {
    this.closeMenu(false);
    this.#menuOpener = document.activeElement;
    const anchor = document.createElement("div");
    anchor.style.cssText = `position:fixed;left:${x}px;top:${y}px;width:1px;height:1px;pointer-events:none`;
    anchor.setAttribute("aria-hidden", "true");
    document.body.append(anchor);
    this.#set({ menuAt: anchor });
  }

  /** Closes the context menu and, unless `focus` says not to, gives the focus back to what had it. */
  closeMenu(focus = true): void {
    const anchor = this.#state.menuAt;
    if (anchor === undefined) return;
    anchor.remove();
    this.#set({ menuAt: undefined });
    const opener = this.#menuOpener;
    this.#menuOpener = null;
    if (!focus) return;
    const back = opener instanceof HTMLElement && opener.isConnected ? opener : this.#viewer.element;
    back.focus({ preventScroll: true });
  }

  /** Sets the alternative text of the figure that draws the picked picture, as one step. */
  async setAlt(text: string): Promise<boolean> {
    const [image] = this.#originals.pick?.objects ?? [];
    if (image === undefined || image.kind !== "image" || text.trim() === "") return false;
    const op: Op = { op: "setAlt", args: { page: this.#pageId(image.page), object: image.ref, alt: text.trim() } };
    return this.#apply([op], laterMessage("content.original.step.alt"), [image.page]);
  }

  /**
   * Fix it first (A07): selects the picture whose replacement left it without alternative text,
   * or else the first picture of the page in view that can be changed, and asks the inspector to
   * put the focus in Alternative text.
   */
  fixAlt(): void {
    const page = this.#viewer.getState().currentPage;
    const found =
      this.#originals.replacedOn(page) ??
      this.#originals.on(page).find((o) => o.kind === "image" && o.readOnly === undefined);
    if (found !== undefined) this.#originals.choose(page, [found]);
    this.#set({ fieldFocus: "alt" });
  }

  /** Crop image: the focus goes to the crop's first inset, once the inspector shows it. */
  focusCrop(): void {
    this.#set({ fieldFocus: "crop" });
  }

  /** The inspector put the focus where it was asked to. */
  fieldFocused(): void {
    if (this.#state.fieldFocus !== undefined) this.#set({ fieldFocus: undefined });
  }

  /** Escape's steps: a polygon being drawn, then the tool on, then the selection. */
  #escape(): void {
    if (this.#polygon !== undefined) {
      this.#polygon = undefined;
      this.#draft = undefined;
      this.#render();
    } else if (this.#state.menuAt !== undefined) this.closeMenu();
    else if (this.#text.editor.pick !== undefined) this.#text.editor.clear();
    else if (this.#state.tool !== "select") this.useTool("select");
    else if (this.#state.selected !== undefined) this.select(0);
    else if (this.#originals.pick !== undefined) this.#originals.clear();
  }

  /** Enter closes a polygon, or starts typing into the selected text. */
  #enter(event: KeyboardEvent): void {
    const page = this.#viewer.getState().currentPage;
    const size = this.sizeOf(page);
    const { tool, shape } = this.#state;
    if (this.#polygon !== undefined) void this.#closePolygon();
    else if (tool === "text" && size !== undefined) {
      // Without a pointer: a box at the top of the page, as wide as its text.
      const top = 2 * MARGIN;
      this.openText(page, [
        top,
        top,
        Math.max(size.width - top, top + NARROWEST_TEXT),
        top + this.#state.look.size * 1.4,
      ]);
    } else if (tool === "shape" && shape !== "polygon" && size !== undefined) {
      // Without a pointer: a shape in the middle of the page.
      const from = { x: size.width / 2 - DEFAULT_SHAPE / 2, y: size.height / 2 - DEFAULT_SHAPE / 2 };
      void this.#finishShape({ kind: "shape", page, from, shape }, from, false, false);
    } else if (this.#state.selected?.kind === "text") this.editSelectedText();
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
    // The lines of the page's own text have no place to nudge: the arrow keys keep scrolling the page.
    if (this.#state.selected === undefined && this.#originals.pick === undefined) return;
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
      onRunType: (text) => this.#text.editor.typed(text),
      onRunKey: (key) => this.#text.onKey(key),
      onRunBlur: (next) => this.#text.onBlur(next),
    };
  }

  #startHandle(handle: HandleName | "turn", event: PointerEvent): void {
    const item = this.#state.selected;
    const pick = this.#originals.pick;
    const page = item?.page ?? pick?.page;
    if (page === undefined) return;
    const from = this.#pagePoint(page, event);
    if (from === undefined) return;
    if (item === undefined) this.#originals.startHandle(handle, from);
    else this.#drag = { kind: "handle", item, handle, from };
    const move = (e: PointerEvent) => this.#move(e);
    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", cancel);
    };
    const up = (e: PointerEvent) => {
      stop();
      this.#up(e);
    };
    const cancel = () => {
      stop();
      this.#cancel();
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", cancel);
  }

  #render(): void {
    const { selected, typing } = this.#state;
    const handles: readonly HandleName[] = selected?.kind === "text" ? TEXT_HANDLES : HANDLES;
    const locked = this.#viewer.edits.getState().blocked !== undefined;
    this.#layer.show({
      ...this.#pickView(),
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
      text: this.#text.view(this.#state.tool === "edit-text"),
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

  /** The layer's view of the picked objects, the hover outline and the marquee. */
  #pickView(): Partial<Parameters<ContentLayer["show"]>[0]> {
    const pick = this.#originals.pick;
    const frame = this.#originals.frame();
    const { names, refusal } = this.#originals.handles();
    const { hover, marquee } = this.#originals.view();
    const locked = refusal !== undefined || this.#viewer.edits.getState().blocked !== undefined;
    return {
      ...(pick === undefined || frame === undefined
        ? {}
        : { selection: { page: pick.page, frame, handles: names, turn: true, locked } }),
      ...(hover === undefined ? {} : { hover }),
      ...(marquee === undefined ? {} : { draft: { page: marquee.page, kind: "box" as const, box: marquee.box } }),
    };
  }

  #set(change: Partial<ContentState>): void {
    this.#state = { ...this.#state, ...change };
    for (const listener of [...this.#listeners]) listener();
  }
}

/** A font file the user chose, added to the session's registry; the refusal's words when it can't be used. */
async function addFontFile(file: File, toasts: ToastQueue): Promise<FontChoice | undefined> {
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
