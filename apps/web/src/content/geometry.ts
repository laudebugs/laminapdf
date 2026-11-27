// The geometry of objects Lamina PDF added to a page (Paper A02), kept apart from the DOM so Node
// can test it: an object's frame (the matrix that maps its unit square to page space, y down),
// hit testing, resizing and turning by a handle, snapping to the margins, the page's centre and
// other objects, and keyboard nudging. Everything is in page space, points from the top-left of
// the page as shown (ADR 0027).

/** `[a, b, c, d, e, f]`: x' = a x + c y + e, y' = b x + d y + f. */
export type Matrix = readonly [number, number, number, number, number, number];

/** A point in page space. */
export interface Point {
  readonly x: number;
  readonly y: number;
}

/** A box in page space: left, top, right, bottom. */
export type Box = readonly [number, number, number, number];

/** Where a handle sits on a frame: the sign of each axis it moves (`0` for none). */
export type HandleName = "nw" | "n" | "ne" | "e" | "se" | "s" | "sw" | "w";

/** Each handle's pull on the frame's width and height. */
const PULLS: Readonly<Record<HandleName, readonly [-1 | 0 | 1, -1 | 0 | 1]>> = {
  nw: [-1, -1],
  n: [0, -1],
  ne: [1, -1],
  e: [1, 0],
  se: [1, 1],
  s: [0, 1],
  sw: [-1, 1],
  w: [-1, 0],
};

/** The handles of a frame, in the order they are drawn. */
export const HANDLES = Object.keys(PULLS) as readonly HandleName[];

/** The handles on a text box: its sides set the width only (Paper A01). */
export const TEXT_HANDLES: readonly HandleName[] = ["w", "e"];

/** The identity. */
export const IDENTITY: Matrix = [1, 0, 0, 1, 0, 0];

/** `m` applied to `p`. */
export function apply(m: Matrix, p: Point): Point {
  return { x: m[0] * p.x + m[2] * p.y + m[4], y: m[1] * p.x + m[3] * p.y + m[5] };
}

/** `outer` after `inner`: the matrix that does `inner` first. */
export function multiply(outer: Matrix, inner: Matrix): Matrix {
  const [a, b, c, d, e, f] = outer;
  const [g, h, i, j, k, l] = inner;
  return [a * g + c * h, b * g + d * h, a * i + c * j, b * i + d * j, a * k + c * l + e, b * k + d * l + f];
}

/** The matrix that undoes `m`; undefined for one that collapses the plane. */
export function invert(m: Matrix): Matrix | undefined {
  const [a, b, c, d, e, f] = m;
  const det = a * d - b * c;
  if (!Number.isFinite(det) || Math.abs(det) < 1e-12) return undefined;
  return [d / det, -b / det, -c / det, a / det, (c * f - d * e) / det, (b * e - a * f) / det];
}

/** The frame that is a `width` by `height` rectangle with its top-left corner at (`x`, `y`), turned `degrees` about its centre. */
export function frameOf(x: number, y: number, width: number, height: number, degrees = 0): Matrix {
  const t = (degrees * Math.PI) / 180;
  const cos = Math.cos(t);
  const sin = Math.sin(t);
  const a = width * cos;
  const b = width * sin || 0;
  // `|| 0` keeps an unturned frame's zeros positive.
  const c = -height * sin || 0;
  const d = height * cos;
  // The centre stays where the unturned rectangle's is.
  const cx = x + width / 2;
  const cy = y + height / 2;
  return [a, b, c, d, cx - (a + c) / 2, cy - (b + d) / 2];
}

/** A frame's width and height in points, and how far it is turned (degrees clockwise, as y runs down). */
export function sizeOf(frame: Matrix): { readonly width: number; readonly height: number; readonly degrees: number } {
  const width = Math.hypot(frame[0], frame[1]);
  const height = Math.hypot(frame[2], frame[3]);
  const degrees = (Math.atan2(frame[1], frame[0]) * 180) / Math.PI;
  return { width, height, degrees: Math.round(degrees * 100) / 100 };
}

/** The corners of a frame, top-left, top-right, bottom-right, bottom-left. */
export function corners(frame: Matrix): readonly [Point, Point, Point, Point] {
  return [
    apply(frame, { x: 0, y: 0 }),
    apply(frame, { x: 1, y: 0 }),
    apply(frame, { x: 1, y: 1 }),
    apply(frame, { x: 0, y: 1 }),
  ];
}

/** The page-space box around `points`. */
export function boxAround(points: readonly Point[]): Box {
  const xs = points.map((p) => p.x);
  const ys = points.map((p) => p.y);
  return [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)];
}

/** The centre of a frame. */
export function centreOf(frame: Matrix): Point {
  return apply(frame, { x: 0.5, y: 0.5 });
}

/** Whether `point` is inside `frame`, `slop` points outside counting. */
export function contains(frame: Matrix, point: Point, slop = 0): boolean {
  const inverse = invert(frame);
  if (inverse === undefined) return false;
  const { width, height } = sizeOf(frame);
  const local = apply(inverse, point);
  const sx = width === 0 ? 0 : slop / width;
  const sy = height === 0 ? 0 : slop / height;
  return local.x >= -sx && local.x <= 1 + sx && local.y >= -sy && local.y <= 1 + sy;
}

/** A handle's place on `frame`, in page space. */
export function handlePoint(frame: Matrix, handle: HandleName): Point {
  const [sx, sy] = PULLS[handle];
  return apply(frame, { x: 0.5 + sx / 2, y: 0.5 + sy / 2 });
}

/** The point the turning handle sits at: `reach` points beyond the middle of the frame's top edge, along its up. */
export function turnPoint(frame: Matrix, reach: number): Point {
  const { height } = sizeOf(frame);
  const top = apply(frame, { x: 0.5, y: 0 });
  const centre = centreOf(frame);
  const length = Math.hypot(top.x - centre.x, top.y - centre.y) || height / 2 || 1;
  return { x: top.x + ((top.x - centre.x) / length) * reach, y: top.y + ((top.y - centre.y) / length) * reach };
}

/** Moves a frame by (`dx`, `dy`). */
export function moved(frame: Matrix, dx: number, dy: number): Matrix {
  return [frame[0], frame[1], frame[2], frame[3], frame[4] + dx, frame[5] + dy];
}

/** The smallest a side gets by a handle, in points. */
export const SMALLEST_SIDE = 4;

/**
 * `frame` with `handle` dragged by (`dx`, `dy`) in page space: the opposite side or corner stays
 * where it is, the sides follow the frame's own turn, and `keepRatio` holds the shape on a corner.
 */
export function resized(frame: Matrix, handle: HandleName, dx: number, dy: number, keepRatio = false): Matrix {
  const { width, height } = sizeOf(frame);
  if (width === 0 || height === 0) return frame;
  const [sx, sy] = PULLS[handle];
  // The drag along the frame's own axes.
  const along = (dx * frame[0] + dy * frame[1]) / width;
  const down = (dx * frame[2] + dy * frame[3]) / height;
  let w = Math.max(SMALLEST_SIDE, width + sx * along);
  let h = Math.max(SMALLEST_SIDE, height + sy * down);
  if (sx === 0) w = width;
  if (sy === 0) h = height;
  if (keepRatio && sx !== 0 && sy !== 0) {
    const scale = Math.max(w / width, h / height);
    w = width * scale;
    h = height * scale;
  }
  const anchor = { x: 0.5 - sx / 2, y: 0.5 - sy / 2 };
  const fixed = apply(frame, anchor);
  const a = (frame[0] / width) * w;
  const b = (frame[1] / width) * w;
  const c = (frame[2] / height) * h;
  const d = (frame[3] / height) * h;
  return [a, b, c, d, fixed.x - a * anchor.x - c * anchor.y, fixed.y - b * anchor.x - d * anchor.y];
}

/** `frame` turned about its centre to point at `to`, as the turning handle drags it; `snap` rounds to 15 degrees. */
export function turnedTo(frame: Matrix, to: Point, snap = false): Matrix {
  const { width, height } = sizeOf(frame);
  const centre = centreOf(frame);
  // The handle sits above the frame, so straight up is no turn.
  let degrees = (Math.atan2(to.y - centre.y, to.x - centre.x) * 180) / Math.PI + 90;
  if (snap) degrees = Math.round(degrees / 15) * 15;
  degrees = ((((degrees + 180) % 360) + 360) % 360) - 180;
  return frameOf(centre.x - width / 2, centre.y - height / 2, width, height, degrees);
}

/** `frame` set to `width` by `height`, keeping its centre and turn. */
export function sizedTo(frame: Matrix, width: number, height: number): Matrix {
  const centre = centreOf(frame);
  const { degrees } = sizeOf(frame);
  return frameOf(
    centre.x - width / 2,
    centre.y - height / 2,
    Math.max(SMALLEST_SIDE, width),
    Math.max(SMALLEST_SIDE, height),
    degrees,
  );
}

/** A guide a drag snapped to: a vertical (`x`) or horizontal (`y`) line across the page, and what it is. */
export interface Guide {
  readonly axis: "x" | "y";
  readonly at: number;
  readonly kind: "margin" | "centre" | "object";
}

/** The margin the snap lines run along, in points (half an inch). */
export const MARGIN = 36;

/** How near a drag gets before it snaps, in points. */
export const SNAP_DISTANCE = 4;

/**
 * Moves `box` by (`dx`, `dy`) and pulls it onto the nearest snap line within `distance`: the
 * page's margins and centre, and the edges and middles of `others`. A line is used when any of
 * the box's left, middle or right (top, middle, bottom) falls near it.
 */
export function snapMove(
  box: Box,
  dx: number,
  dy: number,
  page: { readonly width: number; readonly height: number },
  others: readonly Box[],
  distance = SNAP_DISTANCE,
): { readonly dx: number; readonly dy: number; readonly guides: readonly Guide[] } {
  const lines = (axis: "x" | "y"): Guide[] => {
    const size = axis === "x" ? page.width : page.height;
    const found: Guide[] = [
      { axis, at: MARGIN, kind: "margin" },
      { axis, at: size - MARGIN, kind: "margin" },
      { axis, at: size / 2, kind: "centre" },
    ];
    for (const other of others) {
      const [lo, hi] = axis === "x" ? [other[0], other[2]] : [other[1], other[3]];
      found.push(
        { axis, at: lo, kind: "object" },
        { axis, at: (lo + hi) / 2, kind: "object" },
        { axis, at: hi, kind: "object" },
      );
    }
    return found;
  };
  const pull = (axis: "x" | "y", delta: number) => {
    const [lo, hi] = axis === "x" ? [box[0], box[2]] : [box[1], box[3]];
    const edges = [lo + delta, (lo + hi) / 2 + delta, hi + delta];
    let best: { shift: number; guide: Guide } | undefined;
    for (const guide of lines(axis)) {
      for (const edge of edges) {
        const shift = guide.at - edge;
        if (Math.abs(shift) <= distance && (best === undefined || Math.abs(shift) < Math.abs(best.shift)))
          best = { shift, guide };
      }
    }
    return best;
  };
  const x = pull("x", dx);
  const y = pull("y", dy);
  return {
    dx: dx + (x?.shift ?? 0),
    dy: dy + (y?.shift ?? 0),
    guides: [...(x === undefined ? [] : [x.guide]), ...(y === undefined ? [] : [y.guide])],
  };
}

/** What an arrow key moves a selection by, in points: 1, or 10 with Shift; undefined for another key. */
export function nudgeOf(key: string, shift: boolean): Point | undefined {
  const step = shift ? 10 : 1;
  switch (key) {
    case "ArrowLeft":
      return { x: -step, y: 0 };
    case "ArrowRight":
      return { x: step, y: 0 };
    case "ArrowUp":
      return { x: 0, y: -step };
    case "ArrowDown":
      return { x: 0, y: step };
    default:
      return undefined;
  }
}

/** The page-space box a tool's drag from `from` to `to` makes; `square` (Shift) keeps it square. */
export function dragBox(from: Point, to: Point, square = false): Box {
  let dx = to.x - from.x;
  let dy = to.y - from.y;
  if (square) {
    const side = Math.max(Math.abs(dx), Math.abs(dy));
    dx = Math.sign(dx || 1) * side;
    dy = Math.sign(dy || 1) * side;
  }
  return [
    Math.min(from.x, from.x + dx),
    Math.min(from.y, from.y + dy),
    Math.max(from.x, from.x + dx),
    Math.max(from.y, from.y + dy),
  ];
}

/** A line's end after a drag, with Shift holding it to a multiple of 45 degrees. */
export function lineEnd(from: Point, to: Point, constrain = false): Point {
  if (!constrain) return to;
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  const length = Math.hypot(dx, dy);
  const step = Math.PI / 4;
  const angle = Math.round(Math.atan2(dy, dx) / step) * step;
  return { x: from.x + Math.cos(angle) * length, y: from.y + Math.sin(angle) * length };
}

/** `box` kept to the page: moved in, never resized, unless it is bigger than the page. */
export function keptOnPage(box: Box, page: { readonly width: number; readonly height: number }): Box {
  const w = box[2] - box[0];
  const h = box[3] - box[1];
  const x = Math.min(Math.max(box[0], 0), Math.max(0, page.width - w));
  const y = Math.min(Math.max(box[1], 0), Math.max(0, page.height - h));
  return [x, y, x + w, y + h];
}
