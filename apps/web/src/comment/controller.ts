// Comment mode's state over one open document (Paper C01, C02): the tool on and the style each
// tool remembers on this device, the annotation layer while the mode is current, the selection
// the inspector shows, the banner on a file whose certification forbids comments, and the notice
// when an opaque shape hides text. Tools draw through `DrawHost`: a preview while the pointer
// moves, then one `addAnnot`, its preview gone in the frame the page's new pixels show
// (AnnotationGesture). The inspector's changes are changes.ts's.

import type { Op, PageAnnotation, Violation } from "@laminapdf/sdk";
import type { ShellLayout } from "@laminapdf/ui";
import {
  type CommentColour,
  type CommentTool,
  laterMessage,
  type ShapeKind,
  type ToolRegistry,
} from "@laminapdf/ui/editing";
import {
  type AnnotationGesture,
  AnnotationLayer,
  type AnnotationWords,
  type PageFrame,
  type TextSelection,
  type Viewer,
  type ViewerClient,
} from "@laminapdf/viewer";
import { pointOn, selectedLines } from "../page-points.ts";
import type { Engine } from "../session.ts";
import { AnnotationChanges } from "./changes.ts";
import { COMMENT_INSPECTOR } from "./inspector-id.ts";
import {
  colourValue,
  coversText,
  type Mark,
  type MarkStyle,
  markPreview,
  type PageText,
  type Point,
  snapLines,
} from "./marks.ts";
import { ARROW_ENDS, addAnnotOp, canResize, kindName, kindOf, resizeOp, subtypeOf } from "./ops.ts";
import {
  markStyle,
  type StyleKey,
  storeAuthor,
  storedAuthor,
  storedStyles,
  storeStyles,
  styleKey,
  type ToolStyle,
  type ToolStyles,
} from "./styles.ts";
import { type DrawHost, type MarkTool, markTools, placedMark } from "./tools.ts";

/** The banner on a file whose signature forbids comments (C01). */
interface CommentBanner {
  readonly kind: "p1" | "p2" | "usageRights";
  readonly violations: readonly Violation[];
}

/** Comment mode's state. */
export interface CommentState {
  readonly tool: CommentTool;
  readonly shape: ShapeKind;
  readonly styles: ToolStyles;
  readonly selection: readonly PageAnnotation[];
  readonly page: number | undefined;
  /** Comments on the page in view, for the toolbar. */
  readonly count: number;
  readonly banner: CommentBanner | undefined;
  /** An opaque shape was drawn over text: the notice that the text stays in the file. */
  readonly covered: boolean;
  /** How many covered-text checks have finished, whatever they found. */
  readonly checks: number;
  /** The first creation tool asks for the name comments carry. */
  readonly askingAuthor: boolean;
  /** Bumped when a note is placed and selected: the inspector focuses its note field. */
  readonly noteFocus: number;
  /** Bumped when the session learns what it wrote of an annotation: the inspector shows it. */
  readonly known: number;
}

/** The layer's words, from the later catalogue. */
const WORDS: AnnotationWords = {
  label: (a) =>
    a.contents
      ? laterMessage("kind.labelled", { kind: kindName(a.subtype), contents: a.contents })
      : kindName(a.subtype),
  move: laterMessage("comment.step.move"),
  resize: laterMessage("comment.step.resize"),
};

/** The subtypes the toolbar's count leaves out: pop-ups go with their note, links and widgets aren't comments. */
const NOT_COMMENTS = new Set(["Popup", "Link", "Widget"]);

/** Comment mode's state and actions over one open document. */
export class CommentController {
  readonly #engine: Engine;
  readonly #viewer: Viewer;
  readonly #tools: ToolRegistry;
  readonly #layout: ShellLayout;
  /** The document's worker client, once the mode has been entered. */
  #client: ViewerClient | undefined;
  #state: CommentState;
  readonly #listeners = new Set<() => void>();
  #layer: AnnotationLayer | undefined;
  #marks: MarkTool[] = [];
  #stops: (() => void)[] = [];
  readonly #gestures = new Map<number, AnnotationGesture>();
  /** Each page's text once read, by page; dropped when an edit redraws the page. */
  readonly #texts = new Map<number, PageText | "reading">();
  /** Each page's frame once read; a page's frame changes only with page setup, another mode's. */
  readonly #frames = new Map<number, Promise<PageFrame | undefined>>();
  #selection: TextSelection | undefined;
  /** The note just placed, whose field takes the focus once the layer selects it. */
  #focusNote: string | undefined;
  #current = false;
  /** The inspector's changes to the selection. */
  readonly changes: AnnotationChanges;

  constructor(engine: Engine, viewer: Viewer, tools: ToolRegistry, layout: ShellLayout) {
    this.#engine = engine;
    this.#viewer = viewer;
    this.#tools = tools;
    this.#layout = layout;
    this.#state = {
      tool: "select",
      shape: "rectangle",
      styles: storedStyles(),
      selection: [],
      page: undefined,
      count: 0,
      banner: undefined,
      covered: false,
      checks: 0,
      askingAuthor: false,
      noteFocus: 0,
      known: 0,
    };
    this.changes = new AnnotationChanges(
      viewer.edits,
      () => this.#state.selection,
      () => this.#set({ known: this.#state.known + 1 }),
    );
  }

  getState(): CommentState {
    return this.#state;
  }

  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  /** Comment became the mode: the layer, the tools, the banner and the page's count. */
  enter(): void {
    if (this.#current) return;
    this.#current = true;
    void this.#engine.view().then(({ client }) => {
      if (!this.#current || this.#layer !== undefined) return;
      this.#client = client;
      this.#attach(client);
    }, reportError);
  }

  /** The layer, the tools and the listeners, over the document's worker `client`. */
  #attach(client: ViewerClient): void {
    const layer = new AnnotationLayer(this.#viewer, client, {
      words: WORDS,
      onSelectionChange: (selection) => this.#selected(selection),
      resize: { can: canResize, ops: (page, annotation, resized) => this.#resizeOps(page, annotation, resized) },
    });
    this.#layer = layer;
    this.#marks = markTools(this.#host());
    const unregister = this.#marks.map((tool) => this.#tools.register(tool));
    const element = this.#viewer.element;
    const key = (event: KeyboardEvent) => this.#key(event);
    element.addEventListener("keydown", key);
    this.#stops = [
      this.#viewer.addLayer(layer),
      () => layer.destroy(),
      ...unregister,
      () => element.removeEventListener("keydown", key),
      this.#viewer.on("pagechange", ({ page }) => {
        this.#text(page);
        void this.#count();
      }),
      this.#viewer.on("invalidate", ({ pages }) => {
        for (const { index } of pages) this.#texts.delete(index);
        void this.#count();
      }),
      this.#viewer.on("selectionchange", ({ selection }) => {
        this.#selection = selection;
      }),
    ];
    this.#text(this.#viewer.getState().currentPage);
    void this.#count();
    void this.#checkBanner();
  }

  /** Comment is no longer the mode: tools off, the layer gone, the inspector closed. */
  leave(): void {
    if (!this.#current) return;
    this.#current = false;
    for (const tool of this.#marks) tool.cancel();
    this.#tools.use("select");
    for (const stop of this.#stops.splice(0)) stop();
    this.#layer = undefined;
    this.#marks = [];
    this.#frames.clear();
    if (this.#layout.getState().inspector?.id === COMMENT_INSPECTOR) this.#layout.closeInspector();
    this.#set({ tool: "select", selection: [], page: undefined, banner: undefined, covered: false });
  }

  /** Turns `tool` on (with `shape` for Shapes); a text tool with text selected marks it at once. */
  use(tool: CommentTool, shape: ShapeKind = this.#state.shape): void {
    if (!this.#current) return;
    for (const t of this.#marks) t.done();
    this.#tools.use(tool === "shape" ? shape : tool);
    // A tool that draws takes the pointer; the annotations let it through.
    this.#layer?.letPointerThrough(tool !== "select");
    if (tool !== "select") this.#layer?.select(this.#state.page ?? 0, []);
    this.#set({ tool, shape });
    if (tool !== "select" && storedAuthor() === undefined) this.#set({ askingAuthor: true });
    if ((tool === "highlight" || tool === "underline" || tool === "strike") && this.#selection !== undefined) {
      void this.#markSelection();
    }
  }

  /** The name comments carry, kept on this device; empty leaves it off. */
  setAuthor(name: string | undefined): void {
    if (name !== undefined) storeAuthor(name);
    this.#set({ askingAuthor: false });
  }

  /** Asks for the name comments carry again (the inspector's Change…, the palette's Your name on comments…). */
  askAuthor(): void {
    this.#set({ askingAuthor: true });
  }

  /** The name comments carry now, or undefined for none. */
  author(): string | undefined {
    const name = storedAuthor();
    return name === undefined || name === "" ? undefined : name;
  }

  /** Sets the colour or width the tool on remembers. */
  setStyle(change: Partial<Pick<ToolStyle, "colour" | "width">>): void {
    const key = styleKey(this.#state.tool, this.#state.shape);
    if (key !== undefined) this.#keepStyle(key, { ...this.#state.styles[key], ...change });
  }

  /** Keeps `style` as the tool under `key`'s. */
  #keepStyle(key: StyleKey, style: ToolStyle): void {
    const styles = { ...this.#state.styles, [key]: style };
    storeStyles(styles);
    this.#set({ styles });
  }

  /** A closed shape's fill from the inspector: the tool that draws it remembers it, and an opaque one is checked for text under it. */
  setFill(fill: CommentColour | "none"): void {
    this.changes.setFill(fill);
    const [first] = this.#state.selection;
    const key: StyleKey =
      first?.subtype === "Circle" ? "ellipse" : first?.subtype === "Polygon" ? "polygon" : "rectangle";
    const { fill: _, ...style } = this.#state.styles[key];
    this.#keepStyle(key, fill === "none" ? style : { ...style, fill });
    void this.#checkSelectionCovers(fill);
  }

  /** Dismisses the covered-text notice. */
  dismissCovered(): void {
    this.#set({ covered: false });
  }

  // --- internals ---

  #selected(selection: readonly PageAnnotation[]): void {
    const [first] = selection;
    const page = first === undefined ? undefined : this.#pageOf(first);
    const focus = first !== undefined && first.id === this.#focusNote;
    if (focus) this.#focusNote = undefined;
    this.#set({
      selection,
      page: page ?? this.#state.page,
      ...(focus ? { noteFocus: this.#state.noteFocus + 1 } : {}),
    });
    if (selection.length > 0) this.#layout.openInspector({ id: COMMENT_INSPECTOR, selection: true });
    else if (this.#layout.getState().inspector?.id === COMMENT_INSPECTOR) this.#layout.closeInspector();
  }

  /** The page of an annotation the layer selected: the one whose records hold it. */
  #pageOf(annotation: PageAnnotation): number | undefined {
    const pages = this.#engine.getState().pages.length;
    for (let page = 0; page < pages; page++) {
      if (this.#layer?.annotations(page)?.some((a) => a.id === annotation.id)) return page;
    }
    return undefined;
  }

  /** The tools' host: points from the pointer, the page's text, the layer's gestures. */
  #host(): DrawHost {
    return {
      at: (input) => {
        const at = pointOn(this.#viewer, this.#engine, input);
        return at === undefined ? undefined : { page: at.page, point: [at.x, at.y] };
      },
      text: (page) => this.#text(page),
      whenText: (page) => this.#textNow(page),
      draw: (page, mark) => this.#draw(page, mark),
      finish: (page, mark) => void this.#finish(page, mark),
      later: (run, ms) => {
        const timer = setTimeout(run, ms);
        return () => clearTimeout(timer);
      },
    };
  }

  /** Page `page`'s text if read; starts reading it. */
  #text(page: number): PageText | undefined {
    const known = this.#texts.get(page);
    if (known !== undefined) return known === "reading" ? undefined : known;
    this.#texts.set(page, "reading");
    void this.#textNow(page);
    return undefined;
  }

  /** Page `page`'s text, waiting for it to be read. */
  async #textNow(page: number): Promise<PageText | undefined> {
    const known = this.#texts.get(page);
    if (known !== undefined && known !== "reading") return known;
    try {
      const { client } = await this.#engine.view();
      const text = await client.pageText(page);
      this.#texts.set(page, text);
      return text;
    } catch {
      this.#texts.delete(page);
      return undefined;
    }
  }

  /** The style the tool on draws with. */
  #style(): MarkStyle {
    return markStyle(this.#state.styles, styleKey(this.#state.tool, this.#state.shape) ?? "highlight");
  }

  #draw(page: number, mark: Mark | undefined): void {
    if (mark === undefined) {
      const gesture = this.#gestures.get(page);
      this.#gestures.delete(page);
      void gesture?.cancel();
      return;
    }
    let gesture = this.#gestures.get(page);
    if (gesture === undefined) {
      if (this.#layer === undefined) return;
      gesture = this.#layer.startGesture(page);
      this.#gestures.set(page, gesture);
    }
    gesture.update(markPreview(mark, this.#style()));
  }

  /** Page `page`'s frame: its crop box, `/Rotate` and `/UserUnit`, read once while the mode is on. */
  #frame(page: number): Promise<PageFrame | undefined> {
    let frame = this.#frames.get(page);
    if (frame === undefined) {
      const info = this.#engine.getState().pages[page];
      frame =
        info === undefined
          ? Promise.resolve(undefined)
          : this.#engine.pageBoxes(page).then(
              (boxes) => ({ crop: boxes.crop, rotate: info.rotation, unit: boxes.userUnit }),
              () => undefined,
            );
      this.#frames.set(page, frame);
    }
    return frame;
  }

  async #finish(page: number, mark: Mark | undefined): Promise<void> {
    const gesture = this.#gestures.get(page) ?? (mark === undefined ? undefined : this.#layer?.startGesture(page));
    this.#gestures.delete(page);
    if (gesture === undefined) return;
    if (mark === undefined) {
      await gesture.cancel();
      return;
    }
    const style = this.#style();
    const key = styleKey(this.#state.tool, this.#state.shape);
    gesture.update(markPreview(mark, style));
    const frame = await this.#frame(page);
    const id = this.#engine.getState().pages[page]?.id;
    if (frame === undefined || id === undefined) {
      await gesture.cancel();
      return;
    }
    const op = addAnnotOp(id, mark, frame, style, { author: this.author(), created: new Date() });
    const label = laterMessage("comment.step.add", { kind: kindName(subtypeOf(mark)).toLocaleLowerCase() });
    try {
      const result = await gesture.commit([op], { label });
      const created = result.createdAnnotations?.[0];
      if (created !== undefined) {
        this.#remember(created, mark, key === undefined ? undefined : this.#state.styles[key]);
        // A note's field takes the focus once the layer has read the note and selected it.
        if (mark.type === "note") {
          this.#focusNote = created;
          this.#layer?.select(page, [created]);
        }
      }
    } catch {
      // A failed edit is said by the session's failure toast; the gesture showed the page as it was.
      return;
    }
    await this.#checkCovers(page, mark, style);
    this.#set({ checks: this.#state.checks + 1 });
  }

  /** What the session knows of a new mark's drawing, which its record won't say. */
  #remember(id: string, mark: Mark, style: ToolStyle | undefined): void {
    if (style === undefined) return;
    const kind = kindOf(subtypeOf(mark));
    this.changes.remember(id, {
      width: style.width,
      fill: style.fill ?? "none",
      border: "solid",
      ...(kind.ends === true
        ? { ends: mark.type === "line" && mark.arrow ? ARROW_ENDS : (["none", "none"] as const) }
        : {}),
      ...(kind.patch === "ink" ? { smoothing: true } : {}),
    });
  }

  /** Shows the covered-text notice when `mark` hides text on page `page`. */
  async #checkCovers(page: number, mark: Mark, style: MarkStyle): Promise<void> {
    if (style.fill === undefined || style.opacity < 1) return;
    const text = await this.#textNow(page);
    if (text !== undefined && coversText(mark, style, text)) this.#set({ covered: true });
  }

  /** After a fill is set from the inspector: the notice, if an opaque selected shape now hides text. */
  async #checkSelectionCovers(fill: CommentColour | "none"): Promise<void> {
    const page = this.#state.page;
    if (page !== undefined && fill !== "none") {
      const style: MarkStyle = { colour: [0, 0, 0], width: 1, opacity: 1, fill: colourValue(fill, "fill") };
      for (const a of this.#state.selection) {
        const mark = shapeMark(a);
        if (mark !== undefined && (a.opacity ?? 1) >= 1) await this.#checkCovers(page, mark, style);
      }
    }
    this.#set({ checks: this.#state.checks + 1 });
  }

  /** The resize the layer asks for, in user space through the page's frame. */
  async #resizeOps(page: number, annotation: PageAnnotation, resized: PageAnnotation): Promise<Op[]> {
    const frame = await this.#frame(page);
    const op = frame === undefined ? undefined : resizeOp(annotation, resized, frame);
    return op === undefined ? [] : [op];
  }

  /** Marks the text selection with the text tool on, its lines snapped to the glyphs. */
  async #markSelection(): Promise<void> {
    const selection = this.#selection;
    if (selection === undefined) return;
    const on = selectedLines(this.#viewer, this.#engine, selection.rects);
    if (on === undefined) return;
    const text = await this.#textNow(on.page);
    const lines = text === undefined ? on.lines : snapLines(text, on.lines);
    const tool = this.#state.tool;
    const kind = tool === "underline" ? "underline" : tool === "strike" ? "strikeOut" : "highlight";
    window.getSelection()?.removeAllRanges();
    this.#selection = undefined;
    await this.#finish(on.page, { type: "markup", kind, lines });
  }

  /** The point in the middle of the page in view, and that page. */
  #centre(): { page: number; point: Point } | undefined {
    const r = this.#viewer.element.getBoundingClientRect();
    const at = pointOn(this.#viewer, this.#engine, { clientX: r.left + r.width / 2, clientY: r.top + r.height / 2 });
    if (at !== undefined) return { page: at.page, point: [at.x, at.y] };
    const page = this.#viewer.getState().currentPage;
    const info = this.#engine.getState().pages[page];
    return info === undefined ? undefined : { page, point: [info.width / 2, info.height / 2] };
  }

  /** Enter places or marks with the tool on, Escape stops it, Delete removes the selection. */
  #key(event: KeyboardEvent): void {
    const target = event.target as HTMLElement | null;
    const typing = target?.closest("input, textarea, select, [contenteditable]") !== null;
    if (typing || event.altKey || event.metaKey || event.ctrlKey) return;
    const tool = this.#state.tool;
    const deleting = event.key === "Delete" || event.key === "Backspace";
    if (event.key === "Escape" && tool !== "select") {
      event.preventDefault();
      for (const t of this.#marks) t.cancel();
      this.use("select");
    } else if (deleting && tool === "select" && this.#state.selection.length > 0) {
      event.preventDefault();
      this.#layer?.select(this.#state.page ?? 0, []);
      this.changes.delete();
    } else if (event.key === "Enter" && tool !== "select" && target?.closest("button") == null) {
      event.preventDefault();
      this.#place(tool);
    }
  }

  /** Enter with `tool` on: marks the text selection, or places a mark in the middle of the page in view. */
  #place(tool: Exclude<CommentTool, "select">): void {
    if (tool === "highlight" || tool === "underline" || tool === "strike") {
      void this.#markSelection();
      return;
    }
    const mode = tool === "shape" ? this.#state.shape : tool;
    if (mode === "polygon" || mode === "polyline") this.#marks.find((t) => t.name === mode)?.done();
    const centre = this.#centre();
    if (centre === undefined) return;
    const mark = placedMark(mode, centre.point, this.#text(centre.page));
    if (mark !== undefined) void this.#finish(centre.page, mark);
  }

  /** Counts the comments on the page in view. */
  async #count(): Promise<void> {
    const page = this.#viewer.getState().currentPage;
    try {
      const records = (await this.#client?.pageAnnotations(page)) ?? [];
      const count = records.filter((a) => !NOT_COMMENTS.has(a.subtype)).length;
      if (count !== this.#state.count) this.#set({ count });
    } catch {
      // The page can't be read: the count stays as it was.
    }
  }

  /** Whether a comment would break the file's signature: the banner from the moment the mode opens (C01). */
  async #checkBanner(): Promise<void> {
    const info = this.#engine.getState().pages[0];
    const frame = await this.#frame(0);
    if (info === undefined || frame === undefined) return;
    const probe = addAnnotOp(info.id, { type: "note", at: [10, 10] }, frame, this.#style(), { created: new Date(0) });
    let violations: Violation[];
    try {
      violations = await this.#engine.wouldViolate([probe]);
    } catch {
      return;
    }
    const docMdp = violations.find((v) => v.restriction === "docMdp");
    const banner: CommentBanner | undefined =
      docMdp !== undefined
        ? { kind: docMdp.permission === 1 ? "p1" : "p2", violations }
        : violations.some((v) => v.restriction === "usageRights")
          ? { kind: "usageRights", violations }
          : undefined;
    if (this.#current) this.#set({ banner });
  }

  #set(change: Partial<CommentState>): void {
    this.#state = { ...this.#state, ...change };
    for (const listener of [...this.#listeners]) listener();
  }
}

/** A selected closed shape as a mark, for the covered-text check. */
function shapeMark(a: PageAnnotation): Mark | undefined {
  const g = a.geometry;
  if (kindOf(a.subtype).patch === "shape") {
    const box = g.kind === "inner" ? g.rect : a.rect;
    return box === undefined ? undefined : { type: "shape", kind: a.subtype === "Square" ? "square" : "circle", box };
  }
  if (a.subtype !== "Polygon" || g.kind !== "vertices") return undefined;
  const vertices: Point[] = [];
  for (let i = 0; i + 1 < g.points.length; i += 2) vertices.push([g.points[i] ?? 0, g.points[i + 1] ?? 0]);
  return { type: "poly", kind: "polygon", vertices };
}
