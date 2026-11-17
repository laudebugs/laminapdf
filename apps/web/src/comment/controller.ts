// Comment mode's state over one open document (Paper C01, C02): the tool on and the style each
// tool remembers on this device, the annotation layer while the mode is current, the selection
// the inspector shows, the banner on a file whose certification forbids comments, and the notice
// when an opaque shape hides text. Tools draw through `DrawHost`: a preview while the pointer
// moves, then one `addAnnot`, its preview gone in the frame the page's new pixels show
// (AnnotationGesture). Each inspector change is one `updateAnnot` per selected annotation.

import type { Op, PageAnnotation, Violation } from "@laminapdf/sdk";
import type { ShellLayout } from "@laminapdf/ui";
import {
  type BorderStyleName,
  type CommentColour,
  type CommentTool,
  type LineEndingName,
  laterMessage,
  type ShapeKind,
  type ToolRegistry,
} from "@laminapdf/ui/editing";
import {
  type AnnotationGesture,
  AnnotationLayer,
  type AnnotationWords,
  type TextSelection,
  type Viewer,
  type ViewerClient,
} from "@laminapdf/viewer";
import { pointOn, selectedLines } from "../links.tsx";
import type { Engine } from "../session.ts";
import {
  ARROW_ENDS,
  addOp,
  borderStyle,
  canResize,
  colourValue,
  coversText,
  type Frame,
  type Mark,
  type MarkStyle,
  markPreview,
  type PageText,
  type Point,
  patchType,
  pdfDate,
  resizeOp,
  snapLines,
} from "./marks.ts";
import { type DrawHost, type MarkTool, markTools, placedMark } from "./tools.ts";

/** The inspector's id in the shell's layout. */
const COMMENT_INSPECTOR = "comment";

/** What a tool remembers: its colour and line width. */
export interface ToolStyle {
  readonly colour: CommentColour;
  readonly width: number;
  /** A shape's fill, set from the inspector; none without. */
  readonly fill?: CommentColour;
}

/** The key a style is remembered under: a tool, or a shape for Shapes. */
export type StyleKey = Exclude<CommentTool, "shape" | "select"> | ShapeKind;

/** What the reader knows of an annotation's drawing that its record doesn't carry, from the edits it made. */
export interface KnownStyle {
  readonly width?: number;
  readonly fill?: CommentColour | "none";
  readonly border?: BorderStyleName;
  readonly ends?: readonly [LineEndingName, LineEndingName];
  readonly smoothing?: boolean;
}

/** The banner on a file whose signature forbids comments (C01). */
interface CommentBanner {
  readonly kind: "p1" | "p2" | "usageRights";
  readonly violations: readonly Violation[];
}

/** Comment mode's state. */
export interface CommentState {
  readonly tool: CommentTool;
  readonly shape: ShapeKind;
  readonly styles: Readonly<Record<StyleKey, ToolStyle>>;
  readonly selection: readonly PageAnnotation[];
  readonly page: number | undefined;
  /** Comments on the page in view, for the toolbar. */
  readonly count: number;
  readonly banner: CommentBanner | undefined;
  /** An opaque shape was drawn over text: the notice that the text stays in the file. */
  readonly covered: boolean;
  /** The first creation tool asks for the name comments carry. */
  readonly askingAuthor: boolean;
  /** Bumped when a note is placed: the inspector focuses its note field. */
  readonly noteFocus: number;
}

/** Each style key's colour family: fill colours for highlights and notes, line colours for the rest. */
export function familyOf(key: StyleKey): "fill" | "line" {
  return key === "highlight" || key === "note" ? "fill" : "line";
}

/** The tools that draw lines, and so take a width. */
export function takesWidth(key: StyleKey): boolean {
  return familyOf(key) === "line" && key !== "underline" && key !== "strike" && key !== "caret";
}

const DEFAULT_STYLES: Readonly<Record<StyleKey, ToolStyle>> = {
  highlight: { colour: "yellow", width: 1 },
  underline: { colour: "green", width: 1 },
  strike: { colour: "red", width: 1 },
  caret: { colour: "blue", width: 1 },
  note: { colour: "yellow", width: 1 },
  ink: { colour: "blue", width: 2 },
  rectangle: { colour: "blue", width: 1.5 },
  ellipse: { colour: "blue", width: 1.5 },
  line: { colour: "black", width: 1.5 },
  arrow: { colour: "red", width: 1.5 },
  polygon: { colour: "blue", width: 1.5 },
  polyline: { colour: "black", width: 1.5 },
};

const STYLES_KEY = "lamina.comment.styles";
const AUTHOR_KEY = "lamina.comment.author";

/** Reads `key` from this device's storage; nothing where storage is blocked. */
function stored(key: string): string | undefined {
  try {
    return localStorage.getItem(key) ?? undefined;
  } catch {
    return undefined;
  }
}

function store(key: string, value: string): void {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Storage is blocked: the choice lasts this visit.
  }
}

/** The styles kept on this device, over the defaults. */
function storedStyles(): Readonly<Record<StyleKey, ToolStyle>> {
  try {
    const kept = JSON.parse(stored(STYLES_KEY) ?? "{}") as Partial<Record<StyleKey, ToolStyle>>;
    return { ...DEFAULT_STYLES, ...kept };
  } catch {
    return DEFAULT_STYLES;
  }
}

/** The style key of `tool` with `shape`. */
export function styleKey(tool: CommentTool, shape: ShapeKind): StyleKey | undefined {
  if (tool === "select") return undefined;
  return tool === "shape" ? shape : tool;
}

/** An annotation's name in words: its kind, then its comment. */
export function kindName(subtype: string): string {
  const id = `kind.${subtype}` as Parameters<typeof laterMessage>[0];
  const name = laterMessage(id);
  return name === id ? laterMessage("kind.other") : name;
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
  #selection: TextSelection | undefined;
  readonly #known = new Map<string, KnownStyle>();
  #current = false;

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
      askingAuthor: false,
      noteFocus: 0,
    };
  }

  getState(): CommentState {
    return this.#state;
  }

  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  /** What the reader knows of an annotation's drawing beyond its record. */
  known(id: string): KnownStyle {
    return this.#known.get(id) ?? {};
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
      resize: {
        can: canResize,
        ops: (page, annotation, resized) => this.#resizeOps(page, annotation, resized),
      },
    });
    this.#layer = layer;
    const host = this.#host();
    this.#marks = markTools(host);
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
    // A page's frame changes only with page setup, which is another mode's.
    this.#frames.clear();
    this.#viewer.element.removeAttribute("data-comment-drawing");
    if (this.#layout.getState().inspector?.id === COMMENT_INSPECTOR) this.#layout.closeInspector();
    this.#set({ tool: "select", selection: [], page: undefined, banner: undefined, covered: false });
  }

  /** Turns `tool` on (with `shape` for Shapes); a text tool with text selected marks it at once. */
  use(tool: CommentTool, shape: ShapeKind = this.#state.shape): void {
    if (!this.#current) return;
    for (const t of this.#marks) t.done();
    const mode = tool === "shape" ? shape : tool;
    this.#tools.use(mode);
    if (tool !== "select") {
      this.#viewer.element.setAttribute("data-comment-drawing", "");
      this.#layer?.select(this.#state.page ?? 0, []);
    } else this.#viewer.element.removeAttribute("data-comment-drawing");
    this.#set({ tool, shape });
    if (tool !== "select" && stored(AUTHOR_KEY) === undefined) this.#set({ askingAuthor: true });
    if ((tool === "highlight" || tool === "underline" || tool === "strike") && this.#selection !== undefined) {
      void this.#markSelection();
    }
  }

  /** The name comments carry, kept on this device; empty leaves it off. */
  setAuthor(name: string | undefined): void {
    if (name !== undefined) store(AUTHOR_KEY, name.trim());
    this.#set({ askingAuthor: false });
  }

  /** Asks for the name comments carry again (the palette's Your name on comments…). */
  askAuthor(): void {
    this.#set({ askingAuthor: true });
  }

  /** The name comments carry now, or undefined for none. */
  author(): string | undefined {
    const name = stored(AUTHOR_KEY);
    return name === undefined || name === "" ? undefined : name;
  }

  /** Sets the colour (`colour`) or width the tool on remembers. */
  setStyle(change: Partial<ToolStyle>): void {
    const key = styleKey(this.#state.tool, this.#state.shape);
    if (key === undefined) return;
    const styles = { ...this.#state.styles, [key]: { ...this.#state.styles[key], ...change } };
    store(STYLES_KEY, JSON.stringify(styles));
    this.#set({ styles });
  }

  /** Dismisses the covered-text notice. */
  dismissCovered(): void {
    this.#set({ covered: false });
  }

  // --- the inspector's changes ---

  /** Applies one `updateAnnot` per selected annotation, each with `args(annotation)`, as one step. */
  #change(args: (a: PageAnnotation) => object | undefined, remember?: (a: PageAnnotation) => KnownStyle): void {
    const selection = this.#state.selection;
    const ops = selection.flatMap((a) => {
      const change = args(a);
      return change === undefined
        ? []
        : [{ op: "updateAnnot", args: { id: a.id, modified: pdfDate(new Date()), ...change } } as unknown as Op];
    });
    if (ops.length === 0) return;
    const [first] = selection;
    const label = laterMessage("comment.step.change", { kind: kindName(first?.subtype ?? "").toLocaleLowerCase() });
    void this.#viewer.edits.apply(ops, label).then(
      () => {
        if (remember === undefined) return;
        for (const a of selection) this.#known.set(a.id, { ...this.known(a.id), ...remember(a) });
        this.#emit();
      },
      () => undefined,
    );
  }

  /** The line colour, or a markup's or note's colour. */
  setColour(colour: CommentColour, family: "fill" | "line"): void {
    this.#change(() => ({ color: [...colourValue(colour, family)] }));
  }

  /** A shape's fill (none, or a fill colour); the tool that draws it remembers it, and opaque fills are checked for text under them. */
  setFill(fill: CommentColour | "none"): void {
    const interior = fill === "none" ? null : [...colourValue(fill, "fill")];
    this.#change(
      (a) => {
        const type = patchType(a.subtype);
        return type === "shape" || (type === "poly" && a.subtype === "Polygon")
          ? { spec: { type, interior } }
          : undefined;
      },
      () => ({ fill }),
    );
    const [first] = this.#state.selection;
    const key = first?.subtype === "Circle" ? "ellipse" : first?.subtype === "Polygon" ? "polygon" : "rectangle";
    const styles = {
      ...this.#state.styles,
      [key]: { ...this.#state.styles[key], ...(fill === "none" ? { fill: undefined } : { fill }) },
    };
    store(STYLES_KEY, JSON.stringify(styles));
    this.#set({ styles });
    if (fill !== "none") void this.#checkSelectionCovers(fill);
  }

  /** A line's width (an ink pen's, or an outline's). */
  setWidth(width: number): void {
    this.#change(
      (a) => {
        const type = patchType(a.subtype);
        if (type === "ink") return { spec: { type, width } };
        if (type === undefined) return undefined;
        return { spec: { type, border: { width, style: borderStyle(this.known(a.id).border ?? "solid") } } };
      },
      () => ({ width }),
    );
  }

  /** The opacity, 0 to 100. */
  setOpacity(percent: number): void {
    this.#change(() => ({ opacity: Math.max(0, Math.min(1, percent / 100)) }));
  }

  /** An outline's style: solid, dashed or cloudy. */
  setBorder(style: BorderStyleName): void {
    this.#change(
      (a) => {
        const type = patchType(a.subtype);
        if (type === undefined || type === "ink") return undefined;
        return { spec: { type, border: { width: this.known(a.id).width ?? 1, style: borderStyle(style) } } };
      },
      () => ({ border: style }),
    );
  }

  /** A line's or polyline's endings. */
  setEnds(ends: readonly [LineEndingName, LineEndingName]): void {
    this.#change(
      (a) => {
        const type = patchType(a.subtype);
        return type === "line" || (type === "poly" && a.subtype === "PolyLine")
          ? { spec: { type, ends: [...ends] } }
          : undefined;
      },
      () => ({ ends }),
    );
  }

  /** Whether ink is smoothed through its points. */
  setSmoothing(smoothing: boolean): void {
    this.#change(
      (a) => (patchType(a.subtype) === "ink" ? { spec: { type: "ink", smoothing } } : undefined),
      () => ({ smoothing }),
    );
  }

  /** The note (`/Contents`); empty removes it. */
  setNote(text: string): void {
    this.#change(() => ({ contents: text === "" ? null : text }));
  }

  /** Deletes the selection, as one step. */
  deleteSelection(): void {
    const selection = this.#state.selection;
    if (selection.length === 0) return;
    const ops = selection.map((a) => ({ op: "deleteAnnot", args: { id: a.id } }) as unknown as Op);
    const [first] = selection;
    const label = laterMessage("comment.step.delete", { kind: kindName(first?.subtype ?? "").toLocaleLowerCase() });
    this.#layer?.select(this.#state.page ?? 0, []);
    void this.#viewer.edits.apply(ops, label).catch(() => undefined);
  }

  // --- internals ---

  #selected(selection: readonly PageAnnotation[]): void {
    const [first] = selection;
    const page = first === undefined ? undefined : this.#pageOf(first);
    this.#set({ selection, page: page ?? this.#state.page });
    if (page !== undefined && !this.#frames.has(page)) {
      void this.#frame(page).then((frame) => {
        if (frame !== undefined) this.#frames.set(page, frame);
      });
    }
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
    void this.#engine
      .view()
      .then(({ client }) => client.pageText(page))
      .then(
        (text) => {
          if (this.#texts.get(page) === "reading") this.#texts.set(page, text);
        },
        () => this.#texts.delete(page),
      );
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
      return undefined;
    }
  }

  /** The style the tool on draws with. */
  #style(): MarkStyle {
    const key = styleKey(this.#state.tool, this.#state.shape) ?? "highlight";
    const style = this.#state.styles[key];
    const family = familyOf(key);
    return {
      colour: colourValue(style.colour, family),
      width: style.width,
      opacity: 1,
      ...(style.fill === undefined ? {} : { fill: colourValue(style.fill, "fill") }),
    };
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

  /** Page `page`'s frame: its crop box, `/Rotate` and `/UserUnit`. */
  async #frame(page: number): Promise<Frame | undefined> {
    const info = this.#engine.getState().pages[page];
    if (info === undefined) return undefined;
    try {
      const boxes = await this.#engine.pageBoxes(page);
      return { crop: boxes.crop, rotate: info.rotation, unit: boxes.userUnit };
    } catch {
      return undefined;
    }
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
    gesture.update(markPreview(mark, style));
    const frame = await this.#frame(page);
    const id = this.#engine.getState().pages[page]?.id;
    if (frame === undefined || id === undefined) {
      await gesture.cancel();
      return;
    }
    const op = addOp(id, mark, frame, style, {
      ...(this.author() === undefined ? {} : { author: this.author() as string }),
      created: new Date(),
    });
    const kind = this.#markKind(mark);
    try {
      const result = await gesture.commit([op], {
        label: laterMessage("comment.step.add", { kind: kind.toLocaleLowerCase() }),
      });
      const created = result.createdAnnotations?.[0];
      if (created !== undefined) this.#remember(created, mark);
      if (mark.type === "note" && created !== undefined) this.#selectMade(page, created, true);
      void this.#checkCovers(page, mark, style);
    } catch {
      // A failed edit is said by the session's failure toast; the gesture showed the page as it was.
    }
  }

  /** Selects the annotation just made once the layer has read it; a note's field takes the focus. */
  #selectMade(page: number, id: string, focusNote: boolean): void {
    const tries = { left: 20 };
    const attempt = () => {
      if (this.#layer?.annotations(page)?.some((a) => a.id === id)) {
        this.#layer.select(page, [id]);
        if (focusNote) this.#set({ noteFocus: this.#state.noteFocus + 1 });
        return;
      }
      if (--tries.left > 0) setTimeout(attempt, 50);
    };
    attempt();
  }

  /** What the reader knows of a new mark's drawing, which its record won't say. */
  #remember(id: string, mark: Mark): void {
    const key = styleKey(this.#state.tool, this.#state.shape);
    const style = key === undefined ? undefined : this.#state.styles[key];
    if (style === undefined) return;
    this.#known.set(id, {
      width: style.width,
      fill: style.fill ?? "none",
      border: "solid",
      ...(mark.type === "line" ? { ends: mark.arrow ? ARROW_ENDS : (["none", "none"] as const) } : {}),
      ...(mark.type === "poly" && mark.kind === "polyLine" ? { ends: ["none", "none"] as const } : {}),
      ...(mark.type === "ink" ? { smoothing: true } : {}),
    });
  }

  #markKind(mark: Mark): string {
    switch (mark.type) {
      case "markup":
        return kindName(
          mark.kind === "highlight" ? "Highlight" : mark.kind === "underline" ? "Underline" : "StrikeOut",
        );
      case "note":
        return kindName("Text");
      case "caret":
        return kindName("Caret");
      case "ink":
        return kindName("Ink");
      case "shape":
        return kindName(mark.kind === "square" ? "Square" : "Circle");
      case "line":
        return kindName("Line");
      case "poly":
        return kindName(mark.kind === "polygon" ? "Polygon" : "PolyLine");
    }
  }

  /** Shows the covered-text notice when `mark` hides text on page `page`. */
  async #checkCovers(page: number, mark: Mark, style: MarkStyle): Promise<void> {
    if (style.fill === undefined || style.opacity < 1) return;
    const text = await this.#textNow(page);
    if (text !== undefined && coversText(mark, style, text)) this.#set({ covered: true });
  }

  /** After a fill is set from the inspector: the notice, if a selected shape now hides text. */
  async #checkSelectionCovers(fill: CommentColour): Promise<void> {
    const page = this.#state.page;
    if (page === undefined) return;
    const style: MarkStyle = { colour: [0, 0, 0], width: 1, opacity: 1, fill: colourValue(fill, "fill") };
    for (const a of this.#state.selection) {
      if ((a.opacity ?? 1) < 1) continue;
      const mark = shapeMark(a);
      if (mark !== undefined) await this.#checkCovers(page, mark, style);
    }
  }

  /** The resize the layer asks for, once the page's frame is known; nothing if it isn't yet. */
  #resizeOps(page: number, annotation: PageAnnotation, resized: PageAnnotation): Op[] {
    // Read when the annotation was selected: a resize needs a selection first.
    const frame = this.#frames.get(page);
    if (frame === undefined) return [];
    const op = resizeOp(annotation, resized, frame);
    return op === undefined ? [] : [op];
  }

  /** Frames read for resizing, by page. */
  readonly #frames = new Map<number, Frame>();

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
      this.deleteSelection();
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
      this.#set({ count: records.filter((a) => !NOT_COMMENTS.has(a.subtype)).length });
    } catch {
      // The page can't be read: the count stays as it was.
    }
  }

  /** Whether a comment would break the file's signature: the banner from the moment the mode opens (C01). */
  async #checkBanner(): Promise<void> {
    const info = this.#engine.getState().pages[0];
    const frame = await this.#frame(0);
    if (info === undefined || frame === undefined) return;
    const probe = addOp(info.id, { type: "note", at: [10, 10] }, frame, this.#style(), { created: new Date(0) });
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
    this.#emit();
  }

  #emit(): void {
    this.#state = { ...this.#state };
    for (const listener of [...this.#listeners]) listener();
  }
}

/** A selected shape as a mark, for the covered-text check. */
function shapeMark(a: PageAnnotation): Mark | undefined {
  const g = a.geometry;
  if (a.subtype === "Square" || a.subtype === "Circle") {
    const box = g.kind === "inner" ? g.rect : a.rect;
    return box === undefined ? undefined : { type: "shape", kind: a.subtype === "Square" ? "square" : "circle", box };
  }
  if (a.subtype === "Polygon" && g.kind === "vertices") {
    const vertices: Point[] = [];
    for (let i = 0; i + 1 < g.points.length; i += 2) vertices.push([g.points[i] ?? 0, g.points[i + 1] ?? 0]);
    return { type: "poly", kind: "polygon", vertices };
  }
  return undefined;
}
