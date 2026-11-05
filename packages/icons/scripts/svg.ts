// A small reader for the icon files: the root `<svg>` and its flat list of shapes, and each shape's
// bounding box on the 24-unit grid. Paths are sampled along every segment (lines, curves and arcs,
// relative and absolute; SVG 1.1 §8.3, arcs through the centre form of Appendix F.6.5), so the box
// is the drawn geometry, not the control points. Node-free, so the tests import it as it is.

/** One element: its tag name and attributes. */
export interface Element {
  name: string;
  attrs: Record<string, string>;
}

/** An icon file read: the root's attributes and its shapes, in order. */
export interface Svg {
  root: Record<string, string>;
  shapes: Element[];
}

/** An axis-aligned box on the icon grid. */
export interface Box {
  minX: number;
  minY: number;
  maxX: number;
  maxY: number;
}

const TAG = /<(\/?)([a-z]+)((?:\s+[a-zA-Z-]+="[^"]*")*)\s*(\/?)>/gy;
const ATTR = /([a-zA-Z-]+)="([^"]*)"/g;

function attributes(text: string): Record<string, string> {
  const out: Record<string, string> = {};
  for (const m of text.matchAll(ATTR)) out[m[1] ?? ""] = m[2] ?? "";
  return out;
}

interface Tag {
  close: boolean;
  name: string;
  attrs: Record<string, string>;
  selfClosing: boolean;
}

/** The file's tags in order; any text between them is an error. */
function tags(body: string): Tag[] {
  TAG.lastIndex = 0;
  const out: Tag[] = [];
  while (TAG.lastIndex < body.length) {
    const at = TAG.lastIndex;
    const m = TAG.exec(body);
    if (m === null) throw new Error(`unexpected text at ${at}: ${body.slice(at, at + 20)}`);
    out.push({ close: m[1] === "/", name: m[2] ?? "", attrs: attributes(m[3] ?? ""), selfClosing: m[4] === "/" });
  }
  return out;
}

/** Reads an icon file: one `<svg>` holding shapes that each close at once, nothing else. */
export function parseSvg(text: string): Svg {
  const all = tags(text.trim());
  const first = all[0];
  const last = all.at(-1);
  if (first?.name !== "svg" || first.close || last?.name !== "svg" || !last.close) {
    throw new Error("the file must be one <svg> element");
  }
  const shapes: Element[] = [];
  for (let i = 1; i < all.length - 1; i++) {
    const tag = all[i];
    if (tag === undefined || tag.close) throw new Error("a closing tag without its element");
    const end = tag.selfClosing ? tag : all[++i];
    if (end !== tag && (end?.name !== tag.name || !end.close)) {
      throw new Error(`<${tag.name}> must close at once, with nothing inside`);
    }
    shapes.push({ name: tag.name, attrs: tag.attrs });
  }
  return { root: first.attrs, shapes };
}

type Point = [number, number];

const num = (attrs: Record<string, string>, key: string) => Number.parseFloat(attrs[key] ?? "0");

/** The points a shape passes through: exact for rectangles and circles, sampled for paths. */
function shapePoints(shape: Element): Point[] {
  const a = shape.attrs;
  switch (shape.name) {
    case "rect": {
      const [x, y] = [num(a, "x"), num(a, "y")];
      return [
        [x, y],
        [x + num(a, "width"), y + num(a, "height")],
      ];
    }
    case "circle":
    case "ellipse": {
      const rx = num(a, shape.name === "circle" ? "r" : "rx");
      const ry = num(a, shape.name === "circle" ? "r" : "ry");
      const [cx, cy] = [num(a, "cx"), num(a, "cy")];
      return [
        [cx - rx, cy - ry],
        [cx + rx, cy + ry],
      ];
    }
    case "line":
      return [
        [num(a, "x1"), num(a, "y1")],
        [num(a, "x2"), num(a, "y2")],
      ];
    case "polyline":
    case "polygon": {
      const n = numbers(a.points ?? "");
      return n.flatMap((v, i) => (i % 2 === 0 ? [[v, n[i + 1] ?? 0] as Point] : []));
    }
    case "path":
      return pathPoints(a.d ?? "");
    default:
      throw new Error(`no geometry for <${shape.name}>`);
  }
}

/** The box around every shape. */
export function boundingBox(shapes: Element[]): Box {
  const points = shapes.flatMap(shapePoints);
  const xs = points.map((p) => p[0]);
  const ys = points.map((p) => p[1]);
  return { minX: Math.min(...xs), minY: Math.min(...ys), maxX: Math.max(...xs), maxY: Math.max(...ys) };
}

function numbers(text: string): number[] {
  return [...text.matchAll(/[-+]?(?:\d+\.?\d*|\.\d+)(?:e[-+]?\d+)?/gi)].map((m) => Number(m[0]));
}

const ARGS: Record<string, number> = { m: 2, l: 2, h: 1, v: 1, c: 6, s: 4, q: 4, t: 2, a: 7, z: 0 };
const STEPS = 32;

/** Every command of a path with its numbers, implicit repeats split out (`M1 2 3 4` is a move then a line). */
function commands(d: string): [string, number[]][] {
  return [...d.matchAll(/([MLHVCSQTAZmlhvcsqtaz])([^MLHVCSQTAZmlhvcsqtaz]*)/g)].flatMap((m) => {
    const cmd = m[1] ?? "";
    return segments(cmd, cmd.toLowerCase() === "a" ? arcNumbers(m[2] ?? "") : numbers(m[2] ?? ""));
  });
}

/**
 * An arc's numbers, where the two flags are single `0` or `1` digits that may run into the next
 * number (`a2 2 0 012 2`, as optimisers write them; SVG 1.1 §8.3.9's grammar).
 */
function arcNumbers(text: string): number[] {
  const out: number[] = [];
  const number = /^[\s,]*([-+]?(?:\d+\.?\d*|\.\d+)(?:e[-+]?\d+)?)/i;
  const flag = /^[\s,]*([01])/;
  let rest = text;
  while (rest.trim() !== "") {
    const pattern = out.length % 7 === 3 || out.length % 7 === 4 ? flag : number;
    const m = pattern.exec(rest);
    if (m === null) throw new Error(`can't read the arc's numbers at "${rest.trim().slice(0, 12)}"`);
    out.push(Number(m[1]));
    rest = rest.slice(m[0].length);
  }
  return out;
}

/** One command's numbers cut into segments; a move's extra pairs are line-tos. */
function segments(cmd: string, args: number[]): [string, number[]][] {
  const n = ARGS[cmd.toLowerCase()] ?? 0;
  if (n === 0) return [[cmd, []]];
  if (args.length % n !== 0 || args.length === 0) throw new Error(`"${cmd}" needs ${n} numbers per segment`);
  const repeat = cmd === "M" ? "L" : cmd === "m" ? "l" : cmd;
  const out: [string, number[]][] = [];
  for (let i = 0; i < args.length; i += n) out.push([i === 0 ? cmd : repeat, args.slice(i, i + n)]);
  return out;
}

/** Where a path is: the current point, its subpath's start, and the last segment's control point. */
interface Pen {
  cur: Point;
  start: Point;
  ctrl: Point | undefined;
  last: string;
}

/** One segment: the points it passes through and the control point the next smooth segment reflects. */
type Segment = (
  pen: Pen,
  at: (i: number) => Point,
  v: (i: number) => number,
  rel: boolean,
) => { end: Point; pts: Point[]; ctrl?: Point };

const SEGMENTS: Record<string, Segment> = {
  M: (_pen, at) => ({ end: at(0), pts: [] }),
  L: (_pen, at) => ({ end: at(0), pts: [] }),
  H: (pen, _at, v, rel) => ({ end: [v(0) + (rel ? pen.cur[0] : 0), pen.cur[1]], pts: [] }),
  V: (pen, _at, v, rel) => ({ end: [pen.cur[0], v(0) + (rel ? pen.cur[1] : 0)], pts: [] }),
  C: (pen, at) => ({ end: at(4), pts: curve([pen.cur, at(0), at(2), at(4)]), ctrl: at(2) }),
  S: (pen, at) => {
    const c1 = reflect(pen, "CS") ?? pen.cur;
    return { end: at(2), pts: curve([pen.cur, c1, at(0), at(2)]), ctrl: at(0) };
  },
  Q: (pen, at) => ({ end: at(2), pts: curve([pen.cur, at(0), at(2)]), ctrl: at(0) }),
  T: (pen, at) => {
    const ctrl = reflect(pen, "QT");
    return ctrl === undefined ? { end: at(0), pts: [] } : { end: at(0), pts: curve([pen.cur, ctrl, at(0)]), ctrl };
  },
  A: (pen, at, v) => ({ end: at(5), pts: arc(pen.cur, v(0), v(1), v(2), v(3) !== 0, v(4) !== 0, at(5)) }),
  Z: (pen) => ({ end: pen.start, pts: [] }),
};

/** Points along a path, in absolute coordinates. */
export function pathPoints(d: string): Point[] {
  const out: Point[] = [];
  const pen: Pen = { cur: [0, 0], start: [0, 0], ctrl: undefined, last: "" };
  for (const [cmd, a] of commands(d)) {
    const rel = cmd === cmd.toLowerCase();
    const c = cmd.toUpperCase();
    const v = (i: number) => a[i] ?? 0;
    const at = (i: number): Point => (rel ? [pen.cur[0] + v(i), pen.cur[1] + v(i + 1)] : [v(i), v(i + 1)]);
    const segment = SEGMENTS[c];
    if (segment === undefined) throw new Error(`unknown path command ${cmd}`);
    const { end, pts, ctrl } = segment(pen, at, v, rel);
    out.push(...pts, end);
    if (c === "M") pen.start = end;
    pen.cur = end;
    pen.ctrl = ctrl;
    pen.last = c;
  }
  return out;
}

/** The control point a smooth curve reflects from the previous segment, if that was of its family. */
function reflect(pen: Pen, family: string): Point | undefined {
  if (pen.ctrl === undefined || !family.includes(pen.last)) return undefined;
  return [2 * pen.cur[0] - pen.ctrl[0], 2 * pen.cur[1] - pen.ctrl[1]];
}

/** Points along a Bézier curve of any degree (de Casteljau). */
function curve(ctrl: Point[]): Point[] {
  const out: Point[] = [];
  for (let s = 0; s <= STEPS; s++) {
    let level = ctrl;
    const t = s / STEPS;
    while (level.length > 1) {
      level = level.slice(1).map((p, i) => {
        const q = level[i] ?? p;
        return [q[0] + (p[0] - q[0]) * t, q[1] + (p[1] - q[1]) * t] as Point;
      });
    }
    if (level[0] !== undefined) out.push(level[0]);
  }
  return out;
}

/** Points along an elliptical arc, from the endpoint form to the centre form (SVG 1.1 F.6.5). */
function arc(p1: Point, rxIn: number, ryIn: number, angle: number, large: boolean, sweep: boolean, p2: Point): Point[] {
  let [rx, ry] = [Math.abs(rxIn), Math.abs(ryIn)];
  // Coincident endpoints draw nothing; zero radii draw a line (SVG 1.1 F.6.2).
  if (p1[0] === p2[0] && p1[1] === p2[1]) return [p1];
  if (rx === 0 || ry === 0) return [p1, p2];
  const phi = (angle * Math.PI) / 180;
  const [cos, sin] = [Math.cos(phi), Math.sin(phi)];
  const dx = (p1[0] - p2[0]) / 2;
  const dy = (p1[1] - p2[1]) / 2;
  const x1 = cos * dx + sin * dy;
  const y1 = -sin * dx + cos * dy;
  const lambda = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry);
  if (lambda > 1) [rx, ry] = [rx * Math.sqrt(lambda), ry * Math.sqrt(lambda)];
  const num2 = rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1;
  const den = rx * rx * y1 * y1 + ry * ry * x1 * x1;
  const k = (large === sweep ? -1 : 1) * Math.sqrt(Math.max(0, num2 / den));
  const cx1 = (k * rx * y1) / ry;
  const cy1 = (-k * ry * x1) / rx;
  const cx = cos * cx1 - sin * cy1 + (p1[0] + p2[0]) / 2;
  const cy = sin * cx1 + cos * cy1 + (p1[1] + p2[1]) / 2;
  const theta = (ux: number, uy: number, vx: number, vy: number) => Math.atan2(ux * vy - uy * vx, ux * vx + uy * vy);
  const t1 = theta(1, 0, (x1 - cx1) / rx, (y1 - cy1) / ry);
  let dt = theta((x1 - cx1) / rx, (y1 - cy1) / ry, (-x1 - cx1) / rx, (-y1 - cy1) / ry);
  if (!sweep && dt > 0) dt -= 2 * Math.PI;
  if (sweep && dt < 0) dt += 2 * Math.PI;
  const out: Point[] = [];
  for (let s = 0; s <= STEPS * 2; s++) {
    const t = t1 + (dt * s) / (STEPS * 2);
    const [ex, ey] = [rx * Math.cos(t), ry * Math.sin(t)];
    out.push([cos * ex - sin * ey + cx, sin * ex + cos * ey + cy]);
  }
  return out;
}
