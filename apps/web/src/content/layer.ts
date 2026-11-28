// Edit mode's marks over each page (Paper A01, A02): the selected object's frame with its eight
// handles and the turning handle, the dashed snap guides while a drag is near one, the outline of
// what a tool is drawing, and the box text is typed into. It is a viewer layer (`PageLayer`), a
// DOM overlay above the page, so a drag moves only the overlay and the page redraws once, when
// the edit is made. Handles are marks for the pointer (`aria-hidden`): every handle's
// action also has an inspector field and a key (WCAG 2.5.7). Positions are fractions of the page
// as shown, so they follow the zoom and the view's turn with no work.

import { laterMessage } from "@laminapdf/ui/editing";
import type { Viewer } from "@laminapdf/viewer";
import {
  type Box,
  corners,
  type Guide,
  HANDLES,
  type HandleName,
  handlePoint,
  type Matrix,
  type Point,
  turnPoint,
} from "./geometry.ts";
import { lineWidth } from "./text-measure.ts";
import "./layer.css";

/** What the layer asks of Edit mode. */
export interface LayerHost {
  /** Page `page`'s size in points as shown (before the view's turn), once known. */
  pageSize(page: number): { readonly width: number; readonly height: number } | undefined;
  /** A pointer went down on a handle (`turn` for the turning handle) of the selection. */
  onHandle(handle: HandleName | "turn", event: PointerEvent): void;
  /** The text being typed changed. */
  onType(text: string): void;
  /** Typing ends: Escape, a click away, or a tool change. */
  onTypeEnd(why: "escape" | "away", next: Element | null): void;
  /** The text typed into a line of the page's own text changed. */
  onRunType(text: string): void;
  /** A key that ends or moves typing in a line of the page's own text. */
  onRunKey(key: "enter" | "escape" | "tab" | "shift-tab"): void;
  /** The line of the page's own text lost the focus. */
  onRunBlur(next: Element | null): void;
}

/** What is outlined while a tool draws. */
export type Draft =
  | { readonly page: number; readonly kind: "box"; readonly box: Box }
  | { readonly page: number; readonly kind: "ellipse"; readonly box: Box }
  | { readonly page: number; readonly kind: "line"; readonly from: Point; readonly to: Point }
  | { readonly page: number; readonly kind: "polygon"; readonly points: readonly Point[]; readonly closed: boolean };

/** The box text is typed into. */
export interface EditorView {
  readonly page: number;
  /** In page space: its left, top and width; the height follows the text. */
  readonly rect: Box;
  readonly text: string;
  readonly label: string;
  /** CSS for the text, at 1 point = 1 unit. */
  readonly style: {
    readonly family: string;
    readonly size: number;
    readonly color: string;
    readonly bold: boolean;
    readonly italic: boolean;
    readonly align: "left" | "center" | "right";
  };
}

/** The box a line of the page's own text is typed into: over the line, in its look. */
export interface RunEditorView {
  readonly page: number;
  /** The line's box in page space. */
  readonly box: Box;
  readonly text: string;
  /** Bumped when the text is set from outside the box, so the box shows it. */
  readonly version: number;
  readonly label: string;
  readonly family: string;
  /** In points. */
  readonly size: number;
  readonly bold: boolean;
  readonly italic: boolean;
}

/** What Edit text draws: outlines of the lines that can be edited, the pick and the box typed into. */
export interface TextView {
  /** Every line of text that can be edited, by page: dashed outlines. */
  readonly outlines: ReadonlyMap<number, readonly Box[]>;
  /** The picked lines. */
  readonly picked: readonly { readonly page: number; readonly box: Box }[];
  /** The line under the pointer. */
  readonly hover?: { readonly page: number; readonly box: Box };
  readonly editor?: RunEditorView;
}

/** What the layer draws. */
export interface LayerView {
  readonly selection?: {
    readonly page: number;
    readonly frame: Matrix;
    readonly handles: readonly HandleName[];
    readonly turn: boolean;
    /** Locked: the frame shows but there are no handles. */
    readonly locked: boolean;
  };
  readonly guides?: { readonly page: number; readonly guides: readonly Guide[] };
  /** The box of the object under the pointer: a 1 px outline, no handles. */
  readonly hover?: { readonly page: number; readonly box: Box };
  readonly draft?: Draft;
  readonly editor?: EditorView;
  readonly text?: TextView;
}

/** One page's overlay. */
interface Slot {
  readonly root: HTMLElement;
  readonly svg: SVGSVGElement;
  readonly handles: Map<HandleName | "turn", HTMLElement>;
  readonly editor: HTMLTextAreaElement;
  /** The box a line of the page's own text is typed into. */
  readonly runEditor: HTMLInputElement;
  readonly labels: HTMLElement;
  /** The version of the text the run editor last showed. */
  shown: number;
  width: number;
  height: number;
}

const SVG = "http://www.w3.org/2000/svg";

/** A point of the page (page space) as a fraction of the page as shown, in a view turned by `rotation`. */
function shownFraction(point: Point, size: { width: number; height: number }, rotation: number): Point {
  const x = point.x / size.width;
  const y = point.y / size.height;
  switch (rotation) {
    case 90:
      return { x: 1 - y, y: x };
    case 180:
      return { x: 1 - x, y: 1 - y };
    case 270:
      return { x: y, y: 1 - x };
    default:
      return { x, y };
  }
}

/** The layer's DOM, one overlay per mounted page. */
export class ContentLayer {
  readonly name = "content";
  readonly #viewer: Viewer;
  readonly #host: LayerHost;
  readonly #slots = new Map<number, Slot>();
  #view: LayerView = {};
  #stop: (() => void) | undefined;

  constructor(viewer: Viewer, host: LayerHost) {
    this.#viewer = viewer;
    this.#host = host;
  }

  /** Mounts the layer in the viewer. */
  attach(): void {
    this.#stop = this.#viewer.addLayer(this);
  }

  /** Takes it out again. */
  detach(): void {
    this.#stop?.();
    this.#stop = undefined;
    this.#slots.clear();
  }

  /** The page's overlay element, to measure it against the pointer. */
  rectOf(page: number): DOMRect | undefined {
    return this.#slots.get(page)?.root.getBoundingClientRect();
  }

  /** The text being typed, as it stands in the box. */
  typed(): string {
    for (const slot of this.#slots.values()) if (slot.root.dataset.typing === "") return slot.editor.value;
    return "";
  }

  /** The box text is typed into measured, in page points: its content's height. */
  typedHeight(page: number): number {
    const slot = this.#slots.get(page);
    const size = this.#host.pageSize(page);
    if (slot === undefined || size === undefined) return 0;
    const scale = this.#scale(slot, size);
    return scale === 0 ? 0 : slot.editor.scrollHeight / scale;
  }

  /** Puts the caret in the box. */
  focusEditor(): void {
    for (const slot of this.#slots.values()) {
      if (slot.root.dataset.typing === "") {
        slot.editor.focus({ preventScroll: true });
        const end = slot.editor.value.length;
        slot.editor.setSelectionRange(end, end);
      }
    }
  }

  /** Puts `text` into the box at the caret. */
  insertAtCaret(text: string): void {
    for (const slot of this.#slots.values()) {
      if (slot.root.dataset.typing !== "") continue;
      const e = slot.editor;
      const [from, to] = [e.selectionStart, e.selectionEnd];
      e.setRangeText(text, from, to, "end");
      e.dispatchEvent(new Event("input", { bubbles: true }));
      e.focus({ preventScroll: true });
    }
  }

  /** Draws `view`. */
  show(view: LayerView): void {
    this.#view = view;
    for (const [page, slot] of this.#slots) this.#draw(page, slot);
  }

  onMount(page: number, element: HTMLElement): void {
    const root = document.createElement("div");
    root.className = "lamina-content";
    const svg = document.createElementNS(SVG, "svg");
    svg.setAttribute("class", "lamina-content-svg");
    svg.setAttribute("viewBox", "0 0 1 1");
    svg.setAttribute("preserveAspectRatio", "none");
    svg.setAttribute("aria-hidden", "true");
    const labels = document.createElement("div");
    labels.className = "lamina-content-labels";
    labels.setAttribute("aria-hidden", "true");
    const editor = document.createElement("textarea");
    editor.className = "lamina-content-editor";
    editor.spellcheck = true;
    editor.rows = 1;
    editor.addEventListener("input", () => {
      this.#host.onType(editor.value);
      this.#fit(slot);
    });
    editor.addEventListener("keydown", (event) => {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        this.#host.onTypeEnd("escape", null);
      }
    });
    editor.addEventListener("blur", (event) => this.#host.onTypeEnd("away", event.relatedTarget as Element | null));
    const handles = new Map<HandleName | "turn", HTMLElement>();
    root.append(svg, labels);
    for (const name of [...HANDLES, "turn" as const]) {
      const handle = document.createElement("div");
      handle.className = name === "turn" ? "lamina-content-handle lamina-content-turn" : "lamina-content-handle";
      handle.dataset.contentHandle = name;
      handle.setAttribute("aria-hidden", "true");
      handle.hidden = true;
      handle.addEventListener("pointerdown", (event) => {
        if (event.button !== 0) return;
        event.preventDefault();
        event.stopPropagation();
        this.#host.onHandle(name, event);
      });
      handles.set(name, handle);
      root.append(handle);
    }
    const runEditor = this.#makeRunEditor();
    root.append(editor, runEditor);
    element.append(root);
    const slot: Slot = { root, svg, handles, editor, runEditor, labels, shown: -1, width: 0, height: 0 };
    this.#slots.set(page, slot);
    this.#draw(page, slot);
  }

  /** The one-line box for typing into the page's own text; its keys end or move the typing. */
  #makeRunEditor(): HTMLInputElement {
    const input = document.createElement("input");
    input.type = "text";
    input.className = "lamina-content-editor lamina-content-run-editor";
    input.spellcheck = false;
    input.autocomplete = "off";
    // The notice in the inspector (a substitution, text that can't be saved) describes the box.
    input.setAttribute("aria-describedby", "lamina-text-notice");
    input.addEventListener("input", () => {
      this.#host.onRunType(input.value);
      this.#fitRun(input);
    });
    input.addEventListener("keydown", (event) => {
      const key = runKeyOf(event);
      if (key === undefined || event.isComposing) return;
      event.preventDefault();
      event.stopPropagation();
      this.#host.onRunKey(key);
    });
    input.addEventListener("blur", (event) => this.#host.onRunBlur(event.relatedTarget as Element | null));
    input.hidden = true;
    return input;
  }

  /** Widens the box to its text, never narrower than the line it covers. */
  #fitRun(input: HTMLInputElement): void {
    const least = Number(input.dataset.least ?? 0);
    const size = Number.parseFloat(input.style.fontSize);
    const wide = lineWidth(
      input.value,
      size,
      input.style.fontFamily,
      input.style.fontWeight === "700",
      input.style.fontStyle === "italic",
    );
    input.style.inlineSize = `${Math.max(least, Math.ceil(wide) + 6)}px`;
  }

  /** Puts the caret in the box typed into a line of the page's own text. */
  focusRunEditor(): void {
    for (const slot of this.#slots.values()) {
      if (slot.runEditor.hidden) continue;
      slot.runEditor.focus({ preventScroll: true });
      slot.runEditor.select();
    }
  }

  onUnmount(page: number): void {
    this.#slots.get(page)?.root.remove();
    this.#slots.delete(page);
  }

  onZoom(page: number): void {
    const slot = this.#slots.get(page);
    if (slot !== undefined) this.#draw(page, slot);
  }

  /** CSS px per point on `slot`'s page. */
  #scale(slot: Slot, size: { width: number; height: number }): number {
    const shownWidth = this.#viewer.getState().rotation % 180 === 0 ? size.width : size.height;
    return shownWidth === 0 ? 0 : slot.root.clientWidth / shownWidth;
  }

  /** Grows the box to the text typed in it, up to the bottom of the page. */
  #fit(slot: Slot): void {
    slot.editor.style.blockSize = "auto";
    slot.editor.style.blockSize = `${slot.editor.scrollHeight}px`;
  }

  #draw(page: number, slot: Slot): void {
    const size = this.#host.pageSize(page);
    const view = this.#view;
    const rotation = this.#viewer.getState().rotation;
    slot.svg.replaceChildren();
    slot.labels.replaceChildren();
    for (const handle of slot.handles.values()) handle.hidden = true;
    delete slot.root.dataset.typing;
    if (size === undefined) return;
    const at = (p: Point) => shownFraction(p, size, rotation);
    const selection = view.selection?.page === page ? view.selection : undefined;
    if (selection !== undefined) this.#drawSelection(slot, selection, size, at);
    if (view.hover?.page === page) this.#drawHover(slot, view.hover.box, at);
    if (view.guides?.page === page) this.#drawGuides(slot, view.guides.guides, size, at);
    const draft = view.draft?.page === page ? view.draft : undefined;
    if (draft !== undefined) this.#drawDraft(slot, draft, at);
    const editor = view.editor?.page === page ? view.editor : undefined;
    if (editor !== undefined) this.#place(slot, editor, size, at);
    else slot.editor.hidden = true;
    this.#drawText(slot, view.text, page, size, at);
  }

  /** Edit text's marks on one page: the dashed outlines, the picked lines and the box typed into. */
  #drawText(
    slot: Slot,
    text: TextView | undefined,
    page: number,
    size: { width: number; height: number },
    at: (p: Point) => Point,
  ): void {
    const outlines = text?.outlines.get(page) ?? [];
    if (outlines.length > 0) {
      const d = outlines.map((box) => `M${quad(box, at)}Z`).join("");
      slot.svg.append(shape("path", { d, class: "lamina-content-run-outline" }));
    }
    if (text?.hover?.page === page) this.#drawHover(slot, text.hover.box, at);
    for (const pick of text?.picked ?? []) {
      if (pick.page !== page) continue;
      slot.svg.append(shape("polygon", { points: quad(pick.box, at, " ", ","), class: "lamina-content-frame" }));
    }
    const editor = text?.editor?.page === page ? text.editor : undefined;
    if (editor === undefined) slot.runEditor.hidden = true;
    else this.#placeRun(slot, editor, size, at);
  }

  /** Shows the box a line is typed into: over the line, at its size at this zoom, in its look. */
  #placeRun(slot: Slot, editor: RunEditorView, size: { width: number; height: number }, at: (p: Point) => Point): void {
    const [x0, y0, x1, y1] = editor.box;
    const origin = at({ x: x0, y: y0 });
    const e = slot.runEditor;
    const scale = this.#scale(slot, size);
    const turn = this.#viewer.getState().rotation;
    e.hidden = false;
    e.style.transformOrigin = "0 0";
    e.style.transform = turn === 0 ? "" : `rotate(${turn}deg)`;
    e.style.left = `${origin.x * 100}%`;
    e.style.top = `${origin.y * 100}%`;
    e.style.blockSize = `${Math.max((y1 - y0) * scale, 4)}px`;
    e.style.lineHeight = `${Math.max((y1 - y0) * scale - 2, 2)}px`;
    e.style.fontFamily = editor.family;
    e.style.fontSize = `${editor.size * scale}px`;
    e.style.fontWeight = editor.bold ? "700" : "400";
    e.style.fontStyle = editor.italic ? "italic" : "normal";
    e.dataset.least = String((x1 - x0) * scale);
    e.setAttribute("aria-label", editor.label);
    // The box shows the text the controller holds when it is set from outside (Revert) or is new.
    if (slot.shown !== editor.version || (e.value !== editor.text && document.activeElement !== e)) {
      e.value = editor.text;
      slot.shown = editor.version;
    }
    this.#fitRun(e);
  }

  /** The selection's frame, its handles and the turning handle on its stem. */
  #drawSelection(
    slot: Slot,
    selection: NonNullable<LayerView["selection"]>,
    size: { width: number; height: number },
    at: (p: Point) => Point,
  ): void {
    const points = corners(selection.frame).map(at);
    slot.svg.append(
      shape("polygon", { points: points.map((p) => `${p.x},${p.y}`).join(" "), class: "lamina-content-frame" }),
    );
    if (selection.locked) return;
    for (const name of selection.handles) {
      const handle = slot.handles.get(name);
      if (handle !== undefined) place(handle, at(handlePoint(selection.frame, name)));
    }
    const turn = slot.handles.get("turn");
    if (!selection.turn || turn === undefined) return;
    // The turning handle sits 24 points beyond the top edge, which is 24 CSS px at 100 %.
    const beyond = turnPoint(selection.frame, 24 / (this.#scale(slot, size) || 1));
    const from = at(handlePoint(selection.frame, "n"));
    const to = at(beyond);
    slot.svg.append(
      shape("line", {
        x1: String(from.x),
        y1: String(from.y),
        x2: String(to.x),
        y2: String(to.y),
        class: "lamina-content-stem",
      }),
    );
    place(turn, to);
  }

  /** The outline of the object under the pointer. */
  #drawHover(slot: Slot, [x0, y0, x1, y1]: Box, at: (p: Point) => Point): void {
    const points = [
      { x: x0, y: y0 },
      { x: x1, y: y0 },
      { x: x1, y: y1 },
      { x: x0, y: y1 },
    ].map(at);
    slot.svg.append(
      shape("polygon", { points: points.map((p) => `${p.x},${p.y}`).join(" "), class: "lamina-content-hover" }),
    );
  }

  /** The snap guides a drag is on, each with its label in the margin. */
  #drawGuides(
    slot: Slot,
    guides: readonly Guide[],
    size: { width: number; height: number },
    at: (p: Point) => Point,
  ): void {
    for (const guide of guides) {
      const across = guide.axis === "x";
      const from = at(across ? { x: guide.at, y: 0 } : { x: 0, y: guide.at });
      const to = at(across ? { x: guide.at, y: size.height } : { x: size.width, y: guide.at });
      slot.svg.append(
        shape("line", {
          x1: String(from.x),
          y1: String(from.y),
          x2: String(to.x),
          y2: String(to.y),
          class: "lamina-content-guide",
        }),
      );
      const label = document.createElement("span");
      label.className = "lamina-content-guide-label";
      label.textContent = laterMessage(`content.guide.${guide.kind}`);
      place(label, from);
      slot.labels.append(label);
    }
  }

  #drawDraft(slot: Slot, draft: Draft, at: (p: Point) => Point): void {
    const attrs = { class: "lamina-content-draft" };
    switch (draft.kind) {
      case "box": {
        const [x0, y0, x1, y1] = draft.box;
        const points = [
          { x: x0, y: y0 },
          { x: x1, y: y0 },
          { x: x1, y: y1 },
          { x: x0, y: y1 },
        ].map(at);
        slot.svg.append(shape("polygon", { ...attrs, points: points.map((p) => `${p.x},${p.y}`).join(" ") }));
        break;
      }
      case "ellipse": {
        const [x0, y0, x1, y1] = draft.box;
        const a = at({ x: x0, y: y0 });
        const b = at({ x: x1, y: y1 });
        slot.svg.append(
          shape("ellipse", {
            ...attrs,
            cx: String((a.x + b.x) / 2),
            cy: String((a.y + b.y) / 2),
            rx: String(Math.abs(b.x - a.x) / 2),
            ry: String(Math.abs(b.y - a.y) / 2),
          }),
        );
        break;
      }
      case "line": {
        const a = at(draft.from);
        const b = at(draft.to);
        slot.svg.append(
          shape("line", { ...attrs, x1: String(a.x), y1: String(a.y), x2: String(b.x), y2: String(b.y) }),
        );
        break;
      }
      case "polygon": {
        const points = draft.points
          .map(at)
          .map((p) => `${p.x},${p.y}`)
          .join(" ");
        slot.svg.append(shape(draft.closed ? "polygon" : "polyline", { ...attrs, points }));
        break;
      }
    }
  }

  /** Shows the box text is typed into, sized to the page's scale. */
  #place(slot: Slot, editor: EditorView, size: { width: number; height: number }, at: (p: Point) => Point): void {
    const [x0, y0, x1] = editor.rect;
    const origin = at({ x: x0, y: y0 });
    const e = slot.editor;
    const scale = this.#scale(slot, size);
    slot.root.dataset.typing = "";
    e.hidden = false;
    // A view turned a quarter or more turns the typed text with the page: about the box's top-left corner.
    const turn = this.#viewer.getState().rotation;
    e.style.transformOrigin = "0 0";
    e.style.transform = turn === 0 ? "" : `rotate(${turn}deg)`;
    e.style.left = `${origin.x * 100}%`;
    e.style.top = `${origin.y * 100}%`;
    e.style.inlineSize = `${(x1 - x0) * scale}px`;
    e.style.fontFamily = editor.style.family;
    e.style.fontSize = `${editor.style.size * scale}px`;
    e.style.color = editor.style.color;
    e.style.fontWeight = editor.style.bold ? "700" : "400";
    e.style.fontStyle = editor.style.italic ? "italic" : "normal";
    e.style.textAlign = editor.style.align;
    e.setAttribute("aria-label", editor.label);
    if (e.value !== editor.text && document.activeElement !== e) e.value = editor.text;
    this.#fit(slot);
  }
}

/** The key that ends or moves typing in a line of the page's own text, if `event` is one. */
function runKeyOf(event: KeyboardEvent): "enter" | "escape" | "tab" | "shift-tab" | undefined {
  if (event.key === "Enter") return "enter";
  if (event.key === "Escape") return "escape";
  if (event.key === "Tab") return event.shiftKey ? "shift-tab" : "tab";
  return undefined;
}

/** The four corners of `box` as shown, written as path or polygon points. */
function quad([x0, y0, x1, y1]: Box, at: (p: Point) => Point, between = "L", within = " "): string {
  return [
    { x: x0, y: y0 },
    { x: x1, y: y0 },
    { x: x1, y: y1 },
    { x: x0, y: y1 },
  ]
    .map(at)
    .map((p) => `${p.x}${within}${p.y}`)
    .join(between);
}

/** Puts `element` at the fraction `at` of the page. */
function place(element: HTMLElement, at: Point): void {
  element.style.left = `${at.x * 100}%`;
  element.style.top = `${at.y * 100}%`;
  element.hidden = false;
}

/** An SVG element with `attributes`. */
function shape(tag: string, attributes: Readonly<Record<string, string>>): SVGElement {
  const element = document.createElementNS(SVG, tag);
  for (const [name, value] of Object.entries(attributes)) element.setAttribute(name, value);
  return element;
}
