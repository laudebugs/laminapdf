// The box a text box is typed into, in place over the page (Paper C03): a page layer whose slot
// holds, while a box is open, a `contenteditable` `role="textbox"` in page points at the box's
// place, so it scales and turns with the page as the other layers do. It takes the box's font,
// size, colour, fill and padding, so the browser's layout of the text stays close to the
// appearance drawn once editing ends; it grows downward as the text wraps (typewriter text grows
// to the right and never wraps). Its two side handles set the wrap width; a callout also shows its
// leader line with round handles at the knee and the end. Each handle moves by drag, or by Tab to
// it and the arrow keys (Shift: 10 pt). Escape, Done editing or a press elsewhere ends the box;
// the focus moving into the inspector doesn't. The text read back is text-session.ts's.

import type { RichText } from "@laminapdf/sdk";
import type { PageLayer, Viewer } from "@laminapdf/viewer";
import { placeBox, type Rotation } from "@laminapdf/viewer";
import type { Point } from "./marks.ts";
import { cssFont, editorHtml, richTextOf } from "./rich-text.ts";
import { calloutLine, type TextEdit, type TextLook, type TextNow } from "./text-session.ts";

/** The words the box gives screen readers, from the later catalogue. */
export interface TextEditorWords {
  /** "Callout text", by kind. */
  readonly name: (edit: TextEdit) => string;
  readonly placeholder: string;
  readonly start: string;
  readonly end: string;
  readonly knee: string;
  readonly point: string;
}

/** Why a box stopped being typed into. */
export type EndReason = "escape" | "done" | "away";

/** What the editor tells Comment mode. */
export interface TextEditorHost {
  /** The box ended: what it holds now. The box stays, read-only, until the promise settles: the edit drawn. */
  ended(edit: TextEdit, now: TextNow, reason: EndReason): Promise<void>;
  /** Its bold and italic at the caret changed, or its look did: the inspector shows them. */
  changed(): void;
  /** Whether `element`, which took the focus, is part of the inspector, which doesn't end the box. */
  inInspector(element: Element | null): boolean;
}

/** Space between the border and the text, as the appearance lays it out: border + 2 pt, or 1 pt with none. */
const padding = (look: TextLook) => (look.border > 0 ? look.border + 2 : 1);

/** The smallest a box gets by its side handles, points. */
const NARROWEST = 24;

/** The open box's elements. */
interface Open {
  readonly edit: TextEdit;
  readonly box: HTMLElement;
  readonly frame: HTMLElement;
  readonly area: HTMLElement;
  readonly line: SVGSVGElement | undefined;
  readonly handles: HTMLElement[];
  rect: [number, number, number, number];
  points: Point[];
  look: TextLook;
  /** Whether a handle moved the box or its line: otherwise the box keeps its place as stored. */
  reshaped: boolean;
  /** The caret's range when the focus last left the box, for the inspector's bold and italic. */
  range: Range | undefined;
}

const css = ([r, g, b]: readonly number[]) =>
  `rgb(${Math.round((r ?? 0) * 255)} ${Math.round((g ?? 0) * 255)} ${Math.round((b ?? 0) * 255)})`;

/** The editing layer: add it to the viewer while Comment mode is on. */
export class TextEditor implements PageLayer {
  readonly name = "annot-text";
  readonly #viewer: Viewer;
  readonly #host: TextEditorHost;
  readonly #words: TextEditorWords;
  readonly #slots = new Map<number, { element: HTMLElement; size: { width: number; height: number } | undefined }>();
  #open: Open | undefined;
  /** Bold and italic at the caret change as it moves: the inspector follows. */
  readonly #caretMoved = () => {
    if (this.#open?.area.contains(document.activeElement) === true) this.#host.changed();
  };
  /** A press outside the box and the inspector ends it, wherever the focus was (its text, a handle, a field). */
  readonly #pressed = (e: PointerEvent) => {
    const target = e.target as Element | null;
    if (this.#open === undefined || this.contains(target) || this.#host.inInspector(target)) return;
    // The press only ends the box: the tool under it lets this one press go by.
    this.#endingPress = e;
    setTimeout(() => {
      if (this.#endingPress === e) this.#endingPress = undefined;
    }, 0);
    this.end("away");
  };
  /** The press that just ended a box, until its own handlers have run. */
  #endingPress: PointerEvent | undefined;

  constructor(viewer: Viewer, host: TextEditorHost, words: TextEditorWords) {
    this.#viewer = viewer;
    this.#host = host;
    this.#words = words;
    document.addEventListener("selectionchange", this.#caretMoved);
    // In the capture phase: before a tool's own press, which a box being typed into lets through.
    document.addEventListener("pointerdown", this.#pressed, true);
  }

  /** Removes the layer's listeners; an open box is dropped. */
  destroy(): void {
    document.removeEventListener("selectionchange", this.#caretMoved);
    document.removeEventListener("pointerdown", this.#pressed, true);
    this.cancel();
  }

  onMount(page: number, element: HTMLElement): void {
    this.#slots.set(page, { element, size: undefined });
  }

  onUnmount(page: number): void {
    // A page scrolled out of the mounted range ends its box, as a press elsewhere does.
    if (this.#open?.edit.page === page) this.end("away");
    this.#slots.delete(page);
  }

  onZoom(page: number, _element: HTMLElement, size: { readonly width: number; readonly height: number }): void {
    const slot = this.#slots.get(page);
    if (slot === undefined) return;
    slot.size = { width: size.width, height: size.height };
    if (this.#open?.edit.page === page) this.#place(this.#open);
  }

  /** The box open now. */
  get editing(): TextEdit | undefined {
    return this.#open?.edit;
  }

  /** Whether a box is open, or a press is ending one: either way the press is the box's, not a tool's. */
  get busy(): boolean {
    return this.#open !== undefined || this.#endingPress !== undefined;
  }

  /** Whether `element` is part of the open box: its text, handles or line. */
  contains(element: Element | null): boolean {
    return element !== null && this.#open?.box.contains(element) === true;
  }

  /** Opens `edit` for typing, the caret at the end of its text; false when its page isn't mounted. */
  open(edit: TextEdit): boolean {
    const slot = this.#slots.get(edit.page);
    if (slot === undefined) return false;
    const box = document.createElement("div");
    box.className = "web-text-editor";
    const frame = document.createElement("div");
    frame.className = "web-text-editor-frame";
    const area = this.#area(edit);
    frame.append(area);
    const line = edit.kind === "callout" ? document.createElementNS("http://www.w3.org/2000/svg", "svg") : undefined;
    if (line !== undefined) line.classList.add("web-text-editor-line");
    const open: Open = {
      edit,
      box,
      frame,
      area,
      line,
      handles: [],
      rect: [...edit.rect],
      points: [...(edit.line ?? [])],
      look: edit.look,
      reshaped: false,
      range: undefined,
    };
    open.handles.push(...this.#handles(open));
    box.append(...(line === undefined ? [] : [line]), frame, ...open.handles);
    slot.element.append(box);
    this.#open = open;
    this.#style(open);
    this.#place(open);
    area.focus({ preventScroll: true });
    caretAtEnd(area);
    return true;
  }

  /** Ends the open box: Comment mode makes its edit. */
  end(reason: EndReason): void {
    const open = this.#open;
    if (open === undefined) return;
    this.#open = undefined;
    const text = richTextOf(open.area);
    // The box's layout in the browser can be a little taller than the box as stored: an untouched
    // box keeps its place, so opening one and leaving it makes no edit.
    const changed = open.reshaped || JSON.stringify(text) !== JSON.stringify(open.edit.text);
    const now: TextNow = {
      text,
      rect: changed ? open.rect : open.edit.rect,
      ...(open.edit.kind === "callout" ? { line: changed ? open.points : (open.edit.line ?? []) } : {}),
      look: open.look,
    };
    // The box shows the text until the page draws the edit, so nothing flickers between them.
    open.area.contentEditable = "false";
    open.box.dataset.ended = "true";
    if (open.box.contains(document.activeElement)) (document.activeElement as HTMLElement).blur();
    void this.#host.ended(open.edit, now, reason).finally(() => open.box.remove());
  }

  /** Drops the open box without an edit. */
  cancel(): void {
    this.#open?.box.remove();
    this.#open = undefined;
  }

  /** Gives the open box `look`: the inspector's font, size, colour, fill, border or line end. */
  setLook(look: TextLook): void {
    const open = this.#open;
    if (open === undefined) return;
    open.look = look;
    this.#style(open);
    this.#place(open);
    this.#refocus(open);
  }

  /** The open box's look now. */
  look(): TextLook | undefined {
    return this.#open?.look;
  }

  /** The open box's text now. */
  text(): RichText | undefined {
    const open = this.#open;
    return open === undefined ? undefined : richTextOf(open.area);
  }

  /** Whether the text at the caret is bold and italic. */
  styleAtCaret(): { bold: boolean; italic: boolean } {
    if (this.#open === undefined) return { bold: false, italic: false };
    return { bold: document.queryCommandState("bold"), italic: document.queryCommandState("italic") };
  }

  /**
   * Bold or italic, as ⌘B and ⌘I do: on the selected text, or at a caret for what is typed next.
   * With no caret in the box at all, on all of its text.
   */
  toggle(style: "bold" | "italic"): void {
    const open = this.#open;
    if (open === undefined) return;
    this.#refocus(open);
    const selection = window.getSelection();
    if (selection !== null && (selection.rangeCount === 0 || !open.area.contains(selection.anchorNode))) {
      selection.selectAllChildren(open.area);
    }
    document.execCommand(style);
    this.#grow(open);
    this.#host.changed();
  }

  /** The text area of `edit`: its text, name and keys. */
  #area(edit: TextEdit): HTMLElement {
    const area = document.createElement("div");
    area.className = "web-text-editor-area";
    area.contentEditable = "true";
    area.setAttribute("role", "textbox");
    area.setAttribute("aria-multiline", String(edit.kind !== "typewriter"));
    area.setAttribute("aria-label", this.#words.name(edit));
    area.dataset.placeholder = this.#words.placeholder;
    area.spellcheck = true;
    area.innerHTML = editorHtml(edit.text);
    area.addEventListener("keydown", (e) => this.#key(e));
    area.addEventListener("input", () => {
      const open = this.#open;
      if (open === undefined) return;
      this.#grow(open);
      this.#host.changed();
    });
    area.addEventListener("focusout", (e) => this.#left(e));
    return area;
  }

  #key(e: KeyboardEvent): void {
    // Typing stays in the box: no page shortcut (T, H, Delete) sees it.
    e.stopPropagation();
    const mod = e.metaKey || e.ctrlKey;
    if (e.key === "Escape") {
      e.preventDefault();
      this.end("escape");
    } else if (mod && !e.altKey && (e.key === "b" || e.key === "B" || e.key === "i" || e.key === "I")) {
      e.preventDefault();
      this.toggle(e.key.toLowerCase() === "b" ? "bold" : "italic");
    }
  }

  /**
   * The focus left the text: into the box's own handles or the inspector keeps it open, and so
   * does the focus leaving the window. Tab to anything else ends it; a press elsewhere ends it
   * through `#pressed`, whatever it focuses.
   */
  #left(e: FocusEvent): void {
    const open = this.#open;
    if (open === undefined) return;
    const to = e.relatedTarget as Element | null;
    const selection = window.getSelection();
    if (selection !== null && selection.rangeCount > 0 && open.area.contains(selection.anchorNode)) {
      open.range = selection.getRangeAt(0).cloneRange();
    }
    if (to === null || this.contains(to) || this.#host.inInspector(to)) return;
    this.end("away");
  }

  /** The focus back in the text, at the caret it had. */
  #refocus(open: Open): void {
    if (open.area.contains(document.activeElement)) return;
    open.area.focus({ preventScroll: true });
    if (open.range === undefined) return;
    const selection = window.getSelection();
    selection?.removeAllRanges();
    selection?.addRange(open.range);
  }

  /** The text's look as CSS, in page points (the layer's box is in points). */
  #style(open: Open): void {
    const { look, area, edit } = open;
    const pad = padding(look);
    Object.assign(area.style, {
      fontFamily: cssFont(look.font),
      fontSize: `${look.size}px`,
      lineHeight: "1.2",
      padding: `${pad}px`,
      color: css(look.colour),
      background: edit.kind === "typewriter" || look.fill === undefined ? "transparent" : css(look.fill),
      textAlign: look.align,
      whiteSpace: edit.kind === "typewriter" ? "pre" : "pre-wrap",
      border: look.border > 0 && edit.kind !== "typewriter" ? `${look.border}px solid ${css(look.colour)}` : "0",
    });
    open.frame.dataset.kind = edit.kind;
  }

  /** Places the open box in its page's slot: the layer box in points, the frame at the box's place. */
  #place(open: Open): void {
    const slot = this.#slots.get(open.edit.page);
    const points = this.#viewer.pageSizes()[open.edit.page];
    if (slot?.size === undefined || points === undefined) return;
    const rotation = this.#viewer.getState().rotation as Rotation;
    placeBox(open.box, points, slot.size, rotation);
    const turned = rotation % 180 !== 0;
    const scale = slot.size.width / (turned ? points.height : points.width);
    open.box.style.setProperty("--web-text-editor-px", `${1 / scale}px`);
    this.#grow(open);
  }

  /** Sizes the frame to the box, grown to hold the text, and moves the handles and line with it. */
  #grow(open: Open): void {
    const [x0, y0, x1] = open.rect;
    const typewriter = open.edit.kind === "typewriter";
    Object.assign(open.frame.style, {
      left: `${x0}px`,
      top: `${y0}px`,
      width: typewriter ? "auto" : `${x1 - x0}px`,
      minWidth: `${typewriter ? x1 - x0 : 0}px`,
      minHeight: `${open.edit.rect[3] - open.edit.rect[1]}px`,
    });
    const height = Math.max(open.edit.rect[3] - open.edit.rect[1], open.area.offsetHeight);
    const width = typewriter ? Math.max(x1 - x0, open.area.offsetWidth) : x1 - x0;
    open.rect = [x0, y0, x0 + width, y0 + height];
    if (open.edit.kind === "callout" && open.points.length > 0) {
      const at = open.points[0] as Point;
      const knee = open.points.length > 2 ? (open.points[1] as Point) : undefined;
      const fresh = calloutLine(at, open.rect);
      open.points = knee === undefined ? [at, fresh[2] as Point] : [at, knee, fresh[2] as Point];
    }
    this.#drawLine(open);
    this.#placeHandles(open);
  }

  #drawLine(open: Open): void {
    if (open.line === undefined) return;
    const path = document.createElementNS("http://www.w3.org/2000/svg", "polyline");
    path.setAttribute("points", open.points.map(([x, y]) => `${x},${y}`).join(" "));
    path.setAttribute("fill", "none");
    path.setAttribute("stroke", css(open.look.colour));
    path.setAttribute("stroke-width", String(Math.max(open.look.border, 1)));
    open.line.replaceChildren(path);
  }

  /** The side handles, and a callout's knee and end handles. */
  #handles(open: Open): HTMLElement[] {
    const handle = (role: Handle, label: string) => {
      const h = document.createElement("button");
      h.type = "button";
      h.className = `web-text-editor-handle web-text-editor-handle-${role}`;
      h.dataset.handle = role;
      h.setAttribute("aria-label", label);
      h.addEventListener("pointerdown", (e) => this.#dragHandle(e, role));
      h.addEventListener("keydown", (e) => this.#keyHandle(e, role));
      return h;
    };
    const sides =
      open.edit.kind === "typewriter" ? [] : [handle("start", this.#words.start), handle("end", this.#words.end)];
    if (open.edit.kind !== "callout") return sides;
    const ends = open.points.length > 2 ? [handle("knee", this.#words.knee)] : [];
    return [...sides, ...ends, handle("point", this.#words.point)];
  }

  /** Where handle `role` of the open box is, page points. */
  #handleAt(open: Open, role: Handle): Point | undefined {
    const [x0, y0, x1, y1] = open.rect;
    switch (role) {
      case "start":
        return [x0, (y0 + y1) / 2];
      case "end":
        return [x1, (y0 + y1) / 2];
      case "knee":
        return open.points.length > 2 ? open.points[1] : undefined;
      default:
        return open.points[0];
    }
  }

  #placeHandles(open: Open): void {
    for (const h of open.handles) {
      const p = this.#handleAt(open, h.dataset.handle as Handle);
      if (p !== undefined) Object.assign(h.style, { left: `${p[0]}px`, top: `${p[1]}px` });
    }
  }

  /** Moves handle `role` to page point `to`. */
  #moveHandle(open: Open, role: Handle, to: Point): void {
    const [x0, y0, x1, y1] = open.rect;
    if (role === "start") open.rect = [Math.min(to[0], x1 - NARROWEST), y0, x1, y1];
    else if (role === "end") open.rect = [x0, y0, Math.max(to[0], x0 + NARROWEST), y1];
    else if (role === "knee") open.points = [open.points[0] as Point, to, ...open.points.slice(2)];
    else open.points = [to, ...open.points.slice(1)];
    open.reshaped = true;
    this.#grow(open);
  }

  #dragHandle(e: PointerEvent, role: Handle): void {
    const open = this.#open;
    if (open === undefined || e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    const target = e.currentTarget as HTMLElement;
    target.setPointerCapture(e.pointerId);
    const move = (m: PointerEvent) => {
      const to = this.#pointOn(open, m.clientX, m.clientY);
      if (to !== undefined) this.#moveHandle(open, role, to);
    };
    const up = () => {
      target.removeEventListener("pointermove", move);
      target.removeEventListener("pointerup", up);
      target.removeEventListener("pointercancel", up);
      this.#refocus(open);
    };
    target.addEventListener("pointermove", move);
    target.addEventListener("pointerup", up);
    target.addEventListener("pointercancel", up);
  }

  #keyHandle(e: KeyboardEvent, role: Handle): void {
    const open = this.#open;
    const arrow = ARROWS[e.key];
    e.stopPropagation();
    if (e.key === "Escape") {
      e.preventDefault();
      this.end("escape");
      return;
    }
    if (open === undefined || arrow === undefined) return;
    e.preventDefault();
    const step = e.shiftKey ? 10 : 1;
    const from = this.#handleAt(open, role);
    if (from === undefined) return;
    this.#moveHandle(open, role, [from[0] + arrow[0] * step, from[1] + arrow[1] * step]);
  }

  /** The page point under the pointer, in the open box's page. */
  #pointOn(open: Open, clientX: number, clientY: number): Point | undefined {
    const points = this.#viewer.pageSizes()[open.edit.page];
    if (points === undefined) return undefined;
    const r = open.box.getBoundingClientRect();
    const rotation = this.#viewer.getState().rotation as Rotation;
    const turned = rotation % 180 !== 0;
    const scale = r.width / (turned ? points.height : points.width);
    const [dx, dy] = [(clientX - r.left) / scale, (clientY - r.top) / scale];
    // The box turns with the view: a point on screen maps back through the turn.
    switch (rotation) {
      case 90:
        return [dy, points.height - dx];
      case 180:
        return [points.width - dx, points.height - dy];
      case 270:
        return [points.width - dy, dx];
      default:
        return [dx, dy];
    }
  }
}

/** A handle of the open box. */
type Handle = "start" | "end" | "knee" | "point";

/** Each arrow key's step, in page points. */
const ARROWS: Readonly<Record<string, Point>> = {
  ArrowLeft: [-1, 0],
  ArrowRight: [1, 0],
  ArrowUp: [0, -1],
  ArrowDown: [0, 1],
};

/**
 * Puts the caret at the end of `area`'s text: inside its last line, before a closing line break,
 * so what is typed joins that line in every browser (Firefox would start a new one after the
 * line's `div`).
 */
function caretAtEnd(area: HTMLElement): void {
  const selection = window.getSelection();
  if (selection === null) return;
  let last: Node = area;
  while (last.lastChild !== null && last.lastChild.nodeName !== "BR") last = last.lastChild;
  const range = document.createRange();
  range.selectNodeContents(last);
  range.collapse(false);
  selection.removeAllRanges();
  selection.addRange(range);
}
