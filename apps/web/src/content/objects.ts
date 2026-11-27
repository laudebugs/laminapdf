// What Edit mode knows of an object Lamina PDF added to a page (Paper A02): its kind, and its
// frame, the matrix that maps its unit square to page space. An image and a page of another PDF
// fill the unit square, so the frame is their matrix; a shape's path is in page space, so its
// frame is the path's box moved by the matrix; added text is a box whose matrix flips y, so its
// frame is the box above the matrix's origin. Handles and hit tests read the frame; changes are
// written back as the matrix that gives the new frame.

import type { AddedObject, PathSpec } from "@laminapdf/sdk";
import { type Box, boxAround, frameOf, invert, type Matrix, multiply } from "./geometry.ts";

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

/** The item for `added` on `page`; undefined for an object whose form another version stored differently. */
export function itemOf(page: number, added: AddedObject): Item | undefined {
  const spec = added.object;
  if (spec === undefined) return undefined;
  const base = { id: added.id, page, added };
  if ("text" in spec) {
    const [, , , , e, f] = added.matrix;
    return { ...base, kind: "text", frame: frameOf(e, f - spec.text.height, spec.text.width, spec.text.height) };
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

/** The matrix that gives `item` the frame `frame`; a text box takes only a move. */
export function matrixFor(item: Item, frame: Matrix): Matrix {
  if (item.kind === "shape") return shapeMatrix(item, frame) ?? item.added.matrix;
  if (item.kind === "text") {
    const [a, b, c, d] = item.added.matrix;
    const height = item.added.object !== undefined && "text" in item.added.object ? item.added.object.text.height : 0;
    // The box's top-left corner is where the frame's is; its matrix origin is its bottom-left.
    return [a, b, c, d, frame[4], frame[5] + height];
  }
  return frame;
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
