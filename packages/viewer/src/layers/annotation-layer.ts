// The annotation layer: on every mounted page, in its `lamina-layer-annot` slot, a button over
// each annotation the page shows, in reading order, so Tab reaches each one and a screen reader
// names it. PDFium draws every annotation from its saved appearance; this layer draws only what an
// editing gesture is making, so the page shows what will be saved.
//
// A press selects the annotation under the pointer (hit-tested on its marks, so a press between
// an ink stroke's lines reaches what lies under it), Shift or ⌘/Ctrl adds or removes one, Escape
// clears the selection. The selection gets a 1 px accent outline and eight square handles 8 CSS px
// wide whatever the zoom. Dragging a selected annotation moves the selection: the page is drawn
// without it (`Viewer.hideAnnotations`), an SVG preview follows the pointer, and on release one
// `updateAnnot` per annotation moves it, as one edit; the preview goes in the frame the new page
// shows. The arrow keys move the selection 1 pt, with Shift 10 pt. Locked annotations select but
// never move. `startGesture` opens the same gesture to callers beyond the layer's own moves.
//
// Given `resize`, one selected annotation the app can resize gets its handles live: each takes the
// pointer over 24 CSS px, a drag resizes from it (with Shift keeping the proportions) through the
// same gesture, and Alt+arrows resize from the bottom-right handle, 1 pt or with Shift 10 pt, the
// alternative to dragging (WCAG 2.5.7). The app turns the stretched annotation into its edit,
// since a resize is a change to the subtype's own geometry.
//
// Like the other layers, the buttons sit in page points inside one box that a CSS transform scales
// and turns to the page. The buttons take the pointer, so the layer is for an editing mode: an app
// adds it while one is on and removes it after.

import type { Op, PageAnnotation } from "@laminapdf/sdk";
import type { ViewerEngine } from "../client.js";
import type { PageSize, Rotation } from "../layout/page-layout.js";
import type { EditResult } from "../protocol.js";
import type { PageLayer, Viewer } from "../view/viewer.js";
import {
  annotationPreview,
  type Box,
  handlePoints,
  hitTest,
  isLocked,
  type Move,
  type PreviewShape,
  pageMove,
  resizeBox,
  resizedAnnotation,
  screenBottomRight,
  shownAnnotations,
  unionBox,
  userMove,
} from "./annotation-geometry.js";
import { AnnotationGesture } from "./annotation-gesture.js";
import { placeBox, zoomView } from "./box.js";

/** What the layer needs of its viewer. */
export type AnnotationViewer = Pick<
  Viewer,
  "hideAnnotations" | "whenDrawn" | "applyEdit" | "pageSizes" | "getState" | "element"
>;

/** What the layer needs of the engine: each page's annotations, and the pages' own rotation. */
export type AnnotationEngine = Pick<ViewerEngine, "pageAnnotations" | "document">;

/**
 * The words the layer gives screen readers and the edit history. The viewer has no message
 * catalogue: like the link layer's `LinkWords`, the app passes them in its own language, and the
 * layer uses English without them.
 */
export interface AnnotationWords {
  /** An annotation's accessible name, such as its kind and its comment ("Note: Check this"). */
  label(annotation: PageAnnotation): string;
  /** The history label of a move. */
  readonly move: string;
  /** The history label of a resize. */
  readonly resize: string;
}

/** How the app resizes an annotation: the layer stretches it, the app writes the edit. */
export interface AnnotationResize {
  /** Whether `annotation` can be resized (a text markup or a note can't). */
  can(annotation: PageAnnotation): boolean;
  /**
   * The edit that gives page `page`'s `annotation` the shape of `resized`, which is it stretched;
   * it may resolve later, as when the app reads the page's frame first.
   */
  ops(page: number, annotation: PageAnnotation, resized: PageAnnotation): Op[] | Promise<Op[]>;
}

/** The annotation layer's options. */
export interface AnnotationLayerOptions {
  /** The buttons' names and the moves' history label (default English). */
  readonly words?: AnnotationWords;
  /** The selection changed: the annotations selected now, all on one page, or none. */
  readonly onSelectionChange?: (selection: readonly PageAnnotation[]) => void;
  /** Resizing through the handles and Alt+arrows; without it the handles only show the selection. */
  readonly resize?: AnnotationResize;
}

/** Each subtype's name in English, for annotations without words of the app's. */
const KINDS: Readonly<Record<string, string>> = {
  Highlight: "Highlight",
  Underline: "Underline",
  StrikeOut: "Strikethrough",
  Squiggly: "Squiggly underline",
  Text: "Note",
  FreeText: "Text box",
  Ink: "Ink",
  Line: "Line",
  Square: "Rectangle",
  Circle: "Ellipse",
  Polygon: "Polygon",
  PolyLine: "Polyline",
  Caret: "Insertion mark",
  Stamp: "Stamp",
  FileAttachment: "Attachment",
};

/** English, for a viewer whose app gives none: an annotation's kind, then its comment, if any. */
const ENGLISH: AnnotationWords = {
  label(annotation) {
    const kind = KINDS[annotation.subtype] ?? "Annotation";
    return annotation.contents ? `${kind}: ${annotation.contents}` : kind;
  },
  move: "Move annotation",
  resize: "Resize annotation",
};

/** How far a press moves (CSS px) before it is a drag. */
const DRAG_SLOP = 3;
/** How near a mark a press counts as on it (CSS px). */
const HIT_SLOP = 4;
/** The handles' side and border (CSS px; design language §7.32). */
const HANDLE = 8;
/** The side of the area round each handle that takes the pointer (CSS px; Paper C02). */
const HANDLE_TARGET = 24;
/** Each handle's resize cursor, in `handlePoints`' order. */
const HANDLE_CURSORS = [
  "nwse-resize",
  "nesw-resize",
  "nwse-resize",
  "nesw-resize",
  "ns-resize",
  "ew-resize",
  "ns-resize",
  "ew-resize",
] as const;
/** The least side of an annotation's button (CSS px; WCAG 2.5.8). */
const MIN_TARGET = 24;
const ACCENT = "var(--lamina-annotation-accent, #1a73e8)";
const SVG = "http://www.w3.org/2000/svg";

/** One mounted page. */
interface PageAnnotations {
  readonly slot: HTMLElement;
  readonly box: HTMLDivElement;
  /** The selection's outline and handles. */
  readonly marks: SVGSVGElement;
  /** The areas round the handles that take the pointer, while the selection can be resized. */
  readonly grips: HTMLDivElement;
  readonly abort: AbortController;
  records: readonly PageAnnotation[];
  size: PageSize | undefined;
}

/** A press on an annotation that may become a drag. */
interface Press {
  readonly page: number;
  readonly pointer: number;
  readonly x: number;
  readonly y: number;
  readonly moving: readonly PageAnnotation[];
  gesture: AnnotationGesture | undefined;
  move: Move;
  /** A press on a handle: which one, and the box it resizes. */
  readonly resize?: { readonly handle: number; readonly box: Box };
  /** Whether Shift was held at the last move: a resize keeps its proportions. */
  keep: boolean;
}

/** A box in page points over the page; it takes no pointer events, its buttons do. */
function layerBox(className: string): HTMLDivElement {
  const box = document.createElement("div");
  box.className = className;
  Object.assign(box.style, {
    position: "absolute",
    left: "0",
    top: "0",
    transformOrigin: "0 0",
    pointerEvents: "none",
  });
  return box;
}

/** An SVG over the whole box, in page points. */
function layerSvg(points: PageSize | undefined): SVGSVGElement {
  const svg = document.createElementNS(SVG, "svg");
  Object.assign(svg.style, { position: "absolute", left: "0", top: "0", overflow: "visible", pointerEvents: "none" });
  if (points !== undefined) {
    svg.setAttribute("width", String(points.width));
    svg.setAttribute("height", String(points.height));
  }
  return svg;
}

/** An SVG path element for `shape`. */
function pathOf(shape: PreviewShape): SVGPathElement {
  const path = document.createElementNS(SVG, "path");
  path.setAttribute("d", shape.d);
  path.setAttribute("fill", shape.fill ?? "none");
  path.setAttribute("stroke", shape.stroke ?? "none");
  path.setAttribute("stroke-width", String(shape.width ?? 1));
  path.setAttribute("stroke-linecap", "round");
  path.setAttribute("stroke-linejoin", "round");
  if (shape.opacity !== undefined) path.setAttribute("opacity", String(shape.opacity));
  return path;
}

/** The annotation layer of one viewer. Add it with `viewer.addLayer(layer)`. */
export class AnnotationLayer implements PageLayer {
  readonly name = "annot";
  readonly #viewer: AnnotationViewer;
  readonly #engine: AnnotationEngine;
  readonly #options: AnnotationLayerOptions;
  readonly #pages = new Map<number, PageAnnotations>();
  /** Each page's gesture preview: its own box, kept while the page remounts after an edit. */
  readonly #previews = new Map<number, { readonly box: HTMLDivElement; readonly svg: SVGSVGElement }>();
  /**
   * The records of the page unmounted last, until the task ends: a page mounted again at once (an
   * edit remounts it) gets its buttons from them, so the focus has somewhere to stay until the new
   * records are read.
   */
  #unmounted: { readonly page: number; readonly records: readonly PageAnnotation[] } | undefined;
  /** The selected annotations' IDs, all on `#selectedPage`. */
  #selected = new Set<string>();
  #selectedPage: number | undefined;
  #press: Press | undefined;
  /** Whether the buttons and handles let the pointer through, to a tool that draws (`letPointerThrough`). */
  #through = false;
  /** The annotation whose button takes focus again once its page's buttons are made anew. */
  #refocus: { readonly page: number; readonly id: string } | undefined;
  readonly #listening = new AbortController();

  constructor(viewer: AnnotationViewer, engine: AnnotationEngine, options: AnnotationLayerOptions = {}) {
    this.#viewer = viewer;
    this.#engine = engine;
    this.#options = options;
    const { signal } = this.#listening;
    document.addEventListener("pointermove", (e) => this.#drag(e), { signal });
    document.addEventListener("pointerup", (e) => void this.#release(e), { signal });
    document.addEventListener("pointercancel", (e) => void this.#cancelPress(e.pointerId), { signal });
    document.addEventListener(
      "keydown",
      (e) => {
        if (e.key === "Escape" && this.#press?.gesture !== undefined) {
          e.preventDefault();
          void this.#cancelPress(this.#press.pointer);
        }
      },
      { signal, capture: true },
    );
  }

  /** Stops listening and clears every page: the layer is going. */
  destroy(): void {
    this.#listening.abort();
    for (const [page, view] of [...this.#pages]) this.onUnmount(page, view.slot);
  }

  onMount(page: number, slot: HTMLElement): void {
    const box = layerBox("lamina-annotation-layer");
    const marks = layerSvg(this.#viewer.pageSizes()[page]);
    const grips = document.createElement("div");
    grips.className = "lamina-annotation-grips";
    box.append(marks, grips);
    slot.append(box);
    const view: PageAnnotations = {
      slot,
      box,
      marks,
      grips,
      abort: new AbortController(),
      records: [],
      size: undefined,
    };
    this.#pages.set(page, view);
    const preview = this.#previews.get(page);
    if (preview !== undefined) slot.append(preview.box);
    const known = this.#unmounted;
    this.#unmounted = undefined;
    if (known?.page === page) {
      view.records = known.records;
      this.#render(page, view);
    }
    void this.#load(page, view);
  }

  onUnmount(page: number, _slot: HTMLElement): void {
    const view = this.#pages.get(page);
    if (view === undefined) return;
    view.abort.abort();
    view.box.remove();
    this.#pages.delete(page);
    const unmounted = { page, records: view.records };
    this.#unmounted = unmounted;
    queueMicrotask(() => {
      if (this.#unmounted === unmounted) this.#unmounted = undefined;
    });
    // A gesture's preview outlives its page's buttons: an edit remounts the page before its new
    // pixels are drawn, and the preview stays until they are.
    this.#previews.get(page)?.box.remove();
  }

  onZoom(page: number, _slot: HTMLElement, size: PageSize): void {
    zoomView(this.#pages, page, size, (p, view) => this.#place(p, view));
  }

  /**
   * Lets the pointer through the buttons and handles to what lies under them (`true`), as while a
   * tool draws over the page, or gives it back to them (`false`). The keyboard still reaches them.
   */
  letPointerThrough(through: boolean): void {
    this.#through = through;
    for (const view of this.#pages.values()) {
      for (const el of view.box.querySelectorAll<HTMLElement>(".lamina-annotation, .lamina-annotation-grip")) {
        el.style.pointerEvents = through ? "none" : "auto";
      }
    }
  }

  /** Page `page`'s annotation records as last read, or undefined before they are. */
  annotations(page: number): readonly PageAnnotation[] | undefined {
    return this.#pages.get(page)?.records;
  }

  /** The annotation at (`x`, `y`) of page `page`, in page points: the topmost one it hits. */
  hitTest(page: number, x: number, y: number): PageAnnotation | undefined {
    const view = this.#pages.get(page);
    if (view === undefined) return undefined;
    const shown = shownAnnotations(view.records).map(({ annotation }) => annotation);
    return hitTest(shown, x, y, this.#slop(page));
  }

  /** The selected annotations, all on one page. */
  selection(): readonly PageAnnotation[] {
    const page = this.#selectedPage;
    const records = page === undefined ? [] : (this.#pages.get(page)?.records ?? []);
    return records.filter((a) => this.#selected.has(a.id));
  }

  /** Selects the annotations `ids` on page `page` (none: clears the selection). */
  select(page: number, ids: Iterable<string>): void {
    this.#selected = new Set(ids);
    this.#selectedPage = this.#selected.size === 0 ? undefined : page;
    this.#drawSelection();
    this.#options.onSelectionChange?.(this.selection());
  }

  /**
   * Starts an editing gesture on page `page` that leaves `hide` out of the page's renders while
   * its preview draws them (none hides nothing). Update its preview as the pointer moves, then
   * commit its edit or cancel it.
   */
  startGesture(page: number, hide: readonly PageAnnotation[] = []): AnnotationGesture {
    return new AnnotationGesture(
      {
        hideAnnotations: (p, places) => this.#viewer.hideAnnotations(p, places),
        whenDrawn: (p) => this.#viewer.whenDrawn(p),
        applyEdit: (ops, options) => this.#viewer.applyEdit(ops, options),
        preview: (p, shapes) => this.#preview(p, shapes),
      },
      page,
      hide.map((a) => a.index),
    );
  }

  /**
   * The edit that moves `annotations` (on page `page`) by `move` page points: one `updateAnnot`
   * each, its move in user space, as the op takes it.
   */
  moveOps(page: number, annotations: readonly PageAnnotation[], move: Move): Op[] {
    const rotate = this.#engine.document?.pages[page]?.rotation ?? 0;
    const translate = userMove(move, rotate);
    return annotations.map((a) => ({ op: "updateAnnot", args: { id: a.id, translate } }) as Op);
  }

  /** Moves `annotations` on page `page` by `move` page points, as one edit. */
  moveBy(page: number, annotations: readonly PageAnnotation[], move: Move): Promise<EditResult> {
    return this.#viewer.applyEdit(this.moveOps(page, annotations, move), { label: this.#words().move });
  }

  // --- internals ---

  async #load(page: number, view: PageAnnotations): Promise<void> {
    let records: readonly PageAnnotation[];
    try {
      records = await this.#engine.pageAnnotations(page, { signal: view.abort.signal });
    } catch {
      // Unmounted meanwhile, or the page can't be read: the page has no buttons.
      return;
    }
    if (this.#pages.get(page) !== view) return;
    view.records = records;
    this.#render(page, view);
    if (this.#refocus?.page === page) this.#refocus = undefined;
  }

  #place(page: number, view: PageAnnotations): void {
    const points = this.#viewer.pageSizes()[page];
    if (points === undefined || view.size === undefined) return;
    placeBox(view.box, points, view.size, this.#rotation());
    this.#sizeButtons(page, view);
    const preview = this.#previews.get(page);
    if (preview !== undefined) placeBox(preview.box, points, view.size, this.#rotation());
    this.#drawSelection();
  }

  /**
   * Sizes each button to its annotation, and to at least 24 CSS px a side round its middle at the
   * page's scale now (WCAG 2.5.8): a caret or a small mark is still easy to reach.
   */
  #sizeButtons(page: number, view: PageAnnotations): void {
    const least = MIN_TARGET / this.#scale(page);
    for (const button of view.box.querySelectorAll<HTMLElement>("button.lamina-annotation")) {
      const [x0 = 0, y0 = 0, x1 = 0, y1 = 0] = (button.dataset.box ?? "").split(" ").map(Number);
      const [w, h] = [Math.max(x1 - x0, least), Math.max(y1 - y0, least)];
      const [cx, cy] = [(x0 + x1) / 2, (y0 + y1) / 2];
      Object.assign(button.style, {
        left: `${cx - w / 2}px`,
        top: `${cy - h / 2}px`,
        width: `${w}px`,
        height: `${h}px`,
      });
    }
  }

  #rotation(): Rotation {
    return this.#viewer.getState().rotation as Rotation;
  }

  /** CSS px per point on page `page`, as shown now (1 before it is laid out). */
  #scale(page: number): number {
    const view = this.#pages.get(page);
    const points = this.#viewer.pageSizes()[page];
    if (view?.size === undefined || points === undefined) return 1;
    const turned = this.#rotation() % 180 !== 0;
    return view.size.width / (turned ? points.height : points.width);
  }

  #slop(page: number): number {
    return HIT_SLOP / this.#scale(page);
  }

  /** The buttons of page `page`, one per shown annotation, in reading order. */
  #render(page: number, view: PageAnnotations): void {
    const words = this.#words();
    const buttons = shownAnnotations(view.records).map(({ annotation, box: [x0, y0, x1, y1] }) => {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "lamina-annotation";
      button.dataset.annotationId = annotation.id;
      button.setAttribute("aria-label", words.label(annotation));
      button.setAttribute("aria-pressed", String(this.#selected.has(annotation.id)));
      button.dataset.box = `${x0} ${y0} ${x1} ${y1}`;
      Object.assign(button.style, {
        position: "absolute",
        margin: "0",
        padding: "0",
        border: "0",
        background: "transparent",
        cursor: isLocked(annotation) ? "default" : "move",
        pointerEvents: this.#through ? "none" : "auto",
        touchAction: "none",
      });
      button.addEventListener("pointerdown", (e) => this.#pressOn(e, page, annotation));
      button.addEventListener("keydown", (e) => this.#key(e, page, annotation));
      // A click from the keyboard (Enter, Space) selects; a pointer's press already did.
      button.addEventListener("click", (e) => {
        if (e.detail === 0) this.#choose(page, annotation, e.shiftKey || e.metaKey || e.ctrlKey);
      });
      return button;
    });
    // The focus stays on the annotation it was on: the buttons are made anew.
    const active = document.activeElement;
    const focused =
      active instanceof HTMLElement && view.box.contains(active) ? active.dataset.annotationId : undefined;
    const refocus = this.#refocus?.page === page ? this.#refocus.id : focused;
    view.box.replaceChildren(view.marks, ...buttons, view.grips);
    this.#place(page, view);
    if (refocus !== undefined) {
      buttons.find((b) => b.dataset.annotationId === refocus)?.focus({ preventScroll: true });
    }
    if (this.#selectedPage === page) this.#options.onSelectionChange?.(this.selection());
  }

  /** Selects `annotation`; with `toggle`, adds it to the selection or takes it out. */
  #choose(page: number, annotation: PageAnnotation, toggle: boolean): void {
    if (!toggle || this.#selectedPage !== page) {
      if (this.#selectedPage === page && this.#selected.size === 1 && this.#selected.has(annotation.id)) return;
      this.select(page, [annotation.id]);
      return;
    }
    const next = new Set(this.#selected);
    if (next.has(annotation.id)) next.delete(annotation.id);
    else next.add(annotation.id);
    this.select(page, next);
  }

  /** Draws the selection's outline and handles, and marks the selected buttons. */
  #drawSelection(): void {
    for (const [page, view] of this.#pages) {
      for (const button of view.box.querySelectorAll<HTMLElement>("button.lamina-annotation")) {
        button.setAttribute("aria-pressed", String(this.#selected.has(button.dataset.annotationId ?? "")));
      }
      const box = page === this.#selectedPage ? unionBox(this.selection()) : undefined;
      view.marks.replaceChildren(...(box === undefined ? [] : this.#selectionMarks(page, box)));
      const resizing = box === undefined ? undefined : this.#resizable(page);
      view.grips.replaceChildren(...(box === undefined || resizing === undefined ? [] : this.#grips(page, box)));
    }
  }

  /** The one selected annotation, when it can be resized: not locked, and the app can resize its kind. */
  #resizable(page: number): PageAnnotation | undefined {
    const selection = this.selection();
    const [only] = selection;
    const resize = this.#options.resize;
    if (resize === undefined || this.#selectedPage !== page || selection.length !== 1 || only === undefined)
      return undefined;
    return !isLocked(only) && resize.can(only) ? only : undefined;
  }

  /** The areas 24 CSS px wide round each handle of `box` that start a resize from it. */
  #grips(page: number, box: Box): HTMLElement[] {
    const side = HANDLE_TARGET / this.#scale(page);
    return handlePoints(box).map(([x, y], handle) => {
      const grip = document.createElement("div");
      grip.className = "lamina-annotation-grip";
      grip.dataset.handle = String(handle);
      Object.assign(grip.style, {
        position: "absolute",
        left: `${x - side / 2}px`,
        top: `${y - side / 2}px`,
        width: `${side}px`,
        height: `${side}px`,
        cursor: HANDLE_CURSORS[handle] ?? "move",
        pointerEvents: this.#through ? "none" : "auto",
        touchAction: "none",
      });
      grip.addEventListener("pointerdown", (e) => this.#pressGrip(e, page, handle, box));
      return grip;
    });
  }

  /** A primary press on a handle: a drag from it resizes the selected annotation. */
  #pressGrip(e: PointerEvent, page: number, handle: number, box: Box): void {
    const annotation = this.#resizable(page);
    if (e.button !== 0 || this.#press !== undefined || annotation === undefined) return;
    e.preventDefault();
    e.stopPropagation();
    // The resized annotation's button takes the focus once its page's buttons are made anew, so Alt+arrows go on from there.
    this.#refocus = { page, id: annotation.id };
    this.#press = {
      page,
      pointer: e.pointerId,
      x: e.clientX,
      y: e.clientY,
      moving: [annotation],
      gesture: undefined,
      move: { dx: 0, dy: 0 },
      resize: { handle, box },
      keep: e.shiftKey,
    };
  }

  /** The outline round `box` and its eight handles, sized in CSS px at the page's scale now. */
  #selectionMarks(page: number, box: Box): SVGElement[] {
    const scale = this.#scale(page);
    const [x0, y0, x1, y1] = box;
    const outline = document.createElementNS(SVG, "rect");
    for (const [name, value] of [
      ["x", x0],
      ["y", y0],
      ["width", x1 - x0],
      ["height", y1 - y0],
    ] as const) {
      outline.setAttribute(name, String(value));
    }
    outline.setAttribute("fill", "none");
    outline.setAttribute("stroke", ACCENT);
    outline.setAttribute("stroke-width", "1");
    outline.setAttribute("vector-effect", "non-scaling-stroke");
    outline.setAttribute("class", "lamina-annotation-outline");
    const side = HANDLE / scale;
    const handles = handlePoints(box).map(([x, y]) => {
      const handle = document.createElementNS(SVG, "rect");
      handle.setAttribute("x", String(x - side / 2));
      handle.setAttribute("y", String(y - side / 2));
      handle.setAttribute("width", String(side));
      handle.setAttribute("height", String(side));
      handle.setAttribute("fill", "#fff");
      handle.setAttribute("stroke", ACCENT);
      handle.setAttribute("stroke-width", "1.5");
      handle.setAttribute("vector-effect", "non-scaling-stroke");
      handle.setAttribute("class", "lamina-annotation-handle");
      return handle;
    });
    return [outline, ...handles];
  }

  /** Draws `shapes` as page `page`'s gesture preview, or removes it. */
  #preview(page: number, shapes: readonly PreviewShape[] | undefined): void {
    let preview = this.#previews.get(page);
    if (shapes === undefined) {
      preview?.box.remove();
      this.#previews.delete(page);
      return;
    }
    if (preview === undefined) {
      const box = layerBox("lamina-annotation-preview");
      const svg = layerSvg(this.#viewer.pageSizes()[page]);
      box.append(svg);
      preview = { box, svg };
      this.#previews.set(page, preview);
      const view = this.#pages.get(page);
      if (view !== undefined) {
        view.slot.append(box);
        this.#place(page, view);
      }
    }
    preview.svg.replaceChildren(...shapes.map(pathOf));
  }

  /** The point of page `page` under the viewport position (`clientX`, `clientY`), in page points. */
  #pointOn(page: number, clientX: number, clientY: number): { x: number; y: number } | undefined {
    const view = this.#pages.get(page);
    if (view === undefined) return undefined;
    const rect = view.slot.getBoundingClientRect();
    const scale = this.#scale(page);
    const [u, v] = [clientX - rect.left, clientY - rect.top];
    const [w, h] = [rect.width, rect.height];
    switch (this.#rotation()) {
      case 90:
        return { x: v / scale, y: (w - u) / scale };
      case 180:
        return { x: (w - u) / scale, y: (h - v) / scale };
      case 270:
        return { x: (h - v) / scale, y: u / scale };
      default:
        return { x: u / scale, y: v / scale };
    }
  }

  /** A primary press on `annotation`'s button: selects what it hits, and may start a move. */
  #pressOn(e: PointerEvent, page: number, annotation: PageAnnotation): void {
    if (e.button !== 0 || this.#press !== undefined) return;
    const at = this.#pointOn(page, e.clientX, e.clientY);
    const hit = (at === undefined ? undefined : this.hitTest(page, at.x, at.y)) ?? annotation;
    const toggle = e.shiftKey || e.metaKey || e.ctrlKey;
    // A press on a selected annotation keeps the selection, so the whole of it moves.
    if (toggle || !(this.#selectedPage === page && this.#selected.has(hit.id))) this.#choose(page, hit, toggle);
    e.preventDefault();
    (e.currentTarget as HTMLElement).focus({ preventScroll: true });
    const moving = this.selection().filter((a) => !isLocked(a));
    if (toggle || moving.length === 0) return;
    this.#press = {
      page,
      pointer: e.pointerId,
      x: e.clientX,
      y: e.clientY,
      moving,
      gesture: undefined,
      move: { dx: 0, dy: 0 },
      keep: false,
    };
  }

  /** The pointer moved during a press: past the slop, the selection follows it. */
  #drag(e: PointerEvent): void {
    const press = this.#press;
    if (press === undefined || e.pointerId !== press.pointer) return;
    const [sx, sy] = [e.clientX - press.x, e.clientY - press.y];
    if (press.gesture === undefined) {
      if (Math.hypot(sx, sy) < DRAG_SLOP) return;
      press.gesture = this.startGesture(press.page, press.moving);
    }
    e.preventDefault();
    press.move = pageMove(sx, sy, this.#scale(press.page), this.#rotation());
    press.keep = e.shiftKey;
    press.gesture.update(
      press.resize === undefined
        ? this.#movePreview(press.page, press.moving, press.move)
        : this.#resizePreview(
            press.page,
            press.moving,
            resizeBox(press.resize.box, press.resize.handle, press.move, press.keep),
          ),
    );
  }

  /** What a resize draws: each annotation stretched onto `to`, and the outline round it. */
  #resizePreview(page: number, annotations: readonly PageAnnotation[], to: Box): PreviewShape[] {
    const from = unionBox(annotations);
    if (from === undefined) return [];
    const shapes = annotations.flatMap((a) => annotationPreview(resizedAnnotation(a, from, to), { dx: 0, dy: 0 }));
    const [x0, y0, x1, y1] = to;
    shapes.push({ d: `M${x0} ${y0} H${x1} V${y1} H${x0} Z`, stroke: "#1a73e8", width: 1 / this.#scale(page) });
    return shapes;
  }

  /** The edit that resizes `annotation` on page `page` from its box onto `to`, if the app can. */
  async #resizeOps(page: number, annotation: PageAnnotation, to: Box): Promise<Op[] | undefined> {
    const from = unionBox([annotation]);
    const resize = this.#options.resize;
    if (from === undefined || resize === undefined) return undefined;
    return resize.ops(page, annotation, resizedAnnotation(annotation, from, to));
  }

  /** What a move draws: each annotation at its new place and the selection's outline round them. */
  #movePreview(page: number, moving: readonly PageAnnotation[], move: Move): PreviewShape[] {
    const shapes = moving.flatMap((a) => annotationPreview(a, move));
    const box = unionBox(moving, move);
    if (box !== undefined) {
      const [x0, y0, x1, y1] = box;
      const width = 1 / this.#scale(page);
      shapes.push({ d: `M${x0} ${y0} H${x1} V${y1} H${x0} Z`, stroke: "#1a73e8", width });
    }
    return shapes;
  }

  /** The pointer was released: a drag moves the selection, a press alone has selected. */
  async #release(e: PointerEvent): Promise<void> {
    const press = this.#press;
    if (press === undefined || e.pointerId !== press.pointer) return;
    this.#press = undefined;
    const { gesture, move, page, moving } = press;
    if (gesture === undefined) {
      if (press.resize !== undefined) this.#refocus = undefined;
      return;
    }
    if (move.dx === 0 && move.dy === 0) {
      await gesture.cancel();
      return;
    }
    // An edit since the press (another tool's, an undo) may have changed the page's `/Annots`:
    // the places the gesture hides, and the IDs the move names, must still be the same annotations.
    if (!(await this.#samePlaces(page, moving))) {
      await gesture.cancel();
      return;
    }
    const [only] = moving;
    const ops =
      press.resize === undefined || only === undefined
        ? this.moveOps(page, moving, move)
        : await this.#resizeOps(page, only, resizeBox(press.resize.box, press.resize.handle, move, press.keep));
    if (ops === undefined || ops.length === 0) {
      await gesture.cancel();
      return;
    }
    const label = press.resize === undefined ? this.#words().move : this.#words().resize;
    try {
      await gesture.commit(ops, { label });
    } catch {
      // The edit failed: the gesture showed the annotations where they were.
    }
  }

  /** Whether each of `annotations` is still at its place in page `page`'s `/Annots`, as read now. */
  async #samePlaces(page: number, annotations: readonly PageAnnotation[]): Promise<boolean> {
    let now: readonly PageAnnotation[];
    try {
      now = await this.#engine.pageAnnotations(page);
    } catch {
      return false;
    }
    return annotations.every((a) => now[a.index]?.id === a.id);
  }

  #words(): AnnotationWords {
    return this.#options.words ?? ENGLISH;
  }

  async #cancelPress(pointer: number): Promise<void> {
    const press = this.#press;
    if (press === undefined || press.pointer !== pointer) return;
    this.#press = undefined;
    await press.gesture?.cancel();
  }

  /** The keyboard on an annotation's button: the arrows move the selection, Escape clears it. */
  #key(e: KeyboardEvent, page: number, annotation: PageAnnotation): void {
    if (e.key === "Escape" && this.#selected.size > 0) {
      e.preventDefault();
      this.select(page, []);
      return;
    }
    const arrows: Readonly<Record<string, readonly [number, number]>> = {
      ArrowLeft: [-1, 0],
      ArrowRight: [1, 0],
      ArrowUp: [0, -1],
      ArrowDown: [0, 1],
    };
    const arrow = arrows[e.key];
    if (arrow === undefined || e.metaKey || e.ctrlKey) return;
    if (e.altKey) {
      this.#keyResize(e, page, annotation, arrow);
      return;
    }
    e.preventDefault();
    if (!(this.#selectedPage === page && this.#selected.has(annotation.id))) this.select(page, [annotation.id]);
    const moving = this.selection().filter((a) => !isLocked(a));
    if (moving.length === 0) return;
    // The arrows move on screen; the step is in points of the page, whatever the zoom.
    const step = e.shiftKey ? 10 : 1;
    const move = pageMove(arrow[0] * step, arrow[1] * step, 1, this.#rotation());
    this.#refocus = { page, id: annotation.id };
    this.moveBy(page, moving, move).catch(() => {
      // The edit failed: nothing moved, and the button keeps its focus.
      this.#refocus = undefined;
    });
  }

  /** Alt+arrows: resizes the one selected annotation from its bottom-right handle, 1 pt or with Shift 10 pt. */
  #keyResize(e: KeyboardEvent, page: number, annotation: PageAnnotation, arrow: readonly [number, number]): void {
    if (!(this.#selectedPage === page && this.#selected.has(annotation.id))) this.select(page, [annotation.id]);
    const resizing = this.#resizable(page);
    const box = unionBox(this.selection());
    if (resizing === undefined || box === undefined) return;
    e.preventDefault();
    const step = e.shiftKey ? 10 : 1;
    const move = pageMove(arrow[0] * step, arrow[1] * step, 1, this.#rotation());
    this.#refocus = { page, id: annotation.id };
    void this.#resizeOps(page, resizing, resizeBox(box, screenBottomRight(this.#rotation()), move))
      .then((ops) =>
        ops === undefined || ops.length === 0
          ? undefined
          : this.#viewer.applyEdit(ops, { label: this.#words().resize }),
      )
      .catch(() => undefined)
      .then((result) => {
        // Nothing resized: the button keeps its focus, and no new buttons take it.
        if (result === undefined) this.#refocus = undefined;
      });
  }
}
