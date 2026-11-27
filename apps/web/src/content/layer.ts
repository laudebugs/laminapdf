// Edit mode's marks over each page (Paper A01, A02): the selected object's frame with its eight
// handles and the turning handle, the dashed snap guides while a drag is near one, the outline of
// what a tool is drawing, and the box text is typed into. It is a viewer layer (`PageLayer`), a
// DOM overlay above the page, so a drag moves only the overlay and the page redraws once, when
// the edit is made (E6f). Handles are marks for the pointer (`aria-hidden`): every handle's
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
  readonly draft?: Draft;
  readonly editor?: EditorView;
}

/** One page's overlay. */
interface Slot {
  readonly root: HTMLElement;
  readonly svg: SVGSVGElement;
  readonly handles: Map<HandleName | "turn", HTMLElement>;
  readonly editor: HTMLTextAreaElement;
  readonly labels: HTMLElement;
  width: number;
  height: number;
}

const SVG = "http://www.w3.org/2000/svg";

/** A point of the page (page space) as a fraction of the page as shown, in a view turned by `rotation`. */
export function shownFraction(point: Point, size: { width: number; height: number }, rotation: number): Point {
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
    root.append(editor);
    element.append(root);
    const slot: Slot = { root, svg, handles, editor, labels, width: 0, height: 0 };
    this.#slots.set(page, slot);
    this.#draw(page, slot);
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
    if (view.guides?.page === page) this.#drawGuides(slot, view.guides.guides, size, at);
    const draft = view.draft?.page === page ? view.draft : undefined;
    if (draft !== undefined) this.#drawDraft(slot, draft, at);
    const editor = view.editor?.page === page ? view.editor : undefined;
    if (editor !== undefined) this.#place(slot, editor, size, at);
    else slot.editor.hidden = true;
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
