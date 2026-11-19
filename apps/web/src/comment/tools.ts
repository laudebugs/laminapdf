// Comment mode's tools, one module per mark: what a press, a drag and a release make, as state
// machines over page points. Each draws its preview and finishes its mark through a `DrawHost`,
// which Comment mode backs with the annotation layer's gestures and the edit session, so the
// machines run in Node with a fake host. Every tool stays on until Escape or another tool.

import type { ToolModule } from "@laminapdf/ui";
import type { TextKind } from "@laminapdf/ui/editing";
import { type Box, caretBox, dragBox, lineEnd, type Mark, type PageText, type Point, textLines } from "./marks.ts";

/** The pointer as a tool reads it. */
export interface PointerInput {
  readonly clientX: number;
  readonly clientY: number;
  readonly button: number;
  readonly shiftKey: boolean;
  readonly timeStamp: number;
}

/** What the tools need of Comment mode. */
export interface DrawHost {
  /** The page under the pointer and the point there, in page points. */
  at(input: PointerInput): { readonly page: number; readonly point: Point } | undefined;
  /** The page's text, if it has been read; reading starts on the first ask. */
  text(page: number): PageText | undefined;
  /** The page's text once read (undefined when it can't be). */
  whenText(page: number): Promise<PageText | undefined>;
  /** Draws what the tool would make on `page`, or clears it. */
  draw(page: number, mark: Mark | undefined): void;
  /** Makes `mark` on `page` (the preview goes once its page is drawn); `undefined` drops the preview. */
  finish(page: number, mark: Mark | undefined): void;
  /** Runs `run` after `ms`; returns what cancels it. */
  later(run: () => void, ms: number): () => void;
  /** Opens a new box of `kind` for typing, from a press at `from` and a release at `to` (C03). */
  openText(page: number, kind: TextKind, from: Point, to: Point): void;
  /** Whether a box is open for typing: a press then only ends it. */
  typing(): boolean;
  /** The chosen stamp's size on the page, points; undefined before one is chosen (C05). */
  stampSize(): readonly [number, number] | undefined;
  /** Places the chosen stamp centred on `at`. */
  placeStamp(page: number, at: Point): void;
  /** Asks for a file and attaches it under an icon at `at`. */
  attachAt(page: number, at: Point): void;
}

/** A Comment tool: an interaction mode that makes one kind of mark. */
export interface MarkTool extends ToolModule {
  /** Ends what the tool is drawing: a polygon or a pending ink drawing is made, anything else dropped. */
  done(): void;
  /** Drops what the tool is drawing. */
  cancel(): void;
}

/** Smaller than this, in points, a drag is a click and makes no shape. */
const SMALLEST = 3;
/** Ink strokes this close in time join one drawing (C02). */
const INK_JOIN_MS = 1000;
/** A second click this close to the last vertex, in points and ms, ends a polygon. */
const CLOSE_PT = 4;
const CLOSE_MS = 500;

/** The interaction modes Comment mode adds, by declaration merging. */
declare module "@laminapdf/ui" {
  interface InteractionModes {
    highlight: true;
    underline: true;
    strike: true;
    caret: true;
    note: true;
    ink: true;
    rectangle: true;
    ellipse: true;
    line: true;
    arrow: true;
    polygon: true;
    polyline: true;
    textBox: true;
    callout: true;
    typewriter: true;
    stamp: true;
    attach: true;
  }
}

/** Highlight, Underline and Strike: a drag across text marks the glyphs it covers, line by line. */
export function textTool(name: "highlight" | "underline" | "strike", host: DrawHost): MarkTool {
  const kind = name === "strike" ? "strikeOut" : name;
  let press: { page: number; from: Point; to: Point; lines: Box[] } | undefined;
  const mark = (lines: readonly Box[]): Mark => ({ type: "markup", kind, lines });
  return {
    name,
    cursor: "text",
    selectsText: false,
    pointerDown(event) {
      const at = host.at(event);
      if (at === undefined || event.button !== 0) return false;
      host.text(at.page);
      press = { page: at.page, from: at.point, to: at.point, lines: [] };
      return true;
    },
    pointerMove(event) {
      const at = host.at(event);
      if (press === undefined || at === undefined || at.page !== press.page) return;
      const text = host.text(press.page);
      press.to = at.point;
      press.lines = text === undefined ? [] : textLines(text, press.from, at.point);
      host.draw(press.page, press.lines.length === 0 ? undefined : mark(press.lines));
    },
    pointerUp(event) {
      if (press === undefined) return;
      this.pointerMove?.(event as PointerEvent, undefined as never);
      const { page, from, to, lines } = press;
      press = undefined;
      if (host.text(page) !== undefined) {
        host.finish(page, lines.length === 0 ? undefined : mark(lines));
        return;
      }
      // Released before the page's text was read: mark it once it is.
      void host.whenText(page).then((text) => {
        const late = text === undefined ? [] : textLines(text, from, to);
        host.finish(page, late.length === 0 ? undefined : mark(late));
      });
    },
    done() {
      this.cancel();
    },
    cancel() {
      if (press !== undefined) host.finish(press.page, undefined);
      press = undefined;
    },
  };
}

/** Caret and Note: a click places the mark (a caret between the two letters nearest the click). */
function clickTool(name: "caret" | "note", host: DrawHost): MarkTool {
  let down: { page: number } | undefined;
  return {
    name,
    cursor: "crosshair",
    selectsText: false,
    pointerDown(event) {
      const at = host.at(event);
      if (at === undefined || event.button !== 0) return false;
      if (name === "caret") host.text(at.page);
      down = { page: at.page };
      return true;
    },
    pointerUp(event) {
      const at = host.at(event);
      const page = down?.page;
      down = undefined;
      if (at === undefined || page !== at.page) return;
      if (name === "note") {
        host.finish(page, { type: "note", at: at.point });
        return;
      }
      void host
        .whenText(page)
        .then((text) => host.finish(page, { type: "caret", box: caretBox(text ?? { runs: [], quads: [] }, at.point) }));
    },
    done() {
      down = undefined;
    },
    cancel() {
      down = undefined;
    },
  };
}

/** Ink: each drag a stroke; strokes drawn within a second of each other on one page make one drawing. */
export function inkTool(host: DrawHost): MarkTool {
  let drawing: { page: number; strokes: Point[][]; drawing: boolean } | undefined;
  let stopWait: (() => void) | undefined;
  const mark = (): Mark | undefined => {
    const strokes = drawing?.strokes.filter((s) => s.length > 0) ?? [];
    return strokes.length === 0
      ? undefined
      : { type: "ink", strokes: strokes.map((s) => (s.length === 1 ? [s[0] as Point, s[0] as Point] : s)) };
  };
  const flush = () => {
    stopWait?.();
    stopWait = undefined;
    if (drawing === undefined) return;
    const { page } = drawing;
    const made = mark();
    drawing = undefined;
    host.finish(page, made);
  };
  return {
    name: "ink",
    cursor: "crosshair",
    selectsText: false,
    pointerDown(event) {
      const at = host.at(event);
      if (at === undefined || event.button !== 0) return false;
      if (drawing !== undefined && drawing.page !== at.page) flush();
      stopWait?.();
      stopWait = undefined;
      drawing ??= { page: at.page, strokes: [], drawing: false };
      drawing.strokes.push([at.point]);
      drawing.drawing = true;
      host.draw(drawing.page, mark());
      return true;
    },
    pointerMove(event) {
      const at = host.at(event);
      if (drawing?.drawing !== true || at === undefined || at.page !== drawing.page) return;
      const stroke = drawing.strokes[drawing.strokes.length - 1];
      const last = stroke?.[stroke.length - 1];
      if (
        stroke === undefined ||
        (last !== undefined && Math.hypot(at.point[0] - last[0], at.point[1] - last[1]) < 0.5)
      )
        return;
      stroke.push(at.point);
      host.draw(drawing.page, mark());
    },
    pointerUp() {
      if (drawing === undefined) return;
      drawing.drawing = false;
      stopWait = host.later(flush, INK_JOIN_MS);
    },
    done: flush,
    cancel() {
      stopWait?.();
      stopWait = undefined;
      if (drawing !== undefined) host.finish(drawing.page, undefined);
      drawing = undefined;
    },
  };
}

/** Rectangle, Ellipse, Line and Arrow: a drag from corner to corner, or end to end; Shift for squares, circles and 45°. */
export function dragTool(name: "rectangle" | "ellipse" | "line" | "arrow", host: DrawHost): MarkTool {
  let press: { page: number; from: Point; mark: Mark | undefined } | undefined;
  const markFor = (from: Point, to: Point, shift: boolean): Mark | undefined => {
    if (name === "line" || name === "arrow") {
      const end = lineEnd(from, to, shift);
      return Math.hypot(end[0] - from[0], end[1] - from[1]) < SMALLEST
        ? undefined
        : { type: "line", from, to: end, arrow: name === "arrow" };
    }
    const box = dragBox(from, to, shift);
    return box[2] - box[0] < SMALLEST || box[3] - box[1] < SMALLEST
      ? undefined
      : { type: "shape", kind: name === "rectangle" ? "square" : "circle", box };
  };
  return {
    name,
    cursor: "crosshair",
    selectsText: false,
    pointerDown(event) {
      const at = host.at(event);
      if (at === undefined || event.button !== 0) return false;
      host.text(at.page);
      press = { page: at.page, from: at.point, mark: undefined };
      return true;
    },
    pointerMove(event) {
      const at = host.at(event);
      if (press === undefined || at === undefined || at.page !== press.page) return;
      press.mark = markFor(press.from, at.point, event.shiftKey);
      host.draw(press.page, press.mark);
    },
    pointerUp(event) {
      if (press === undefined) return;
      this.pointerMove?.(event as PointerEvent, undefined as never);
      const { page, mark } = press;
      press = undefined;
      host.finish(page, mark);
    },
    done() {
      this.cancel();
    },
    cancel() {
      if (press !== undefined) host.finish(press.page, undefined);
      press = undefined;
    },
  };
}

/** Polygon and Polyline: a click at each corner; a second click on the last corner, or Enter, ends it. */
export function polyTool(name: "polygon" | "polyline", host: DrawHost): MarkTool {
  const kind = name === "polygon" ? "polygon" : "polyLine";
  const least = name === "polygon" ? 3 : 2;
  let shape: { page: number; vertices: Point[]; last: number } | undefined;
  const mark = (vertices: readonly Point[]): Mark => ({ type: "poly", kind, vertices });
  const finish = () => {
    if (shape === undefined) return;
    const { page, vertices } = shape;
    shape = undefined;
    host.finish(page, vertices.length >= least ? mark(vertices) : undefined);
  };
  return {
    name,
    cursor: "crosshair",
    selectsText: false,
    pointerDown(event) {
      const at = host.at(event);
      if (at === undefined || event.button !== 0) return false;
      if (shape !== undefined && shape.page !== at.page) finish();
      if (shape === undefined) host.text(at.page);
      shape ??= { page: at.page, vertices: [], last: Number.NEGATIVE_INFINITY };
      return true;
    },
    pointerMove(event) {
      const at = host.at(event);
      if (shape === undefined || at === undefined || at.page !== shape.page) return;
      host.draw(shape.page, mark([...shape.vertices, at.point]));
    },
    pointerUp(event) {
      const at = host.at(event);
      if (shape === undefined || at === undefined || at.page !== shape.page) return;
      const previous = shape.vertices[shape.vertices.length - 1];
      const again =
        previous !== undefined &&
        Math.hypot(at.point[0] - previous[0], at.point[1] - previous[1]) < CLOSE_PT &&
        event.timeStamp - shape.last < CLOSE_MS;
      if (again) {
        finish();
        return;
      }
      shape.vertices.push(at.point);
      shape.last = event.timeStamp;
      host.draw(shape.page, mark(shape.vertices));
    },
    done: finish,
    cancel() {
      if (shape !== undefined) host.finish(shape.page, undefined);
      shape = undefined;
    },
  };
}

/**
 * Text box, Callout and Typewriter text (C03): a press and release open a box for typing. A text
 * box takes the drag's width, or a default size at a click; a callout's press is the point it
 * points at and its release where its box goes; typewriter text starts at the press. While a box
 * is open, a press elsewhere only ends it.
 */
export function typeTool(kind: TextKind, host: DrawHost): MarkTool {
  let press: { page: number; from: Point } | undefined;
  return {
    name: kind,
    cursor: kind === "typewriter" ? "text" : "crosshair",
    selectsText: false,
    pointerDown(event) {
      if (host.typing() || event.button !== 0) return false;
      const at = host.at(event);
      if (at === undefined) return false;
      press = { page: at.page, from: at.point };
      return true;
    },
    pointerMove(event) {
      const at = host.at(event);
      if (press === undefined || at === undefined || at.page !== press.page || kind !== "textBox") return;
      host.draw(press.page, { type: "shape", kind: "square", box: dragBox(press.from, at.point, false) });
    },
    pointerUp(event) {
      if (press === undefined) return;
      const { page, from } = press;
      press = undefined;
      const at = host.at(event);
      host.draw(page, undefined);
      host.openText(page, kind, from, at !== undefined && at.page === page ? at.point : from);
    },
    done() {
      this.cancel();
    },
    cancel() {
      if (press !== undefined) host.draw(press.page, undefined);
      press = undefined;
    },
  };
}

/** Stamp (C05): the chosen stamp's box follows a press, and the release places it there. */
export function stampTool(host: DrawHost): MarkTool {
  let shown: number | undefined;
  const clear = () => {
    if (shown !== undefined) host.draw(shown, undefined);
    shown = undefined;
  };
  return {
    name: "stamp",
    cursor: "copy",
    selectsText: false,
    pointerDown(event) {
      return event.button === 0 && host.at(event) !== undefined && host.stampSize() !== undefined;
    },
    pointerMove(event) {
      const at = host.at(event);
      const size = host.stampSize();
      if (at === undefined || size === undefined) return clear();
      if (shown !== undefined && shown !== at.page) clear();
      shown = at.page;
      const [w, h] = size;
      const [x, y] = at.point;
      host.draw(at.page, { type: "shape", kind: "square", box: [x - w / 2, y - h / 2, x + w / 2, y + h / 2] });
    },
    pointerUp(event) {
      const at = host.at(event);
      // A cancelled press (a touch that became a scroll) places nothing.
      if (event.type === "pointercancel" || at === undefined) return clear();
      // The preview stays until the stamp's drawing replaces it, or placing drops it.
      shown = undefined;
      host.placeStamp(at.page, at.point);
    },
    done: clear,
    cancel: clear,
  };
}

/** Attach file (C05): a click asks for the file, which goes under an icon centred on the click. */
export function attachTool(host: DrawHost): MarkTool {
  return {
    name: "attach",
    cursor: "copy",
    selectsText: false,
    pointerDown(event) {
      return event.button === 0 && host.at(event) !== undefined;
    },
    pointerUp(event) {
      const at = host.at(event);
      if (event.type !== "pointercancel" && at !== undefined) host.attachAt(at.page, at.point);
    },
    done() {},
    cancel() {},
  };
}

/** Every tool, by its interaction mode: Select is the reader's own. */
export function markTools(host: DrawHost): MarkTool[] {
  return [
    textTool("highlight", host),
    textTool("underline", host),
    textTool("strike", host),
    clickTool("caret", host),
    clickTool("note", host),
    inkTool(host),
    dragTool("rectangle", host),
    dragTool("ellipse", host),
    dragTool("line", host),
    dragTool("arrow", host),
    polyTool("polygon", host),
    polyTool("polyline", host),
    typeTool("textBox", host),
    typeTool("callout", host),
    typeTool("typewriter", host),
    stampTool(host),
    attachTool(host),
  ];
}

/**
 * The mark a tool places from the keyboard (Enter, C01): a default-size one in the middle of the
 * page in view, `centre`, which the arrow keys then move. Caret goes between the letters nearest
 * the middle. Text tools mark the text selection instead, so they have none here.
 */
export function placedMark(tool: string, centre: Point, text: PageText | undefined): Mark | undefined {
  const [x, y] = centre;
  switch (tool) {
    case "note":
      return { type: "note", at: [x - 10, y - 10] };
    case "caret":
      return { type: "caret", box: caretBox(text ?? { runs: [], quads: [] }, centre) };
    case "ink":
      return {
        type: "ink",
        strokes: [
          [
            [x - 36, y],
            [x - 18, y - 12],
            [x, y],
            [x + 18, y + 12],
            [x + 36, y],
          ],
        ],
      };
    case "rectangle":
    case "ellipse":
      return { type: "shape", kind: tool === "rectangle" ? "square" : "circle", box: [x - 48, y - 32, x + 48, y + 32] };
    case "line":
    case "arrow":
      return { type: "line", from: [x - 48, y], to: [x + 48, y], arrow: tool === "arrow" };
    case "polygon":
      return {
        type: "poly",
        kind: "polygon",
        vertices: [
          [x, y - 32],
          [x + 40, y + 32],
          [x - 40, y + 32],
        ],
      };
    case "polyline":
      return {
        type: "poly",
        kind: "polyLine",
        vertices: [
          [x - 48, y + 16],
          [x - 16, y - 16],
          [x + 16, y + 16],
          [x + 48, y - 16],
        ],
      };
    default:
      return undefined;
  }
}
