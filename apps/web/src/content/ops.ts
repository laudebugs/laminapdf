// Edit mode's ops, apart from the DOM so Node can test them (Paper A01, A02): what adding text,
// an image or a shape, changing one of them and deleting it write. Every op names its page by ID
// and carries the date it was made, so no op reads a clock (E6 plan §2.1); the dates come from
// `now`, which tests give a fixed one.

import type { AnnotColor, ContentChanges, Op, PathSpec, Stroke, StrokeDash, TextBlock } from "@laminapdf/sdk";
import { pdfNow } from "../comment/ops.ts";
import type { Matrix, Point } from "./geometry.ts";

/** An RGB colour, each channel 0 to 1. */
export type Rgb = readonly [number, number, number];

/** What a shape tool draws: its fill and outline. */
export interface ShapeLook {
  readonly fill: Rgb | null;
  readonly stroke: { readonly color: Rgb; readonly width: number; readonly dash: StrokeDash } | null;
}

/** The shapes the Shape tool draws. */
export type ShapeKind = "line" | "rect" | "ellipse" | "polygon";

/** A new object's ID: unique among a page's (an ID is 1 to 64 of letters, digits, `-`, `_` and `.`). */
export function newId(now = Date.now(), random = Math.random): string {
  return `obj-${now.toString(36)}-${Math.floor(random() * 36 ** 4).toString(36)}`;
}

/** The date an op carries. */
export function modifiedNow(when = new Date()): string {
  return pdfNow(when);
}

/** A colour as the ops write it. */
export function colour(rgb: Rgb): AnnotColor {
  return [rgb[0], rgb[1], rgb[2]] as unknown as AnnotColor;
}

/** A stroke as the ops write it. */
export function strokeOf(look: NonNullable<ShapeLook["stroke"]>): Stroke {
  return { color: colour(look.color), width: look.width, ...(look.dash === "solid" ? {} : { dash: look.dash }) };
}

/**
 * The `addContentObject` op for a picture: `blob`, drawn into `frame`. With no `alt` the core
 * refuses it on a PDF/UA file (`content-needs-alt-text`), and the person is asked for one.
 */
export function addImageOp(
  page: string,
  id: string,
  blob: string,
  frame: Matrix,
  alt: string | "decorative" | undefined,
  when?: Date,
): Op {
  return {
    op: "addContentObject",
    args: {
      id,
      page,
      object: { image: { blob } },
      matrix: [...frame],
      opacity: 1,
      layer: "front",
      ...(alt === undefined ? {} : { alt: alt === "decorative" ? "decorative" : { text: alt } }),
      modified: modifiedNow(when),
    },
  } as unknown as Op;
}

/** The path a shape tool makes from a drag between `from` and `to` (a polygon from its points). */
export function pathOf(kind: ShapeKind, from: Point, to: Point, points: readonly Point[] = []): PathSpec {
  const x0 = Math.min(from.x, to.x);
  const y0 = Math.min(from.y, to.y);
  const x1 = Math.max(from.x, to.x);
  const y1 = Math.max(from.y, to.y);
  switch (kind) {
    case "line":
      return { line: { from, to } } as PathSpec;
    case "rect":
      return { rect: { rect: [x0, y0, x1, y1] } } as PathSpec;
    case "ellipse":
      return { ellipse: { rect: [x0, y0, x1, y1] } } as PathSpec;
    case "polygon":
      return { polygon: { points: [...points], closed: true } } as PathSpec;
  }
}

/** The `addContentObject` op for a shape. */
export function addShapeOp(page: string, id: string, path: PathSpec, look: ShapeLook, when?: Date): Op {
  return {
    op: "addContentObject",
    args: {
      id,
      page,
      object: {
        shape: {
          path,
          ...(look.fill === null ? {} : { fill: colour(look.fill) }),
          ...(look.stroke === null ? {} : { stroke: strokeOf(look.stroke) }),
        },
      },
      matrix: [1, 0, 0, 1, 0, 0],
      opacity: 1,
      layer: "front",
      alt: "decorative",
      modified: modifiedNow(when),
    },
  } as unknown as Op;
}

/** What added text is set in. */
export interface TextLook {
  readonly fonts: readonly string[];
  readonly size: number;
  readonly color: Rgb;
  readonly align: "left" | "center" | "right";
  readonly bold: boolean;
  readonly italic: boolean;
}

/** The runs of `text` in `look`'s style. */
export function runsOf(text: string, look: Pick<TextLook, "bold" | "italic">): TextBlock["runs"] {
  return [{ text, ...(look.bold ? { bold: true } : {}), ...(look.italic ? { italic: true } : {}) }];
}

/** The `addText` op for `text` in the box `rect`. */
export function addTextOp(
  page: string,
  id: string,
  rect: readonly [number, number, number, number],
  text: string,
  look: TextLook,
  when?: Date,
): Op {
  return {
    op: "addText",
    args: {
      id,
      page,
      rect: [...rect],
      runs: runsOf(text, look),
      align: look.align,
      fonts: [...look.fonts],
      size: look.size,
      color: colour(look.color),
      modified: modifiedNow(when),
    },
  } as unknown as Op;
}

/** The `updateContentObject` op that makes `changes` to object `id` on page `page`. */
export function updateOp(page: string, id: string, changes: Omit<ContentChanges, "modified">, when?: Date): Op {
  return { op: "updateContentObject", args: { id, page, ...changes, modified: modifiedNow(when) } } as unknown as Op;
}

/** The `deleteContentObject` op. */
export function deleteOp(page: string, id: string): Op {
  return { op: "deleteContentObject", args: { id, page } } as unknown as Op;
}

/** The text of added text, runs joined. */
export function textOf(block: TextBlock): string {
  return block.runs.map((run) => run.text).join("");
}

/** An added colour as RGB; a grey or a CMYK colour is read as one. */
export function rgbOf(color: unknown): Rgb | undefined {
  if (!Array.isArray(color)) return undefined;
  const c = color as number[];
  if (c.length === 1) return [c[0] ?? 0, c[0] ?? 0, c[0] ?? 0];
  if (c.length === 3) return [c[0] ?? 0, c[1] ?? 0, c[2] ?? 0];
  if (c.length === 4) {
    const k = 1 - (c[3] ?? 0);
    return [(1 - (c[0] ?? 0)) * k, (1 - (c[1] ?? 0)) * k, (1 - (c[2] ?? 0)) * k];
  }
  return undefined;
}
