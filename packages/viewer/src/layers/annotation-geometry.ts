// The annotation layer's sums, kept apart from the DOM so they can be tested in Node: which
// annotations the layer shows and in what order, which one a point hits, the selection's box and
// handles, a pointer's move in page points, that move in user space for `updateAnnot`, and the
// SVG paths that preview an annotation while a gesture moves it. Everything is in page space
// (points, y down, the page's `/Rotate` applied), as the annotation records are.

import type { PageAnnotation } from "@laminapdf/sdk";
import type { Rotation } from "../layout/page-layout.js";

/** `[x0, y0, x1, y1]` in page points, top-left and bottom-right. */
export type Box = readonly [number, number, number, number];

/** A move in page points (y down). */
export interface Move {
  readonly dx: number;
  readonly dy: number;
}

/** One SVG path of a preview, in page points. */
export interface PreviewShape {
  /** SVG path data. */
  readonly d: string;
  /** The stroke colour (CSS); none without. */
  readonly stroke?: string;
  /** The stroke width in points (default 1). */
  readonly width?: number;
  /** The fill colour (CSS); none without. */
  readonly fill?: string;
  /** 0 to 1 (default 1). */
  readonly opacity?: number;
}

// ISO 32000-2 Table 167.
const HIDDEN = 1 << 1;
const NO_VIEW = 1 << 5;
const LOCKED = 1 << 7;

/** Subtypes the layer leaves to others: pop-ups go with their parent, links and widgets have layers of their own. */
const NOT_SHOWN = new Set(["Popup", "Link", "Widget"]);

/** The annotation's rectangle, normalised, if it has one. */
function boxOf(annotation: PageAnnotation): Box | undefined {
  const r = annotation.rect;
  if (r === undefined) return undefined;
  const [x0, y0, x1, y1] = r;
  return [Math.min(x0, x1), Math.min(y0, y1), Math.max(x0, x1), Math.max(y0, y1)];
}

/** An annotation the layer shows, with its rectangle. */
export interface ShownAnnotation {
  readonly annotation: PageAnnotation;
  readonly box: Box;
}

/**
 * The annotations the layer shows, each with its rectangle, in reading order: top to bottom, then
 * left to right, so Tab moves through them as a reader does. Pop-ups, links, widgets, those
 * without a rectangle and those the file hides from view are left out.
 */
export function shownAnnotations(annotations: readonly PageAnnotation[]): ShownAnnotation[] {
  const shown: ShownAnnotation[] = [];
  for (const annotation of annotations) {
    const box = boxOf(annotation);
    if (box === undefined || NOT_SHOWN.has(annotation.subtype) || (annotation.flags & (HIDDEN | NO_VIEW)) !== 0)
      continue;
    shown.push({ annotation, box });
  }
  // Tops within 2 pt are one line, read left to right.
  return shown.sort(({ box: [ax, ay] }, { box: [bx, by] }) => (Math.abs(ay - by) > 2 ? ay - by : ax - bx));
}

/** Whether the file locks the annotation against moves and changes (`Locked`). */
export function isLocked(annotation: PageAnnotation): boolean {
  return (annotation.flags & LOCKED) !== 0;
}

/** The point pairs of a flat `[x, y, …]` list. */
function pairs(flat: readonly number[]): [number, number][] {
  const out: [number, number][] = [];
  for (let i = 0; i + 1 < flat.length; i += 2) out.push([flat[i] ?? 0, flat[i + 1] ?? 0]);
  return out;
}

/** The distance from (`x`, `y`) to the segment from `a` to `b`. */
function toSegment(x: number, y: number, [ax, ay]: [number, number], [bx, by]: [number, number]): number {
  const [vx, vy] = [bx - ax, by - ay];
  const length = vx * vx + vy * vy;
  const t = length === 0 ? 0 : Math.max(0, Math.min(1, ((x - ax) * vx + (y - ay) * vy) / length));
  return Math.hypot(x - (ax + t * vx), y - (ay + t * vy));
}

/** Whether (`x`, `y`) is inside `box`, grown by `slop` on every side. */
function inBox(x: number, y: number, [x0, y0, x1, y1]: Box, slop = 0): boolean {
  return x >= x0 - slop && x <= x1 + slop && y >= y0 - slop && y <= y1 + slop;
}

/** The point lists the annotation draws as lines (ink strokes, a line, a polyline's vertices). */
function lines(annotation: PageAnnotation): { points: [number, number][]; closed: boolean }[] {
  const g = annotation.geometry;
  switch (g.kind) {
    case "ink":
      return g.strokes.map((stroke) => ({ points: pairs(stroke), closed: false }));
    case "line":
      return [{ points: pairs(g.points), closed: false }];
    case "vertices":
      return [{ points: pairs(g.points), closed: annotation.subtype === "Polygon" }];
    default:
      return [];
  }
}

/**
 * Whether (`x`, `y`) hits the annotation: inside a markup's areas, within `slop` points of an ink
 * stroke, a line or a polyline (a filled polygon anywhere inside its box), else inside its
 * rectangle.
 */
function hits(annotation: PageAnnotation, x: number, y: number, slop: number): boolean {
  const box = boxOf(annotation);
  if (box === undefined || !inBox(x, y, box, slop)) return false;
  const g = annotation.geometry;
  if (g.kind === "quads") {
    for (let at = 0; at + 8 <= g.quads.length; at += 8) {
      const corners = pairs(g.quads.slice(at, at + 8));
      const xs = corners.map(([cx]) => cx);
      const ys = corners.map(([, cy]) => cy);
      if (inBox(x, y, [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)], slop)) return true;
    }
    return false;
  }
  const drawn = lines(annotation);
  if (drawn.length === 0 || annotation.subtype === "Polygon") return true;
  return drawn.some(({ points, closed }) =>
    points.some((p, i) => {
      const next = points[i + 1] ?? (closed ? points[0] : undefined);
      return next === undefined ? Math.hypot(x - p[0], y - p[1]) <= slop : toSegment(x, y, p, next) <= slop;
    }),
  );
}

/**
 * The annotation at (`x`, `y`) among `annotations`: the last one in the page's `/Annots` that
 * the point hits, since later annotations are drawn over earlier ones; undefined for none.
 */
export function hitTest(
  annotations: readonly PageAnnotation[],
  x: number,
  y: number,
  slop = 3,
): PageAnnotation | undefined {
  let found: PageAnnotation | undefined;
  for (const a of annotations) {
    if (hits(a, x, y, slop) && (found === undefined || a.index > found.index)) found = a;
  }
  return found;
}

/** The box around every one of `annotations`, moved by `move`; undefined for none. */
export function unionBox(annotations: readonly PageAnnotation[], move: Move = { dx: 0, dy: 0 }): Box | undefined {
  const boxes = annotations.map(boxOf).filter((b) => b !== undefined);
  if (boxes.length === 0) return undefined;
  return [
    Math.min(...boxes.map((b) => b[0])) + move.dx,
    Math.min(...boxes.map((b) => b[1])) + move.dy,
    Math.max(...boxes.map((b) => b[2])) + move.dx,
    Math.max(...boxes.map((b) => b[3])) + move.dy,
  ];
}

/** Where the eight handles sit on `box` (design language §7.32): corners first, then the edges' middles. */
export function handlePoints([x0, y0, x1, y1]: Box): [number, number][] {
  const [mx, my] = [(x0 + x1) / 2, (y0 + y1) / 2];
  return [
    [x0, y0],
    [x1, y0],
    [x1, y1],
    [x0, y1],
    [mx, y0],
    [x1, my],
    [mx, y1],
    [x0, my],
  ];
}

/**
 * A pointer's move of (`sx`, `sy`) CSS px, on a page shown at `scale` CSS px per point turned
 * `rotation` degrees clockwise, as a move in page points.
 */
export function pageMove(sx: number, sy: number, scale: number, rotation: Rotation): Move {
  const [x, y] = [sx / scale, sy / scale];
  switch (rotation) {
    case 90:
      return { dx: y, dy: -x };
    case 180:
      return { dx: -x, dy: -y };
    case 270:
      return { dx: -y, dy: x };
    default:
      return { dx: x, dy: y };
  }
}

/**
 * A move in page points as a move in the page's user space (y up, `/Rotate` undone), as
 * `updateAnnot`'s `translate` takes it, on a page whose `/Rotate` is `rotate` degrees.
 */
export function userMove({ dx, dy }: Move, rotate: number): { dx: number; dy: number } {
  // Page space is user space flipped (y down), then turned by /Rotate clockwise.
  switch ((((rotate % 360) + 360) % 360) as Rotation) {
    case 90:
      return { dx: dy, dy: dx };
    case 180:
      return { dx: -dx, dy };
    case 270:
      return { dx: -dy, dy: -dx };
    default:
      return { dx, dy: -dy };
  }
}

/** The annotation's colour as CSS (`/C`: gray, RGB or CMYK, 0 to 1), or `fallback` without one. */
function cssColour(colour: readonly number[], fallback: string): string {
  const byte = (v: number) => Math.round(Math.max(0, Math.min(1, v)) * 255);
  const [a = 0, b = 0, c = 0, k = 0] = colour;
  switch (colour.length) {
    case 1:
      return `rgb(${byte(a)} ${byte(a)} ${byte(a)})`;
    case 3:
      return `rgb(${byte(a)} ${byte(b)} ${byte(c)})`;
    case 4:
      return `rgb(${byte((1 - a) * (1 - k))} ${byte((1 - b) * (1 - k))} ${byte((1 - c) * (1 - k))})`;
    default:
      return fallback;
  }
}

/** Path data through `points`, closed or not, moved by `move`. */
function path(points: readonly [number, number][], closed: boolean, { dx, dy }: Move): string {
  const d = points.map(([x, y], i) => `${i === 0 ? "M" : "L"}${x + dx} ${y + dy}`).join(" ");
  return closed ? `${d} Z` : d;
}

/** Each corner box of a markup's areas: eight numbers per area, in either corner order. */
function areaBoxes(quads: readonly number[]): Box[] {
  const boxes: Box[] = [];
  for (let at = 0; at + 8 <= quads.length; at += 8) {
    const corners = pairs(quads.slice(at, at + 8));
    const xs = corners.map(([x]) => x);
    const ys = corners.map(([, y]) => y);
    boxes.push([Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)]);
  }
  return boxes;
}

/**
 * A markup's preview: a highlight fills each area, at most half opaque so the words show; an
 * underline, a squiggly underline and a strike-out draw one line across each.
 */
function markupPreview(
  annotation: PageAnnotation,
  quads: readonly number[],
  move: Move,
  { colour, opacity }: { readonly colour: string; readonly opacity: number },
): PreviewShape[] {
  return areaBoxes(quads).map(([x0, y0, x1, y1]): PreviewShape => {
    if (annotation.subtype === "Highlight") {
      const corners: [number, number][] = [
        [x0, y0],
        [x1, y0],
        [x1, y1],
        [x0, y1],
      ];
      return { d: path(corners, true, move), fill: colour, opacity: Math.min(opacity, 0.5) };
    }
    const y = annotation.subtype === "StrikeOut" ? (y0 + y1) / 2 : y1 - 1;
    const line: [number, number][] = [
      [x0, y],
      [x1, y],
    ];
    return { d: path(line, false, move), stroke: colour, width: 1, opacity };
  });
}

/**
 * What a gesture draws for `annotation` moved by `move`: a markup's areas filled, ink, lines and
 * polygons stroked, a circle as an ellipse and anything else as its rectangle outlined, in its
 * own colour and opacity (`fallback` without a colour).
 */
export function annotationPreview(annotation: PageAnnotation, move: Move, fallback = "#1a73e8"): PreviewShape[] {
  const colour = cssColour(annotation.color, fallback);
  const opacity = annotation.opacity ?? 1;
  const g = annotation.geometry;
  if (g.kind === "quads" && g.quads.length >= 8) return markupPreview(annotation, g.quads, move, { colour, opacity });
  const drawn = lines(annotation);
  if (drawn.length > 0) {
    return drawn.map(({ points, closed }) => ({ d: path(points, closed, move), stroke: colour, width: 1.5, opacity }));
  }
  // A square or circle runs along its inner box, inside the rectangle its stroke widens.
  const box = g.kind === "inner" ? g.rect : boxOf(annotation);
  if (box === undefined) return [];
  const [x0, y0, x1, y1] = box;
  if (annotation.subtype === "Circle") {
    const [rx, ry] = [(x1 - x0) / 2, (y1 - y0) / 2];
    const [cx, cy] = [x0 + rx + move.dx, y0 + ry + move.dy];
    const d = `M${cx - rx} ${cy} A${rx} ${ry} 0 1 0 ${cx + rx} ${cy} A${rx} ${ry} 0 1 0 ${cx - rx} ${cy} Z`;
    return [{ d, stroke: colour, width: 1.5, opacity }];
  }
  const outline: [number, number][] = [
    [x0, y0],
    [x1, y0],
    [x1, y1],
    [x0, y1],
  ];
  return [{ d: path(outline, true, move), stroke: colour, width: 1.5, opacity }];
}

/** A page's frame: its crop box in user space, its `/Rotate` in degrees and its `/UserUnit`. */
export interface PageFrame {
  readonly crop: readonly [number, number, number, number];
  readonly rotate: number;
  readonly unit: number;
}

/**
 * A page-space point (ADR 0027) in user space, as laminapdf-model's `PageFrame::to_user_space`
 * maps it; fixtures/geometry/page-to-user.json holds both to the same cases.
 */
export function pageToUser({ crop: [x0, y0, x1, y1], rotate, unit }: PageFrame, [x, y]: readonly [number, number]) {
  const s = 1 / (unit > 0 && Number.isFinite(unit) ? unit : 1);
  switch ((((rotate % 360) + 360) % 360) as Rotation) {
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

/** The handle (handlePoints' order) at the bottom right of the screen when the view is turned `rotation` clockwise. */
export function screenBottomRight(rotation: Rotation): number {
  // Turned a quarter clockwise, the page's top right shows at the bottom right, and so on round.
  return ({ 0: 2, 90: 1, 180: 0, 270: 3 } as const)[rotation];
}

/** The smallest side a resize leaves, in points. */
const SMALLEST_SIDE = 2;

/**
 * `box` with handle `handle` (`handlePoints`' order: the corners from the top left clockwise,
 * then the edges' middles from the top) moved by `move`. A corner moves both edges it joins, an
 * edge's middle moves that edge alone. With `keep`, the box keeps its proportions: a corner scales
 * it about the opposite corner, an edge about the middle of the edge across. The box never turns
 * inside out: each side stays at least 2 points.
 */
export function resizeBox(box: Box, handle: number, move: Move, keep = false): Box {
  const [x0, y0, x1, y1] = box;
  const left = handle === 0 || handle === 3 || handle === 7;
  const right = handle === 1 || handle === 2 || handle === 5;
  const top = handle === 0 || handle === 1 || handle === 4;
  const bottom = handle === 2 || handle === 3 || handle === 6;
  let [a, b, c, d] = [x0, y0, x1, y1];
  if (left) a = Math.min(x0 + move.dx, x1 - SMALLEST_SIDE);
  if (right) c = Math.max(x1 + move.dx, x0 + SMALLEST_SIDE);
  if (top) b = Math.min(y0 + move.dy, y1 - SMALLEST_SIDE);
  if (bottom) d = Math.max(y1 + move.dy, y0 + SMALLEST_SIDE);
  return keep
    ? keepProportions(box, [a, b, c, d], { left, top, sideways: left || right, upright: top || bottom })
    : [a, b, c, d];
}

/**
 * `asked` (`box` resized from a handle on the sides `from` names) scaled back to `box`'s
 * proportions: about the opposite corner for a corner, about the middle across for an edge.
 */
function keepProportions(
  box: Box,
  [a, b, c, d]: Box,
  from: { readonly left: boolean; readonly top: boolean; readonly sideways: boolean; readonly upright: boolean },
): Box {
  const [x0, y0, x1, y1] = box;
  const { left, top, sideways, upright } = from;
  const [width, height] = [x1 - x0, y1 - y0];
  // The scale the handle asks for: the larger change for a corner, the edge's own for an edge.
  // A side with no length (a flat line's box) asks for none, as mapPoints stretches nothing there.
  const asks = [
    sideways && width > 0 ? (c - a) / width : undefined,
    upright && height > 0 ? (d - b) / height : undefined,
  ];
  const scales = asks.filter((v): v is number => v !== undefined && Number.isFinite(v));
  if (scales.length === 0) return [a, b, c, d];
  const scale = Math.max(...scales);
  const [nx0, nx1] = stretch(x0, x1, width * scale, sideways, left);
  const [ny0, ny1] = stretch(y0, y1, height * scale, upright, top);
  return [nx0, ny0, nx1, ny1];
}

/**
 * The span from `lo` to `hi` given `size`: from the far end when `moved` (`fromLo`: the low end
 * moved, so the high end stays), else about its middle.
 */
function stretch(lo: number, hi: number, size: number, moved: boolean, fromLo: boolean): [number, number] {
  if (!moved) return [(lo + hi) / 2 - size / 2, (lo + hi) / 2 + size / 2];
  return fromLo ? [hi - size, hi] : [lo, lo + size];
}

/** Each `[x, y]` pair of `flat` mapped from box `from` onto box `to`. */
function mapPoints(flat: readonly number[], from: Box, to: Box): number[] {
  const sx = (to[2] - to[0]) / (from[2] - from[0] || 1);
  const sy = (to[3] - to[1]) / (from[3] - from[1] || 1);
  return flat.map((v, i) => (i % 2 === 0 ? to[0] + (v - from[0]) * sx : to[1] + (v - from[1]) * sy));
}

/**
 * `annotation` as it would be resized from box `from` onto box `to` (page points): its rectangle
 * and its own geometry stretched with the box, for a resize's preview and its edit.
 */
export function resizedAnnotation(annotation: PageAnnotation, from: Box, to: Box): PageAnnotation {
  const g = annotation.geometry;
  const rect = annotation.rect === undefined ? undefined : mapPoints(annotation.rect, from, to);
  let geometry: PageAnnotation["geometry"];
  switch (g.kind) {
    case "quads":
      geometry = { ...g, quads: mapPoints(g.quads, from, to) };
      break;
    case "ink":
      geometry = { ...g, strokes: g.strokes.map((s) => mapPoints(s, from, to)) };
      break;
    case "line":
    case "vertices":
    case "callout":
      geometry = { ...g, points: mapPoints(g.points, from, to) };
      break;
    case "inner":
      geometry = { ...g, rect: mapPoints(g.rect, from, to) as [number, number, number, number] };
      break;
    default:
      geometry = g;
  }
  return { ...annotation, ...(rect === undefined ? {} : { rect: rect as [number, number, number, number] }), geometry };
}
