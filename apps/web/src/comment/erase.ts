// The Eraser's and the Lasso's geometry (C08, C09), apart from the DOM so it runs in Node: which
// ink strokes a path crosses, and which annotations a closed path holds. Points are page points.

import type { Point } from "./marks.ts";

/** How near, in points, the eraser's path must pass to a stroke to take it. */
export const ERASE_REACH = 6;

/** The distance from `p` to the segment `a`–`b`. */
function toSegment(p: Point, a: Point, b: Point): number {
  const dx = b[0] - a[0];
  const dy = b[1] - a[1];
  const length = dx * dx + dy * dy;
  const t = length === 0 ? 0 : Math.min(1, Math.max(0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / length));
  return Math.hypot(p[0] - (a[0] + t * dx), p[1] - (a[1] + t * dy));
}

/** A flat `[x, y, …]` list as points. */
export function pointsOf(flat: readonly number[]): Point[] {
  const out: Point[] = [];
  for (let i = 0; i + 1 < flat.length; i += 2) out.push([flat[i] ?? 0, flat[i + 1] ?? 0]);
  return out;
}

/** The segments of a polyline; a single point is a segment to itself. */
function segments(points: readonly Point[]): [Point, Point][] {
  if (points.length === 1 && points[0] !== undefined) return [[points[0], points[0]]];
  const out: [Point, Point][] = [];
  for (let i = 1; i < points.length; i++) {
    const a = points[i - 1];
    const b = points[i];
    if (a !== undefined && b !== undefined) out.push([a, b]);
  }
  return out;
}

/** Which side of the line `a`–`b` the point `p` is on: positive, negative, or 0 on it. */
function side(a: Point, b: Point, p: Point): number {
  return (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
}

/** Whether segments `a`–`b` and `c`–`d` cross. */
function cross([a, b]: [Point, Point], [c, d]: [Point, Point]): boolean {
  return side(a, b, c) * side(a, b, d) < 0 && side(c, d, a) * side(c, d, b) < 0;
}

/**
 * Whether polylines `a` and `b` pass within `reach` of each other: a segment of one crosses a
 * segment of the other, or a point of one is near a segment of the other.
 */
function near(a: readonly Point[], b: readonly Point[], reach: number): boolean {
  const as = segments(a);
  const bs = segments(b);
  const close = (points: readonly Point[], lines: [Point, Point][]) =>
    points.some((p) => lines.some(([s, e]) => toSegment(p, s, e) <= reach));
  return as.some((x) => bs.some((y) => cross(x, y))) || close(a, bs) || close(b, as);
}

/** The indexes of the `strokes` that `path` passes within `reach` of. */
export function strokesHit(
  strokes: readonly (readonly Point[])[],
  path: readonly Point[],
  reach = ERASE_REACH,
): number[] {
  return strokes.flatMap((stroke, i) => (stroke.length > 0 && path.length > 0 && near(stroke, path, reach) ? [i] : []));
}

/** Whether `p` is inside the polygon `ring` (even-odd). */
export function inside(p: Point, ring: readonly Point[]): boolean {
  let within = false;
  for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
    const a = ring[i];
    const b = ring[j];
    if (a === undefined || b === undefined) continue;
    if (a[1] > p[1] !== b[1] > p[1] && p[0] < ((b[0] - a[0]) * (p[1] - a[1])) / (b[1] - a[1]) + a[0]) within = !within;
  }
  return within;
}

/** Whether the box `[x0, y0, x1, y1]` lies wholly inside the closed path `ring`. */
export function boxInside(box: readonly [number, number, number, number], ring: readonly Point[]): boolean {
  const [x0, y0, x1, y1] = box;
  const corners: Point[] = [
    [x0, y0],
    [x1, y0],
    [x1, y1],
    [x0, y1],
  ];
  return ring.length >= 3 && corners.every((c) => inside(c, ring));
}
