// Comment mode's state over one open document (Paper C01, C02): the tool on and the style each
// tool remembers on this device, the annotation layer while the mode is current, the selection
// the inspector shows, the banner on a file whose certification forbids comments, and the notice
// when an opaque shape hides text. Tools draw through `DrawHost`: a preview while the pointer
// moves, then one `addAnnot`, its preview gone in the frame the page's new pixels show
// (AnnotationGesture). The inspector's changes are changes.ts's; text boxes, typed into in place
// (C03), are text-boxes.ts's.

import type { CommentThread, Op, PageAnnotation, Violation } from "@laminapdf/sdk";
import type { ShellLayout } from "@laminapdf/ui";
import {
  type CommentColour,
  type CommentTool,
  laterMessage,
  type ShapeKind,
  type StatusChoice,
  type TextKind,
  type ToolRegistry,
} from "@laminapdf/ui/editing";
import {
  type AnnotationGesture,
  AnnotationLayer,
  type AnnotationWords,
  type PageFrame,
  pageToUser,
  type TextSelection,
  type Viewer,
  type ViewerClient,
} from "@laminapdf/viewer";
import { storeAuthor, storedAuthor } from "../author.ts";
import { pointOn, selectedLines } from "../page-points.ts";
import type { Engine } from "../session.ts";
import { AnnotationChanges } from "./changes.ts";
import { boxInside, pointsOf, strokesHit } from "./erase.ts";
import { COMMENT_INSPECTOR, COMMENTS_PANEL } from "./inspector-id.ts";
import {
  colourName,
  colourValue,
  coversText,
  type Mark,
  type MarkStyle,
  markPreview,
  type PageText,
  type Point,
  snapLines,
} from "./marks.ts";
import {
  ARROW_ENDS,
  addAnnotOp,
  canResize,
  deleteAnnotOp,
  inkStrokes,
  kindName,
  kindOf,
  resizeOp,
  subtypeOf,
  type ToUser,
  updateAnnotOp,
} from "./ops.ts";
import { PointerRules, ruled } from "./pointer-rules.ts";
import { type FileServices, type FilesState, StampsAndFiles } from "./stamps-and-files.ts";
import {
  markStyle,
  type StyleKey,
  storedStyles,
  storeStyles,
  styleKey,
  type ToolStyle,
  type ToolStyles,
} from "./styles.ts";
import { TextBoxes, type TextState } from "./text-boxes.ts";
import { TextEditor } from "./text-editor.ts";
import { checkedOp, deleteOps, replyOp, statusOps } from "./thread-ops.ts";
import { type MarkTool, markTools, placedMark } from "./tools.ts";
import { type TouchHost, touchTools } from "./touch-tools.ts";

/** The banner on a file whose signature forbids comments (C01). */
interface CommentBanner {
  readonly kind: "p1" | "p2" | "usageRights";
  readonly violations: readonly Violation[];
}

/** A Comments panel control a command focuses. */
export type PanelControl = "search" | "filter" | "sort" | "reply";
/** Comment mode's state. */
export interface CommentState {
  readonly tool: CommentTool;
  readonly shape: ShapeKind;
  /** What Text box ▾ makes: the last text tool used. */
  readonly text: TextKind;
  /** The text box being typed into, or the one selected: what the text inspector shows. */
  readonly textBox: TextState | undefined;
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
  /** The stamp picker's custom stamps, the stamp chosen and the sound playing (C05). */
  readonly files: FilesState;
  /** The program whose saving waits on O14's warning. */
  readonly program: string | undefined;
  /** The stamp picker is open: Stamp ▾ opened it, or the palette's Add a stamp…. */
  readonly picking: boolean;
  /** Whether a finger draws (C09): on until a pen is seen, then the person's choice. */
  readonly drawWithFinger: boolean;
  /** A pen has drawn this session: a tablet shows the tool shelf (C09). */
  readonly penSeen: boolean;
  /** Bumped when a note is placed and selected: the inspector focuses its note field. */
  readonly noteFocus: number;
  /** Whether the selection is the note just placed, which the phone's sheet opens tall for (C08). */
  readonly notePlaced: boolean;
  /** The Comments panel's control a command asked for, and when, so a second ask focuses again. */
  readonly panelFocus: { readonly target: PanelControl; readonly at: number } | undefined;
  /** The threads ticked in the Comments panel for a bulk action. */
  readonly ticked: ReadonlySet<string>;
  /** Whether the Comments panel holds the right dock (C04). */
  readonly panelOpen: boolean;
  /** Whether the selected comment's menu is open (Shift+F10, C02). */
  readonly menu: boolean;
  /** Bumped when the session learns what it wrote of an annotation: the inspector shows it. */
  readonly known: number;
}

/**
 * The layer's words, from the later catalogue. A mark is named by its kind, its border when the
 * session knows it isn't solid, its colour and its note: "Rectangle, cloudy border, blue" (C02);
 * a screen reader adds "selected" from the button's state.
 */
function layerWords(changes: AnnotationChanges): AnnotationWords {
  return {
    label: (a) => {
      if (a.details?.kind === "fileAttachment" && a.details.name !== undefined) {
        return laterMessage("attachment.named", { name: a.details.name });
      }
      const border = changes.known(a.id).border;
      const colour = colourName(a.color, kindOf(a.subtype).family);
      const parts = [
        kindName(a.subtype),
        ...(border === "cloudy" || border === "dashed"
          ? [laterMessage("kind.border", { border: laterMessage(`inspector.border.${border}`) })]
          : []),
        ...(colour === undefined ? [] : [laterMessage(`colour.${colour}`).toLocaleLowerCase()]),
      ].join(", ");
      return a.contents ? laterMessage("kind.labelled", { kind: parts, contents: a.contents }) : parts;
    },
    move: laterMessage("comment.step.move"),
    resize: laterMessage("comment.step.resize"),
  };
}

/** The words of the box a text box is typed into, from the later catalogue. */
const TEXT_WORDS = {
  name: (edit: { readonly kind: TextKind }) => laterMessage(`textbox.name.${edit.kind}`),
  placeholder: laterMessage("textbox.placeholder"),
  start: laterMessage("textbox.handle.start"),
  end: laterMessage("textbox.handle.end"),
  knee: laterMessage("textbox.handle.knee"),
  point: laterMessage("textbox.handle.point"),
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
  #phone = false;
  /** Each page's text once read, by page; dropped when an edit redraws the page. */
  readonly #texts = new Map<number, PageText | "reading">();
  /**
   * Each page's frame once read, until an edit redraws the page: a turn or a crop (the page
   * commands, an undo) changes it, whatever the mode. A read that fails isn't kept.
   */
  readonly #frames = new Map<number, Promise<PageFrame | undefined>>();
  #selection: TextSelection | undefined;
  /** The note just placed, whose field takes the focus once the layer selects it. */
  #focusNote: string | undefined;
  #current = false;
  /** The text boxes, once the mode has been entered. */
  #boxes: TextBoxes | undefined;
  /** The stamps and attached files, once the mode has been entered. */
  #files: StampsAndFiles | undefined;
  readonly #services: FileServices;
  /** The answer to O14's warning, while a program's saving waits on it. */
  #programAnswer: ((save: boolean) => void) | undefined;
  /** The inspector's changes to the selection. */
  readonly changes: AnnotationChanges;

  constructor(engine: Engine, viewer: Viewer, tools: ToolRegistry, layout: ShellLayout, services: FileServices) {
    this.#services = services;
    this.#engine = engine;
    this.#viewer = viewer;
    this.#tools = tools;
    this.#layout = layout;
    this.#state = {
      tool: "select",
      shape: "rectangle",
      text: "textBox",
      textBox: undefined,
      styles: storedStyles(),
      selection: [],
      page: undefined,
      count: 0,
      banner: undefined,
      covered: false,
      checks: 0,
      askingAuthor: false,
      files: { custom: [], chosen: undefined, playing: undefined },
      program: undefined,
      picking: false,
      drawWithFinger: true,
      penSeen: false,
      noteFocus: 0,
      notePlaced: false,
      panelFocus: undefined,
      ticked: new Set(),
      panelOpen: false,
      menu: false,
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
      words: layerWords(this.changes),
      onSelectionChange: (selection) => this.#selected(selection),
      resize: { can: canResize, ops: (page, annotation, resized) => this.#resizeOps(page, annotation, resized) },
    });
    this.#layer = layer;
    const editor = new TextEditor(this.#viewer, this.#textHost(), TEXT_WORDS);
    this.#boxes = new TextBoxes(this.#textBoxHost(), editor);
    this.#files = new StampsAndFiles(this.#filesHost());
    void this.#files.refresh();
    // Every drawing tool behind the pointer rules: palm rejection by pointer type (C09).
    const host = this.#host();
    this.#marks = [...markTools(host), ...touchTools(host)].map((tool) => ruled(tool, this.#rules));
    const unregister = this.#marks.map((tool) => this.#tools.register(tool));
    const element = this.#viewer.element;
    const key = (event: KeyboardEvent) => this.#key(event);
    const twice = (event: MouseEvent) => this.#doubleClick(event);
    element.addEventListener("keydown", key);
    element.addEventListener("dblclick", twice);
    // The document moves into a copy that lays out text while the person picks a tool.
    void this.#boxes.prepare().catch(() => undefined);
    this.#stops = [
      // The Comments toggle shows whether the panel holds the dock, however it was closed.
      this.#layout.subscribe(() => {
        const open = this.panelOpen();
        if (open !== this.#state.panelOpen) this.#set({ panelOpen: open });
      }),
      this.#viewer.addLayer(layer),
      this.#viewer.addLayer(editor),
      () => layer.destroy(),
      () => editor.destroy(),
      ...unregister,
      () => element.removeEventListener("keydown", key),
      () => element.removeEventListener("dblclick", twice),
      this.#viewer.on("pagechange", ({ page }) => {
        this.#text(page);
        void this.#count();
      }),
      this.#viewer.on("invalidate", ({ pages }) => {
        for (const { index } of pages) {
          this.#texts.delete(index);
          this.#frames.delete(index);
        }
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
    this.#boxes?.leave();
    this.#files?.leave();
    this.#files = undefined;
    this.answerProgram(false);
    this.#boxes = undefined;
    for (const tool of this.#marks) tool.cancel();
    if (this.#answered !== undefined) this.setAuthor(undefined);
    this.#tools.use("select");
    for (const stop of this.#stops.splice(0)) stop();
    this.#layer = undefined;
    this.#marks = [];
    this.#frames.clear();
    const docked = this.#layout.getState().inspector?.id;
    if (docked === COMMENT_INSPECTOR || docked === COMMENTS_PANEL) this.#layout.closeInspector();
    this.#set({
      tool: "select",
      selection: [],
      page: undefined,
      banner: undefined,
      covered: false,
      picking: false,
      panelOpen: false,
    });
  }

  /**
   * Turns `tool` on (with `shape` for Shapes, `text` for Text box ▾); a text tool with text
   * selected marks it at once.
   */
  use(tool: CommentTool, shape: ShapeKind = this.#state.shape, text: TextKind = this.#state.text): void {
    if (!this.#current) return;
    for (const t of this.#marks) t.done();
    this.#tools.use(tool === "shape" ? shape : tool === "text" ? text : tool);
    // A tool that draws takes the pointer; the annotations let it through.
    this.#layer?.letPointerThrough(tool !== "select");
    if (tool !== "select") this.#layer?.select(this.#state.page ?? 0, []);
    this.#set({ tool, shape, text });
    if ((tool === "highlight" || tool === "underline" || tool === "strike") && this.#selection !== undefined) {
      void this.#markSelection();
    }
  }

  /**
   * The answer to the name question: a name (empty leaves it off), kept on this device, or
   * `undefined` when the question was closed unanswered. A mark waiting on the answer is made
   * with it, or dropped when there is none.
   */
  setAuthor(name: string | undefined): void {
    if (name !== undefined) storeAuthor(name);
    this.#set({ askingAuthor: false });
    const answered = this.#answered;
    this.#answered = undefined;
    answered?.(name !== undefined);
  }

  /** Whether the person answered the name question, asked the first time a mark is made. */
  #nameAsked(): Promise<boolean> {
    if (storedAuthor() !== undefined) return Promise.resolve(true);
    // A second mark made while the question is open waits on the same answer.
    this.#asking ??= new Promise<boolean>((resolve) => {
      this.#answered = resolve;
      this.#set({ askingAuthor: true });
    }).finally(() => {
      this.#asking = undefined;
    });
    return this.#asking;
  }

  /** The open name question's answer, while a mark waits on it. */
  #asking: Promise<boolean> | undefined;
  #answered: ((answered: boolean) => void) | undefined;

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

  /** Clears the selection: Escape from the inspector deselects, as it does on the page (C02). */
  clearSelection(): void {
    if (this.#state.selection.length > 0) this.#layer?.select(this.#state.page ?? 0, []);
  }

  /** The stamps and attached files: undefined until the mode is entered. */
  get files(): StampsAndFiles | undefined {
    return this.#files;
  }

  /** Opens or closes the stamp picker. */
  pick(open: boolean): void {
    if (this.#state.picking !== open) this.#set({ picking: open });
  }

  /** The answer to O14's warning before saving a program out. */
  answerProgram(save: boolean): void {
    const answer = this.#programAnswer;
    this.#programAnswer = undefined;
    if (this.#state.program !== undefined) this.#set({ program: undefined });
    answer?.(save);
  }

  /** The text boxes' inspector actions: undefined until the mode is entered. */
  get texts(): TextBoxes | undefined {
    return this.#boxes;
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
      notePlaced: focus,
      ...(focus ? { noteFocus: this.#state.noteFocus + 1 } : {}),
    });
    void this.#boxes?.selected(selection);
    this.#inspect();
  }

  /**
   * The inspector opens for a selection or a box being typed into, and closes without either.
   * While the Comments panel holds the dock, it stays: the selection's thread opens in it (C04).
   */
  #inspect(): void {
    if (this.panelOpen()) return;
    if (this.#state.selection.length > 0 || this.#state.textBox !== undefined) {
      this.#layout.openInspector({ id: COMMENT_INSPECTOR, selection: true });
    } else if (this.#layout.getState().inspector?.id === COMMENT_INSPECTOR) this.#layout.closeInspector();
  }

  /** Whether the Comments panel holds the right dock. */
  panelOpen(): boolean {
    return this.#layout.getState().inspector?.id === COMMENTS_PANEL;
  }

  /** Opens the Comments panel in the right dock, or closes it (`comments.toggle-panel`). */
  togglePanel(): void {
    if (this.panelOpen()) {
      this.#layout.closeInspector();
      this.#inspect();
    } else this.showPanel();
  }

  /** Opens the Comments panel in the right dock, where the selected comment's thread opens. */
  showPanel(): void {
    if (!this.panelOpen()) this.#layout.openInspector({ id: COMMENTS_PANEL, selection: false });
  }

  /** The document's comments as threads, as they are now. */
  async threads(): Promise<readonly CommentThread[]> {
    const { client } = await this.#engine.view();
    return client.commentThreads();
  }

  /** Goes to comment `id` on `page`: shows the page and selects the comment (`comment.go-to`). */
  goTo(page: number, id: string): void {
    this.#viewer.goToPage(page);
    this.#layer?.select(page, [id]);
  }

  /**
   * Opens the Comments panel and focuses `target`: its search field, Filter, the sort, or the
   * selected comment's Reply field (`comments.search`, `comments.filter`, `comments.sort`,
   * `comment.reply`).
   */
  focusPanel(target: PanelControl): void {
    this.showPanel();
    this.#set({ panelFocus: { target, at: (this.#state.panelFocus?.at ?? 0) + 1 } });
  }

  /** Ticks or unticks threads `ids` in the Comments panel. */
  tick(ids: readonly string[], on: boolean): void {
    const ticked = new Set(this.#state.ticked);
    for (const id of ids) {
      if (on) ticked.add(id);
      else ticked.delete(id);
    }
    this.#set({ ticked });
  }

  /** The selected comment's ID, which `comment.go-to`, `comment.reply` and `comment.set-status` act on. */
  selectedId(): string | undefined {
    return this.#state.selection[0]?.id;
  }

  /** Closes the selected comment's menu. */
  closeMenu(): void {
    if (this.#state.menu) this.#set({ menu: false });
  }

  /** Shows the selected comment's page again, as `comment.go-to` does from the palette. */
  goToSelected(): void {
    const page = this.#state.page;
    if (page !== undefined) this.#viewer.goToPage(page);
  }

  /** Adds reply `text` to comment `id`, asking for the name first if it hasn't been. */
  async addReply(id: string, text: string): Promise<boolean> {
    if (!(await this.#nameAsked())) return false;
    await this.#viewer.edits.apply([replyOp(id, text, this.author())], laterMessage("comments.reply"));
    return true;
  }

  /** Sets `status` on each of `ids` as one step (`comment.set-status`). */
  async setStatus(ids: readonly string[], status: StatusChoice): Promise<void> {
    if (ids.length === 0 || !(await this.#nameAsked())) return;
    await this.#viewer.edits.apply(statusOps(ids, status, this.author()), laterMessage("comments.status"));
  }

  /** Turns Mark as checked on or off on comment `id`. */
  async setChecked(id: string, checked: boolean): Promise<void> {
    if (!(await this.#nameAsked())) return;
    await this.#viewer.edits.apply([checkedOp(id, checked, this.author())], laterMessage("comments.checked"));
  }

  /** Deletes the threads `ids`, each with its replies, as one step (`comments.delete-selected`). */
  async deleteThreads(ids: readonly string[]): Promise<void> {
    if (ids.length === 0) return;
    this.#set({ ticked: new Set() });
    await this.#viewer.edits.apply(deleteOps(ids), laterMessage("comments.delete.yes", { count: ids.length }));
  }

  /** The page of an annotation the layer selected: the one whose records hold it. */
  #pageOf(annotation: PageAnnotation): number | undefined {
    const pages = this.#engine.getState().pages.length;
    for (let page = 0; page < pages; page++) {
      if (this.#layer?.annotations(page)?.some((a) => a.id === annotation.id)) return page;
    }
    return undefined;
  }

  /** Which pointer draws (C08, C09). Seeing a pen turns Draw with finger off, as a stylus tablet wants. */
  readonly #rules = new PointerRules<PointerEvent>({
    drawWithFinger: () => this.#phone || this.#state.drawWithFinger,
    penSeen: () => this.#set({ penSeen: true, drawWithFinger: false }),
    handOver: (down) => this.#viewer.adoptTouch(down),
  });

  /** Whether the window is a phone's, where a finger always draws (C08). */
  setPhone(phone: boolean): void {
    this.#phone = phone;
  }

  /** Draw with finger (C09). */
  setDrawWithFinger(on: boolean): void {
    if (this.#state.drawWithFinger !== on) this.#set({ drawWithFinger: on });
  }

  /** The tools' host: points from the pointer, the page's text, the layer's gestures. */
  #host(): TouchHost {
    return {
      at: (input) => {
        const at = pointOn(this.#viewer, this.#engine, input);
        return at === undefined ? undefined : { page: at.page, point: [at.x, at.y] };
      },
      text: (page) => this.#text(page),
      whenText: (page) => this.#textNow(page),
      draw: (page, mark) => this.#draw(page, mark),
      finish: (page, mark) => void this.#finish(page, mark),
      inkStroke: (page, mark, into) => this.#inkStroke(page, mark, into),
      eraseAlong: (page, path) => void this.#erase(page, path),
      selectInside: (page, path) => this.#selectInside(page, path),
      later: (run, ms) => {
        const timer = setTimeout(run, ms);
        return () => clearTimeout(timer);
      },
      openText: (page, kind, from, to) => void this.#boxes?.openNew(page, kind, from, to),
      stampSize: () => this.#files?.size(),
      placeStamp: (page, at) => void this.#files?.place(page, at).catch(reportError),
      attachAt: (page, at) => void this.#files?.attachAt(page, at).catch(reportError),
      typing: () => this.#boxes?.typing === true,
    };
  }

  /** The editor's host: the box ends into its edit; the inspector doesn't end it. */
  #textHost() {
    return {
      ended: (...args: Parameters<TextBoxes["ended"]>) => this.#boxes?.ended(...args) ?? Promise.resolve(),
      changed: () => this.#boxes?.refresh(),
      // The inspector's own content, docked or in a phone's bottom sheet; other popovers (a menu, Save) end the box.
      inInspector: (element: Element | null) => element?.closest(".paper-comment-inspector") != null,
    };
  }

  /** What the text boxes need of the mode. */
  #textBoxHost() {
    return {
      engine: this.#engine,
      viewer: this.#viewer,
      layer: () => this.#layer,
      frame: (page: number) => this.#frame(page),
      nameAsked: () => this.#nameAsked(),
      author: () => this.author(),
      colour: () => this.#state.styles.text.colour,
      changed: (textBox: TextState | undefined) => {
        this.#set({ textBox });
        this.#inspect();
      },
      select: (page: number, id: string) => this.#layer?.select(page, [id]),
      focusPage: () => this.#viewer.element.focus({ preventScroll: true }),
    };
  }

  /** What stamps and attached files need of the mode. */
  #filesHost() {
    return {
      engine: this.#engine,
      services: this.#services,
      drop: (page: number) => this.#draw(page, undefined),
      frame: (page: number) => this.#frame(page),
      nameAsked: () => this.#nameAsked(),
      author: () => this.author(),
      commit: (page: number, ops: Op[], label: string) => this.#commit(page, ops, label),
      select: (page: number, id: string) => this.#layer?.select(page, [id]),
      changed: () => this.#set({ files: this.#files?.state ?? this.#state.files }),
      confirmProgram: (name: string) =>
        new Promise<boolean>((resolve) => {
          this.#programAnswer?.(false);
          this.#programAnswer = resolve;
          this.#set({ program: name });
        }),
      attachColour: () => markStyle(this.#state.styles, "attach").colour,
    };
  }

  /**
   * Applies `ops` on `page` as one step through the page's gesture, so a stamp's preview goes in
   * the frame its drawing shows; the annotations it made. A failed edit is said by the session's
   * failure toast.
   */
  async #commit(
    page: number,
    ops: Op[],
    label: string,
    gesture = this.#takeGesture(page) ?? this.#layer?.startGesture(page),
  ): Promise<readonly string[]> {
    try {
      const result =
        gesture === undefined ? await this.#viewer.edits.apply(ops, label) : await gesture.commit(ops, { label });
      return result.createdAnnotations ?? [];
    } catch {
      await gesture?.cancel();
      return [];
    }
  }

  /** A double-click on a text box with Select opens it for typing (C03). */
  #doubleClick(event: MouseEvent): void {
    if (this.#state.tool !== "select") return;
    const id = (event.target as Element | null)?.closest<HTMLElement>(".lamina-annotation")?.dataset.annotationId;
    const annotation = this.#state.selection.find((a) => a.id === id);
    if (annotation?.subtype !== "FreeText" || this.#state.page === undefined) return;
    event.preventDefault();
    void this.#boxes?.openExisting(this.#state.page, annotation);
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

  /** Page `page`'s frame: its crop box, `/Rotate` and `/UserUnit`, read once until an edit redraws the page. */
  #frame(page: number): Promise<PageFrame | undefined> {
    const known = this.#frames.get(page);
    if (known !== undefined) return known;
    const info = this.#engine.getState().pages[page];
    if (info === undefined) return Promise.resolve(undefined);
    const frame = this.#engine.pageBoxes(page).then(
      (boxes): PageFrame => ({ crop: boxes.crop, rotate: info.rotation, unit: boxes.userUnit }),
      () => {
        // A failed read isn't kept: the next mark asks again.
        if (this.#frames.get(page) === frame) this.#frames.delete(page);
        return undefined;
      },
    );
    this.#frames.set(page, frame);
    return frame;
  }

  /** Page `page`'s gesture, which then belongs to the caller: a later preview starts its own. */
  #takeGesture(page: number): AnnotationGesture | undefined {
    const gesture = this.#gestures.get(page);
    this.#gestures.delete(page);
    return gesture;
  }

  /** What `mark` is made with, taken as it is released: its preview's gesture and the tool's style. */
  #drawn(page: number, mark: Mark): Drawn | undefined {
    const gesture = this.#takeGesture(page) ?? this.#layer?.startGesture(page);
    if (gesture === undefined) return undefined;
    // The Highlighter's text highlight is a highlight like any other; its strokes are translucent.
    const style = mark.type === "markup" ? { ...this.#style(), opacity: 1 } : this.#style();
    const key = styleKey(this.#state.tool, this.#state.shape);
    gesture.update(markPreview(mark, style));
    return { gesture, style, kept: key === undefined ? undefined : this.#state.styles[key] };
  }

  /** Makes `mark` on `page` as one step; the ID of the annotation made, when one was. */
  async #finish(page: number, mark: Mark | undefined): Promise<string | undefined> {
    if (mark === undefined) {
      await this.#takeGesture(page)?.cancel();
      return undefined;
    }
    const drawn = this.#drawn(page, mark);
    return drawn === undefined ? undefined : this.#make(page, mark, drawn);
  }

  /** One ink stroke (C08): a new drawing, or once `into` is made, a stroke added to it. */
  async #inkStroke(
    page: number,
    mark: Mark,
    into: Promise<string | undefined> | undefined,
  ): Promise<string | undefined> {
    const drawn = this.#drawn(page, mark);
    if (drawn === undefined) return undefined;
    if (into === undefined) return this.#make(page, mark, drawn);
    const id = await into;
    if (id !== undefined) return this.#extendInk(page, id, mark, drawn);
    // The drawing wasn't made (its name refused, its page gone): its later strokes go with it.
    await drawn.gesture.cancel();
    return undefined;
  }

  /** Makes `mark` on `page` as one step, as `drawn` shows it; the ID of the annotation made. */
  async #make(page: number, mark: Mark, { gesture, style, kept }: Drawn): Promise<string | undefined> {
    // The first mark asks for the name it carries; the preview stays while the person answers.
    if (!(await this.#nameAsked())) {
      await gesture.cancel();
      return undefined;
    }
    const frame = await this.#frame(page);
    const id = this.#engine.getState().pages[page]?.id;
    if (frame === undefined || id === undefined) {
      await gesture.cancel();
      return undefined;
    }
    const op = addAnnotOp(id, mark, mapping(frame), style, { author: this.author(), created: new Date() });
    const label = laterMessage("comment.step.add", { kind: kindName(subtypeOf(mark)).toLocaleLowerCase() });
    let created: string | undefined;
    try {
      const result = await gesture.commit([op], { label });
      created = result.createdAnnotations?.[0];
      if (created !== undefined) {
        this.#remember(created, mark, kept);
        // A note's field takes the focus once the layer has read the note and selected it.
        if (mark.type === "note") {
          this.#focusNote = created;
          this.#layer?.select(page, [created]);
        }
      }
    } catch {
      // A failed edit is said by the session's failure toast; the gesture showed the page as it was.
      return undefined;
    }
    await this.#checkCovers(page, mark, style);
    this.#set({ checks: this.#state.checks + 1 });
    return created;
  }

  /**
   * Erases the ink strokes `path` passes over on `page`, as one step (C08): a drawing loses those
   * strokes, or goes with its last.
   */
  async #erase(page: number, path: readonly Point[]): Promise<void> {
    const ops: Op[] = [];
    for (const a of this.#layer?.annotations(page) ?? []) {
      if (a.geometry.kind !== "ink") continue;
      const strokes = a.geometry.strokes.map(pointsOf);
      const hit = strokesHit(strokes, path);
      if (hit.length === 0) continue;
      ops.push(
        hit.length === strokes.length
          ? deleteAnnotOp(a.id)
          : updateAnnotOp(a.id, { spec: { type: "ink", remove: hit } }),
      );
    }
    if (ops.length === 0) return;
    await this.#commit(page, ops, laterMessage("comment.step.erase"));
  }

  /** Selects the comments on `page` the loop `path` holds (C09's Lasso). */
  #selectInside(page: number, path: readonly Point[]): void {
    const held = (this.#layer?.annotations(page) ?? []).filter(
      (a) => a.rect !== undefined && a.inReplyTo === undefined && boxInside(a.rect, path),
    );
    this.#layer?.select(
      page,
      held.map((a) => a.id),
    );
  }

  /** Adds ink `mark`'s strokes to drawing `id` on `page`, as one step (each stroke one undo, C08). */
  async #extendInk(page: number, id: string, mark: Mark, { gesture, style }: Drawn): Promise<string | undefined> {
    const frame = await this.#frame(page);
    if (frame === undefined || mark.type !== "ink") {
      await gesture.cancel();
      return id;
    }
    const add = inkStrokes(mark, mapping(frame), style.width);
    const label = laterMessage("comment.step.add", { kind: kindName("Ink").toLocaleLowerCase() });
    await this.#commit(page, [updateAnnotOp(id, { spec: { type: "ink", add } })], label, gesture);
    return id;
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
    const op = frame === undefined ? undefined : resizeOp(annotation, resized, mapping(frame));
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
  /** Opens the selected comment's own menu on Shift+F10 or the menu key (C02), in place of the browser's. */
  #menuKey(event: KeyboardEvent): boolean {
    const menu = event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey);
    if (!menu || this.#state.selection.length === 0) return false;
    this.#set({ menu: true });
    return true;
  }

  #key(event: KeyboardEvent): void {
    const target = event.target as HTMLElement | null;
    const typing = target?.closest("input, textarea, select, [contenteditable]") !== null;
    if (typing || event.altKey || event.metaKey || event.ctrlKey) return;
    const tool = this.#state.tool;
    const deleting = event.key === "Delete" || event.key === "Backspace";
    if (this.#menuKey(event)) {
      event.preventDefault();
    } else if (event.key === "Escape" && tool !== "select") {
      event.preventDefault();
      this.#escape(tool);
    } else if (event.key === "Enter" && tool === "select" && this.#openSelected()) {
      event.preventDefault();
    } else if (deleting && tool === "select" && this.#state.selection.length > 0) {
      event.preventDefault();
      this.#layer?.select(this.#state.page ?? 0, []);
      this.changes.delete();
    } else if (event.key === "Enter" && tool !== "select" && target?.closest("button") == null) {
      event.preventDefault();
      this.#place(tool);
    }
  }

  /** Escape with `tool` on: the tool stops and Select is on; after a box was typed into, its selection goes too (C03). */
  #escape(tool: Exclude<CommentTool, "select">): void {
    for (const t of this.#marks) t.cancel();
    this.use("select");
    if (tool === "text") this.clearSelection();
  }

  /** Opens the one selected annotation for typing, when it is a text box (Enter with Select, C03). */
  #openSelected(): boolean {
    const [first, ...rest] = this.#state.selection;
    if (first?.subtype !== "FreeText" || rest.length > 0 || this.#state.page === undefined) return false;
    void this.#boxes?.openExisting(this.#state.page, first);
    return true;
  }

  /** Enter with Stamp or Attach file on: the chosen stamp, or a file asked for, in the middle of the page in view. */
  #placeFile(tool: "stamp" | "attach"): void {
    const centre = this.#centre();
    if (centre === undefined) return;
    const run =
      tool === "stamp"
        ? this.#files?.place(centre.page, centre.point)
        : this.#files?.attachAt(centre.page, centre.point);
    void run?.catch(reportError);
  }

  /** Enter with `tool` on: marks the text selection, or places a mark in the middle of the page in view. */
  #place(tool: Exclude<CommentTool, "select">): void {
    if (tool === "highlight" || tool === "underline" || tool === "strike") {
      void this.#markSelection();
      return;
    }
    if (tool === "stamp" || tool === "attach") {
      this.#placeFile(tool);
      return;
    }
    if (tool === "text") {
      const centre = this.#centre();
      if (centre !== undefined) void this.#boxes?.openNew(centre.page, this.#state.text, centre.point, centre.point);
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
    const probe = addAnnotOp(info.id, { type: "note", at: [10, 10] }, mapping(frame), this.#style(), {
      created: new Date(0),
    });
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

/** What a released mark is made with: the gesture showing its preview, and the style it was drawn in. */
interface Drawn {
  readonly gesture: AnnotationGesture;
  readonly style: MarkStyle;
  /** The tool's style as the session keeps it, to remember what the annotation won't say. */
  readonly kept: ToolStyle | undefined;
}

/** Page space to user space through `frame`, as the viewer maps it. */
function mapping(frame: PageFrame): ToUser {
  return (point) => pageToUser(frame, point);
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
