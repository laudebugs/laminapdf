// What Comment mode's tools make, kept apart from the DOM so it can be tested in Node: a mark in
// page space (points from the displayed page's top-left, y down, ADR 0027) as the tools draw it,
// its preview, and the `addAnnot` that writes it in user space through the page's frame (its
// crop box, `/Rotate` and `/UserUnit`, as laminapdf-model's PageFrame maps them). Also the text
// a drag covers, snapped to the page's glyph quads; where a caret goes between two letters; whether
// an opaque shape covers text; and the spec patch that resizes an annotation.

import type { Op, PageAnnotation } from "@laminapdf/sdk";
import type { CommentColour, LineEndingName } from "@laminapdf/ui/editing";
import type { PreviewShape } from "@laminapdf/viewer";

/** `[x0, y0, x1, y1]` in page points, top-left and bottom-right. */
export type Box = readonly [number, number, number, number];

/** A point in page points. */
export type Point = readonly [number, number];

/** What a tool makes, in page space. */
export type Mark =
  | { readonly type: "markup"; readonly kind: "highlight" | "underline" | "strikeOut"; readonly lines: readonly Box[] }
  | { readonly type: "note"; readonly at: Point }
  | { readonly type: "caret"; readonly box: Box }
  | { readonly type: "ink"; readonly strokes: readonly (readonly Point[])[] }
  | { readonly type: "shape"; readonly kind: "square" | "circle"; readonly box: Box }
  | { readonly type: "line"; readonly from: Point; readonly to: Point; readonly arrow: boolean }
  | { readonly type: "poly"; readonly kind: "polygon" | "polyLine"; readonly vertices: readonly Point[] };

/** How a mark looks: its colour, fill, width and opacity. */
export interface MarkStyle {
  /** RGB, 0 to 1. */
  readonly colour: readonly [number, number, number];
  /** A shape's fill; none without. */
  readonly fill?: readonly [number, number, number];
  /** A line's width in points. */
  readonly width: number;
  /** 0 to 1. */
  readonly opacity: number;
}

/** A page's frame: its crop box in user space, its `/Rotate` and its `/UserUnit`. */
export interface Frame {
  readonly crop: readonly [number, number, number, number];
  readonly rotate: number;
  readonly unit: number;
}

/** The colours by name, as the design language's tokens give them (fills from annotation-*, lines from content-line-*). */
const FILLS: Readonly<Record<string, readonly [number, number, number]>> = {
  yellow: hex("F2D675"),
  green: hex("B7D9A8"),
  blue: hex("A9C8E8"),
  pink: hex("EBB4C8"),
};
const LINES: Readonly<Record<string, readonly [number, number, number]>> = {
  black: hex("262421"),
  red: hex("B3261E"),
  blue: hex("2D5DA8"),
  green: hex("2F6A45"),
};

function hex(value: string): readonly [number, number, number] {
  const byte = (at: number) => Number.parseInt(value.slice(at, at + 2), 16) / 255;
  return [byte(0), byte(2), byte(4)];
}

/** A named colour as RGB: a fill colour or a line colour. */
export function colourValue(name: CommentColour, family: "fill" | "line"): readonly [number, number, number] {
  return (family === "fill" ? FILLS : LINES)[name] ?? LINES.black ?? [0, 0, 0];
}

/** The name of the colour in `family` nearest `rgb`, when one is close (a colour this reader wrote). */
export function colourName(rgb: readonly number[], family: "fill" | "line"): CommentColour | undefined {
  if (rgb.length !== 3) return undefined;
  const table = family === "fill" ? FILLS : LINES;
  let best: { name: string; distance: number } | undefined;
  for (const [name, value] of Object.entries(table)) {
    const distance = Math.hypot(...value.map((v, i) => v - (rgb[i] ?? 0)));
    if (best === undefined || distance < best.distance) best = { name, distance };
  }
  return best !== undefined && best.distance < 0.02 ? (best.name as CommentColour) : undefined;
}

/** A page-space point in user space (laminapdf-model's `PageFrame::to_user_space`). */
export function toUser({ crop: [x0, y0, x1, y1], rotate, unit }: Frame, [x, y]: Point): { x: number; y: number } {
  const s = 1 / (unit > 0 && Number.isFinite(unit) ? unit : 1);
  switch ((((rotate % 360) + 360) % 360) as 0 | 90 | 180 | 270) {
    case 90:
      return { x: s * y + x0, y: s * x + y0 };
    case 180:
      return { x: -s * x + x1, y: s * y + y0 };
    case 270:
      return { x: -s * y + x1, y: -s * x + y1 };
    default:
      return { x: s * x + x0, y: -s * y + y1 };
  }
}

/** A page-space box in user space, normalised. */
export function boxToUser(frame: Frame, [x0, y0, x1, y1]: Box): { x0: number; y0: number; x1: number; y1: number } {
  const a = toUser(frame, [x0, y0]);
  const b = toUser(frame, [x1, y1]);
  return { x0: Math.min(a.x, b.x), y0: Math.min(a.y, b.y), x1: Math.max(a.x, b.x), y1: Math.max(a.y, b.y) };
}

/** A box's four corners in user space: its top left, top right, bottom left and bottom right on the page. */
function quad(frame: Frame, [x0, y0, x1, y1]: Box) {
  return [toUser(frame, [x0, y0]), toUser(frame, [x1, y0]), toUser(frame, [x0, y1]), toUser(frame, [x1, y1])];
}

/** The `spec` of `addAnnot` that draws `mark` on a page with `frame`, styled by `style`. */
export function markSpec(mark: Mark, frame: Frame, style: MarkStyle): object {
  const user = (p: Point) => toUser(frame, p);
  const border = { width: style.width };
  switch (mark.type) {
    case "markup":
      return { type: "textMarkup", kind: mark.kind, quads: mark.lines.map((line) => quad(frame, line)) };
    case "note":
      return { type: "note", at: user(mark.at), icon: "note" };
    case "caret":
      return { type: "caret", rect: boxToUser(frame, mark.box) };
    case "ink":
      return {
        type: "ink",
        strokes: mark.strokes.map((points) => ({ points: points.map(user) })),
        width: style.width,
      };
    case "shape":
      return {
        type: "shape",
        kind: mark.kind,
        rect: boxToUser(frame, mark.box),
        border,
        ...(style.fill === undefined ? {} : { interior: [...style.fill] }),
      };
    case "line":
      return {
        type: "line",
        from: user(mark.from),
        to: user(mark.to),
        ends: ["none", mark.arrow ? "openArrow" : "none"],
        border,
      };
    case "poly":
      return {
        type: "poly",
        kind: mark.kind,
        vertices: mark.vertices.map(user),
        border,
        ...(style.fill === undefined || mark.kind !== "polygon" ? {} : { interior: [...style.fill] }),
      };
  }
}

/** A PDF date (`D:YYYYMMDDHHmmSSZ`) for `when`, in UTC. */
export function pdfDate(when: Date): string {
  const two = (n: number) => String(n).padStart(2, "0");
  return `D:${when.getUTCFullYear()}${two(when.getUTCMonth() + 1)}${two(when.getUTCDate())}${two(when.getUTCHours())}${two(
    when.getUTCMinutes(),
  )}${two(when.getUTCSeconds())}Z`;
}

/** The `addAnnot` op that makes `mark` on page `page` (its ID). */
export function addOp(
  page: string,
  mark: Mark,
  frame: Frame,
  style: MarkStyle,
  common: { readonly author?: string; readonly created: Date; readonly contents?: string },
): Op {
  const date = pdfDate(common.created);
  return {
    op: "addAnnot",
    args: {
      page,
      spec: markSpec(mark, frame, style),
      common: {
        color: [...style.colour],
        ...(style.opacity < 1 ? { opacity: style.opacity } : {}),
        ...(common.author === undefined || common.author === "" ? {} : { author: common.author }),
        ...(common.contents === undefined ? {} : { contents: common.contents }),
        created: date,
        modified: date,
      },
    },
  } as unknown as Op;
}

/** CSS for an RGB colour (0 to 1). */
const css = ([r, g, b]: readonly number[]) =>
  `rgb(${Math.round((r ?? 0) * 255)} ${Math.round((g ?? 0) * 255)} ${Math.round((b ?? 0) * 255)})`;

/** Path data through `points`, closed or not. */
function path(points: readonly Point[], closed = false): string {
  const d = points.map(([x, y], i) => `${i === 0 ? "M" : "L"}${x} ${y}`).join(" ");
  return closed ? `${d} Z` : d;
}

/** The corners of `box`, clockwise from the top left. */
const corners = ([x0, y0, x1, y1]: Box): Point[] => [
  [x0, y0],
  [x1, y0],
  [x1, y1],
  [x0, y1],
];

/** What the layer draws while `mark` is being made. */
export function markPreview(mark: Mark, style: MarkStyle): PreviewShape[] {
  const stroke = css(style.colour);
  const line = { stroke, width: style.width, opacity: style.opacity };
  switch (mark.type) {
    case "markup":
      return mark.lines.map((box) => {
        const [x0, y0, x1, y1] = box;
        if (mark.kind === "highlight") return { d: path(corners(box), true), fill: stroke, opacity: 0.5 };
        const y = mark.kind === "strikeOut" ? (y0 + y1) / 2 : y1 - 1;
        return {
          d: path([
            [x0, y],
            [x1, y],
          ]),
          stroke,
          width: 1,
          opacity: style.opacity,
        };
      });
    case "note":
      return [
        { d: path(corners([mark.at[0], mark.at[1], mark.at[0] + 20, mark.at[1] + 20]), true), fill: stroke, ...line },
      ];
    case "caret": {
      const [x0, y0, x1, y1] = mark.box;
      return [
        {
          d: path([
            [x0, y1],
            [(x0 + x1) / 2, y0],
            [x1, y1],
          ]),
          ...line,
        },
      ];
    }
    case "ink":
      return mark.strokes.map((points) => ({ d: path(points), ...line }));
    case "shape": {
      const fill = style.fill === undefined ? {} : { fill: css(style.fill) };
      if (mark.kind === "square") return [{ d: path(corners(mark.box), true), ...line, ...fill }];
      const [x0, y0, x1, y1] = mark.box;
      const [rx, ry, cx, cy] = [(x1 - x0) / 2, (y1 - y0) / 2, (x0 + x1) / 2, (y0 + y1) / 2];
      const d = `M${cx - rx} ${cy} A${rx} ${ry} 0 1 0 ${cx + rx} ${cy} A${rx} ${ry} 0 1 0 ${cx - rx} ${cy} Z`;
      return [{ d, ...line, ...fill }];
    }
    case "line":
      return [{ d: path([mark.from, mark.to]), ...line }, ...arrowHead(mark, line)];
    case "poly": {
      const fill = style.fill === undefined || mark.kind !== "polygon" ? {} : { fill: css(style.fill) };
      return [{ d: path(mark.vertices, mark.kind === "polygon"), ...line, ...fill }];
    }
  }
}

/** An arrow's open head at its end, for the preview. */
function arrowHead(mark: Extract<Mark, { type: "line" }>, line: Omit<PreviewShape, "d">): PreviewShape[] {
  if (!mark.arrow) return [];
  const [[ax, ay], [bx, by]] = [mark.from, mark.to];
  const angle = Math.atan2(by - ay, bx - ax);
  const size = 6 + (line.width ?? 1) * 2;
  const wing = (turn: number): Point => [bx - size * Math.cos(angle + turn), by - size * Math.sin(angle + turn)];
  return [{ d: path([wing(Math.PI / 6), [bx, by], wing(-Math.PI / 6)]), ...line }];
}

/**
 * A drag's box from `from` to `to`; with `square`, a square (or circle) as wide as the drag's
 * larger side, from `from` towards `to`.
 */
export function dragBox(from: Point, to: Point, square: boolean): Box {
  let [dx, dy] = [to[0] - from[0], to[1] - from[1]];
  if (square) {
    const side = Math.max(Math.abs(dx), Math.abs(dy));
    [dx, dy] = [Math.sign(dx || 1) * side, Math.sign(dy || 1) * side];
  }
  const [x1, y1] = [from[0] + dx, from[1] + dy];
  return [Math.min(from[0], x1), Math.min(from[1], y1), Math.max(from[0], x1), Math.max(from[1], y1)];
}

/** `to`, with `snap`, turned to the nearest 45° from `from`. */
export function lineEnd(from: Point, to: Point, snap: boolean): Point {
  if (!snap) return to;
  const [dx, dy] = [to[0] - from[0], to[1] - from[1]];
  const length = Math.hypot(dx, dy);
  const angle = Math.round(Math.atan2(dy, dx) / (Math.PI / 4)) * (Math.PI / 4);
  return [from[0] + length * Math.cos(angle), from[1] + length * Math.sin(angle)];
}

/** A page's text as the text layer reads it: its runs' glyph ranges and every glyph's quad. */
export interface PageText {
  readonly runs: readonly { readonly glyphRange: readonly [number, number]; readonly invisible?: boolean }[];
  /** 8 numbers per glyph, in page space. */
  readonly quads: ArrayLike<number>;
}

/** Each glyph's box, in content order. */
function glyphBoxes(text: PageText): Box[] {
  const boxes: Box[] = [];
  for (const run of text.runs) {
    for (let g = run.glyphRange[0]; g < run.glyphRange[1]; g++) {
      const xs = [0, 2, 4, 6].map((i) => text.quads[g * 8 + i] ?? 0);
      const ys = [1, 3, 5, 7].map((i) => text.quads[g * 8 + i] ?? 0);
      boxes.push([Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)]);
    }
  }
  return boxes;
}

/** The distance from `point` to `box` (0 inside). */
function distance([x, y]: Point, [x0, y0, x1, y1]: Box): number {
  return Math.hypot(Math.max(x0 - x, 0, x - x1), Math.max(y0 - y, 0, y - y1));
}

/** The glyph nearest `point` and its distance, or undefined on a page without text. */
function nearestGlyph(boxes: readonly Box[], point: Point): { index: number; distance: number } | undefined {
  let best: { index: number; distance: number } | undefined;
  boxes.forEach((box, index) => {
    const d = distance(point, box);
    if (best === undefined || d < best.distance) best = { index, distance: d };
  });
  return best;
}

/** Whether two glyph boxes sit on one line: their middles within half the taller one's height. */
function sameLine(a: Box, b: Box): boolean {
  return Math.abs((a[1] + a[3]) / 2 - (b[1] + b[3]) / 2) < Math.max(a[3] - a[1], b[3] - b[1]) / 2;
}

/**
 * The lines of text a drag from `from` to `to` covers, snapped to the glyphs: every glyph from the
 * one nearest the press to the one nearest the release, in content order, that sits between the
 * two glyphs' lines, one box per line. The band keeps out text the content draws between them
 * elsewhere on the page (another column's lines, a later paragraph). Nothing when the press is
 * further than `reach` points from any text.
 */
export function textLines(text: PageText, from: Point, to: Point, reach = 24): Box[] {
  const boxes = glyphBoxes(text);
  const a = nearestGlyph(boxes, from);
  const b = nearestGlyph(boxes, to);
  const [start, end] = [boxes[a?.index ?? -1], boxes[b?.index ?? -1]];
  if (a === undefined || b === undefined || start === undefined || end === undefined || a.distance > reach) return [];
  const [first, last] = a.index <= b.index ? [a.index, b.index] : [b.index, a.index];
  const [top, bottom] = [Math.min(start[1], end[1]), Math.max(start[3], end[3])];
  const inBand = (g: Box) => (g[1] + g[3]) / 2 >= top && (g[1] + g[3]) / 2 <= bottom;
  return linesOf(boxes.slice(first, last + 1).filter(inBand));
}

/** Glyph boxes joined into one box per line. */
function linesOf(boxes: readonly Box[]): Box[] {
  const lines: Box[] = [];
  for (const box of boxes) {
    const at = lines.findIndex((line) => sameLine(line, box));
    const line = lines[at];
    if (line === undefined) lines.push(box);
    else
      lines[at] = [
        Math.min(line[0], box[0]),
        Math.min(line[1], box[1]),
        Math.max(line[2], box[2]),
        Math.max(line[3], box[3]),
      ];
  }
  return lines;
}

/** Selection boxes (a text selection's lines) snapped to the glyphs whose middles they hold. */
export function snapLines(text: PageText, lines: readonly Box[]): Box[] {
  const inside = glyphBoxes(text).filter((g) => {
    const [cx, cy] = [(g[0] + g[2]) / 2, (g[1] + g[3]) / 2];
    return lines.some(([x0, y0, x1, y1]) => cx >= x0 && cx <= x1 && cy >= y0 && cy <= y1);
  });
  return inside.length === 0 ? [...lines] : linesOf(inside);
}

/**
 * Where a caret goes for a click at `point`: on the nearest glyph's line, at the edge between two
 * glyphs nearest the click, as tall as the line's lower half. A point far from any text gets a
 * caret 12 pt tall where it was clicked.
 */
export function caretBox(text: PageText, point: Point, reach = 24): Box {
  const boxes = glyphBoxes(text);
  const near = nearestGlyph(boxes, point);
  const glyph = near === undefined || near.distance > reach ? undefined : boxes[near.index];
  if (glyph === undefined) return [point[0] - 4, point[1] - 6, point[0] + 4, point[1] + 6];
  const [x0, y0, x1, y1] = glyph;
  const x = Math.abs(point[0] - x0) < Math.abs(point[0] - x1) ? x0 : x1;
  const height = y1 - y0;
  const half = height / 3;
  return [x - half, y0 + height / 2, x + half, y1 + height / 6];
}

/** Whether `point` is inside `mark`'s filled area: a rectangle, an ellipse or a polygon. */
function insideShape(mark: Mark, [x, y]: Point): boolean {
  if (mark.type === "shape") {
    const [x0, y0, x1, y1] = mark.box;
    if (mark.kind === "square") return x >= x0 && x <= x1 && y >= y0 && y <= y1;
    const [rx, ry] = [(x1 - x0) / 2, (y1 - y0) / 2];
    const [dx, dy] = [(x - (x0 + rx)) / (rx || 1), (y - (y0 + ry)) / (ry || 1)];
    return dx * dx + dy * dy <= 1;
  }
  return mark.type === "poly" && mark.kind === "polygon" && insidePolygon(mark.vertices, [x, y]);
}

/** Whether `point` is inside the polygon through `v`, by even-odd ray casting. */
function insidePolygon(v: readonly Point[], [x, y]: Point): boolean {
  let inside = false;
  for (let i = 0, j = v.length - 1; i < v.length; j = i++) {
    const [xi, yi] = v[i] ?? [0, 0];
    const [xj, yj] = v[j] ?? [0, 0];
    if (yi > y !== yj > y && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) inside = !inside;
  }
  return inside;
}

/**
 * Whether `mark`, drawn with `style`, hides text the page still holds: a filled rectangle, ellipse
 * or polygon at full opacity with the middle of a visible glyph inside it. Text under a box stays
 * in the file and can be copied, the classic failed redaction.
 */
export function coversText(mark: Mark, style: MarkStyle, text: PageText): boolean {
  if (style.fill === undefined || style.opacity < 1) return false;
  if (mark.type !== "shape" && !(mark.type === "poly" && mark.kind === "polygon")) return false;
  for (const run of text.runs) {
    if (run.invisible === true) continue;
    const boxes = glyphBoxes({ runs: [run], quads: text.quads });
    if (boxes.some(([x0, y0, x1, y1]) => insideShape(mark, [(x0 + x1) / 2, (y0 + y1) / 2]))) return true;
  }
  return false;
}

/** The subtypes Comment mode resizes, and what each one's resize patches. */
const RESIZABLE = new Set(["Square", "Circle", "Ink", "Line", "Polygon", "PolyLine", "Caret"]);

/** Whether Comment mode can resize `annotation`. */
export function canResize(annotation: PageAnnotation): boolean {
  return RESIZABLE.has(annotation.subtype);
}

/** Flat `[x, y, …]` as points in user space. */
function usersOf(frame: Frame, flat: readonly number[]): { x: number; y: number }[] {
  const out: { x: number; y: number }[] = [];
  for (let i = 0; i + 1 < flat.length; i += 2) out.push(toUser(frame, [flat[i] ?? 0, flat[i + 1] ?? 0]));
  return out;
}

/**
 * The edit that gives `annotation` the shape of `resized` (the layer's stretched copy): its
 * subtype's spec patch, its geometry in user space. Undefined for a kind that can't be resized.
 */
export function resizeOp(annotation: PageAnnotation, resized: PageAnnotation, frame: Frame): Op | undefined {
  const g = resized.geometry;
  let spec: object | undefined;
  switch (annotation.subtype) {
    case "Square":
    case "Circle": {
      const box = g.kind === "inner" ? g.rect : resized.rect;
      if (box !== undefined) spec = { type: "shape", rect: boxToUser(frame, box) };
      break;
    }
    case "Caret":
      if (resized.rect !== undefined) spec = { type: "caret", rect: boxToUser(frame, resized.rect) };
      break;
    case "Ink":
      if (g.kind === "ink") spec = { type: "ink", strokes: g.strokes.map((s) => ({ points: usersOf(frame, s) })) };
      break;
    case "Line":
      if (g.kind === "line") {
        const [from, to] = usersOf(frame, g.points);
        spec = { type: "line", from, to };
      }
      break;
    case "Polygon":
    case "PolyLine":
      if (g.kind === "vertices") spec = { type: "poly", vertices: usersOf(frame, g.points) };
      break;
    default:
      break;
  }
  return spec === undefined ? undefined : ({ op: "updateAnnot", args: { id: annotation.id, spec } } as unknown as Op);
}

/** A subtype's spec patch type, for the inspector's changes to fill, border, endings and smoothing. */
export function patchType(subtype: string): "shape" | "line" | "poly" | "ink" | undefined {
  switch (subtype) {
    case "Square":
    case "Circle":
      return "shape";
    case "Line":
      return "line";
    case "Polygon":
    case "PolyLine":
      return "poly";
    case "Ink":
      return "ink";
    default:
      return undefined;
  }
}

/** A border style's wire form, as the drawing specs take it. */
export function borderStyle(style: "solid" | "dashed" | "cloudy"): object {
  switch (style) {
    case "dashed":
      return { type: "dashed", pattern: [3, 2] };
    case "cloudy":
      return { type: "cloudy", intensity: 1 };
    default:
      return { type: "solid" };
  }
}

/** The line endings of an arrow's new mark: none at its start, an open arrow at its end. */
export const ARROW_ENDS: readonly [LineEndingName, LineEndingName] = ["none", "openArrow"];
