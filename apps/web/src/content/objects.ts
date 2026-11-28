// What Edit mode knows of an object Lamina PDF added to a page (Paper A02): its kind, and its
// frame, the matrix that maps its unit square to page space. An image and a page of another PDF
// fill the unit square, so the frame is their matrix; a shape's path is in page space, so its
// frame is the path's box moved by the matrix; added text is a box whose matrix flips y, so its
// frame is that box mapped through its matrix. Handles and hit tests read the frame; changes are
// written back as the matrix that gives the new frame.

import type { AddedObject, PathSpec } from "@laminapdf/sdk";
import { apply, type Box, boxAround, frameOf, invert, type Matrix, multiply, type Point } from "./geometry.ts";

/** What an added object is. */
type ItemKind = "text" | "image" | "shape" | "form";

/** One added object on a page, as the handles see it. */
export interface Item {
  readonly id: string;
  /** The page's index. */
  readonly page: number;
  readonly kind: ItemKind;
  readonly added: AddedObject;
  /** Unit square to page space. */
  readonly frame: Matrix;
}

/** The box of a shape's path, page space. */
export function pathBox(path: PathSpec): Box {
  if ("line" in path) return boxAround([path.line.from, path.line.to]);
  if ("rect" in path) return path.rect.rect;
  if ("ellipse" in path) return path.ellipse.rect;
  return boxAround(path.polygon.points);
}

/** A box as the frame that is its rectangle. */
function boxFrame([x0, y0, x1, y1]: Box): Matrix {
  return frameOf(x0, y0, Math.max(x1 - x0, 1e-3), Math.max(y1 - y0, 1e-3));
}

/** The map from a text box's own space (its bottom-left the origin, y up, `width` by `height`) to the unit square, y down. */
function textUnit(width: number, height: number): Matrix {
  return [width, 0, 0, -height, 0, height];
}

/** The item for `added` on `page`; undefined for an object whose form another version stored differently. */
export function itemOf(page: number, added: AddedObject): Item | undefined {
  const spec = added.object;
  if (spec === undefined) return undefined;
  const base = { id: added.id, page, added };
  if ("text" in spec) {
    return { ...base, kind: "text", frame: multiply(added.matrix, textUnit(spec.text.width, spec.text.height)) };
  }
  if ("shape" in spec)
    return { ...base, kind: "shape", frame: multiply(added.matrix, boxFrame(pathBox(spec.shape.path))) };
  return { ...base, kind: "image" in spec ? "image" : "form", frame: added.matrix };
}

/** The matrix that gives a shape (whose path is in page space) the frame `frame`. */
function shapeMatrix(item: Item, frame: Matrix): Matrix | undefined {
  const spec = item.added.object;
  if (spec === undefined || !("shape" in spec)) return undefined;
  const inverse = invert(boxFrame(pathBox(spec.shape.path)));
  return inverse === undefined ? undefined : multiply(frame, inverse);
}

/**
 * The matrix that gives `item` the frame `frame`. A text box's own width is its block's, so a new
 * width goes with the matrix (`width`); its height stays.
 */
export function matrixFor(item: Item, frame: Matrix, width?: number): Matrix {
  const spec = item.added.object;
  if (item.kind === "shape") return shapeMatrix(item, frame) ?? item.added.matrix;
  if (item.kind === "text" && spec !== undefined && "text" in spec) {
    const inverse = invert(textUnit(width ?? spec.text.width, spec.text.height));
    return inverse === undefined ? item.added.matrix : multiply(frame, inverse);
  }
  return frame;
}

/** Whether `m` only turns and moves (no scale, no mirror, no skew). */
function isRigid([a, b, c, d]: Matrix): boolean {
  const near = (x: number, y: number) => Math.abs(x - y) < 1e-6;
  return near(a * a + b * b, 1) && near(c * c + d * d, 1) && near(a * c + b * d, 0) && a * d - b * c > 0;
}

/**
 * A shape resized to `frame` with its path rebuilt, so its outline keeps its width: the new path
 * in the shape's own space, under the matrix it already has. Undefined when the matrix scales or
 * mirrors (then the matrix takes the new size and the outline scales with it).
 */
export function resizedShape(item: Item, frame: Matrix): PathSpec | undefined {
  const spec = item.added.object;
  const m = item.added.matrix;
  if (spec === undefined || !("shape" in spec) || !isRigid(m)) return undefined;
  const path = spec.shape.path;
  const from = boxFrame(pathBox(path));
  const inverseM = invert(m);
  const inverseFrom = invert(from);
  if (inverseM === undefined || inverseFrom === undefined) return undefined;
  const to = multiply(inverseM, frame);
  // Only a resize keeps the box upright in the shape's space.
  if (Math.abs(to[1]) > 1e-6 || Math.abs(to[2]) > 1e-6 || to[0] <= 0 || to[3] <= 0) return undefined;
  const through = multiply(to, inverseFrom);
  const point = (p: Point): Point => apply(through, p);
  const box: Box = [to[4], to[5], to[4] + to[0], to[5] + to[3]];
  if ("line" in path) return { line: { from: point(path.line.from), to: point(path.line.to) } } as PathSpec;
  if ("rect" in path) {
    const radius = path.rect.radius ?? 0;
    const room = Math.min(box[2] - box[0], box[3] - box[1]) / 2;
    return { rect: { rect: box, ...(radius > 0 ? { radius: Math.min(radius, room) } : {}) } } as PathSpec;
  }
  if ("ellipse" in path) return { ellipse: { rect: box } } as PathSpec;
  return { polygon: { points: path.polygon.points.map(point), closed: path.polygon.closed ?? true } } as PathSpec;
}

/** The kind's name in the later catalogue ("kind.text"…), or the shape's own ("shape.rect"…). */
export function nameOf(item: Item): `object.${string}` {
  const spec = item.added.object;
  if (item.kind === "shape" && spec !== undefined && "shape" in spec) {
    const path = spec.shape.path;
    const shape = "line" in path ? "line" : "rect" in path ? "rect" : "ellipse" in path ? "ellipse" : "polygon";
    return `object.${shape}`;
  }
  return `object.${item.kind}`;
}
