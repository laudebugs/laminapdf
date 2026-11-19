// Comment mode's edits, kept apart from the DOM so they can be tested in Node: each annotation
// kind's facts in one table (its name, its colour family, the spec patch that changes it), a mark
// written as `addAnnot` in user space through the page's frame (a `ToUser` mapping: Comment mode
// gives @laminapdf/viewer's pageToUser, which laminapdf-model's PageFrame is held to), and the
// `updateAnnot` and `deleteAnnot` the inspector and resizing make. Every op is built here, typed as
// the SDK's `Op`.

import type { AnnotSpec, Op, PageAnnotation } from "@laminapdf/sdk";
import { isoToPdfDate, type LaterMessageId, type LineEndingName, laterMessage } from "@laminapdf/ui/editing";
import type { Box, Mark, MarkStyle } from "./marks.ts";

/**
 * A page's page-space points in user space: the page's frame as a mapping. Comment mode passes
 * @laminapdf/viewer's `pageToUser` over the page's frame, so this module, tested in Node, never
 * loads the viewer.
 */
export type ToUser = (point: readonly [number, number]) => { x: number; y: number };

/** `updateAnnot`'s arguments. */
export type AnnotChange = Extract<Op, { op: "updateAnnot" }>["args"];

/** A change to an annotation's own geometry or drawing. */
type SpecPatch = NonNullable<AnnotChange["spec"]>;

/** What a kind's spec patch changes: a shape's, a line's, a polygon's, ink's or a caret's. */
type PatchType = "shape" | "line" | "poly" | "ink" | "caret";

/** What Comment mode knows of an annotation kind. */
export interface Kind {
  /** Its name in the later catalogue. */
  readonly name: LaterMessageId;
  /** Fill colours (a highlight's, a note's) or line colours. */
  readonly family: "fill" | "line";
  /** The spec patch that changes its geometry and drawing; none for a kind Comment mode can't resize. */
  readonly patch?: PatchType;
  /** A closed shape: it takes a fill, and a cloudy border. */
  readonly closed?: true;
  /** An open line: it takes line endings. */
  readonly ends?: true;
}

/** Each subtype Comment mode names, by `/Subtype`. */
const KINDS: Readonly<Record<string, Kind>> = {
  Highlight: { name: "kind.Highlight", family: "fill" },
  Underline: { name: "kind.Underline", family: "line" },
  StrikeOut: { name: "kind.StrikeOut", family: "line" },
  Squiggly: { name: "kind.Squiggly", family: "line" },
  Text: { name: "kind.Text", family: "fill" },
  FreeText: { name: "kind.FreeText", family: "line" },
  Caret: { name: "kind.Caret", family: "line", patch: "caret" },
  Ink: { name: "kind.Ink", family: "line", patch: "ink" },
  Line: { name: "kind.Line", family: "line", patch: "line", ends: true },
  Square: { name: "kind.Square", family: "line", patch: "shape", closed: true },
  Circle: { name: "kind.Circle", family: "line", patch: "shape", closed: true },
  Polygon: { name: "kind.Polygon", family: "line", patch: "poly", closed: true },
  PolyLine: { name: "kind.PolyLine", family: "line", patch: "poly", ends: true },
  Stamp: { name: "kind.Stamp", family: "line" },
  FileAttachment: { name: "kind.FileAttachment", family: "fill" },
  Sound: { name: "kind.Sound", family: "line" },
};

const OTHER: Kind = { name: "kind.other", family: "line" };

/** What Comment mode knows of `/Subtype` `subtype`. */
export function kindOf(subtype: string): Kind {
  return KINDS[subtype] ?? OTHER;
}

/** An annotation kind's name in words ("Rectangle"). */
export function kindName(subtype: string): string {
  return laterMessage(kindOf(subtype).name);
}

/** The `/Subtype` a mark writes. */
export function subtypeOf(mark: Mark): string {
  switch (mark.type) {
    case "markup":
      return { highlight: "Highlight", underline: "Underline", strikeOut: "StrikeOut" }[mark.kind];
    case "note":
      return "Text";
    case "caret":
      return "Caret";
    case "ink":
      return "Ink";
    case "shape":
      return mark.kind === "square" ? "Square" : "Circle";
    case "line":
      return "Line";
    case "poly":
      return mark.kind === "polygon" ? "Polygon" : "PolyLine";
  }
}

/** Whether Comment mode can resize `annotation`: its kind has a spec patch. */
export function canResize(annotation: PageAnnotation): boolean {
  return kindOf(annotation.subtype).patch !== undefined;
}

/** A page-space box in user space, normalised. */
export function boxToUser(toUser: ToUser, [x0, y0, x1, y1]: Box): { x0: number; y0: number; x1: number; y1: number } {
  const a = toUser([x0, y0]);
  const b = toUser([x1, y1]);
  return { x0: Math.min(a.x, b.x), y0: Math.min(a.y, b.y), x1: Math.max(a.x, b.x), y1: Math.max(a.y, b.y) };
}

/** A box's four corners in user space: its top left, top right, bottom left and bottom right on the page. */
function quad(toUser: ToUser, [x0, y0, x1, y1]: Box) {
  return (
    [
      [x0, y0],
      [x1, y0],
      [x0, y1],
      [x1, y1],
    ] as const
  ).map(toUser);
}

/** Flat `[x, y, …]` page points as points in user space. */
function usersOf(toUser: ToUser, flat: readonly number[]): { x: number; y: number }[] {
  const out: { x: number; y: number }[] = [];
  for (let i = 0; i + 1 < flat.length; i += 2) out.push(toUser([flat[i] ?? 0, flat[i + 1] ?? 0]));
  return out;
}

/** RGB as an annotation colour. */
const rgb = ([r, g, b]: readonly [number, number, number]): [number, number, number] => [r, g, b];

/** The `spec` of `addAnnot` that draws `mark` on a page that `toUser` maps, styled by `style`. */
export function markSpec(mark: Mark, toUser: ToUser, style: MarkStyle): AnnotSpec {
  const user = toUser;
  const border = { width: style.width };
  const interior = style.fill === undefined ? {} : { interior: rgb(style.fill) };
  switch (mark.type) {
    case "markup":
      return { type: "textMarkup", kind: mark.kind, quads: mark.lines.map((line) => quad(toUser, line)) };
    case "note":
      return { type: "note", at: user(mark.at), icon: "note" };
    case "caret":
      return { type: "caret", rect: boxToUser(toUser, mark.box) };
    case "ink":
      return { type: "ink", strokes: inkStrokes(mark, user, style.width), width: style.width };
    case "shape":
      return { type: "shape", kind: mark.kind, rect: boxToUser(toUser, mark.box), border, ...interior };
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
        ...(mark.kind === "polygon" ? interior : {}),
      };
  }
}

/** Now, as a PDF date. */
function pdfNow(when = new Date()): string {
  return isoToPdfDate(when.toISOString()) ?? "";
}

/** The `addAnnot` op that makes `mark` on page `page` (its ID). */
export function addAnnotOp(
  page: string,
  mark: Mark,
  toUser: ToUser,
  style: MarkStyle,
  common: { readonly author?: string | undefined; readonly created: Date },
): Op {
  const date = pdfNow(common.created);
  return {
    op: "addAnnot",
    args: {
      page,
      spec: markSpec(mark, toUser, style),
      common: {
        color: rgb(style.colour),
        ...(style.opacity < 1 ? { opacity: style.opacity } : {}),
        ...(common.author === undefined || common.author === "" ? {} : { author: common.author }),
        created: date,
        modified: date,
      },
    },
  };
}

/** The `updateAnnot` op that makes `change` to annotation `id`, dated now. */
export function updateAnnotOp(id: string, change: Omit<AnnotChange, "id" | "modified">): Op {
  return { op: "updateAnnot", args: { ...change, id, modified: pdfNow() } };
}

/** The `deleteAnnot` op that deletes annotation `id`. */
export function deleteAnnotOp(id: string): Op {
  return { op: "deleteAnnot", args: { id } };
}

/** An outline, as the drawing specs take it. */
type Border = NonNullable<Extract<AnnotSpec, { type: "shape" }>["border"]>;

/** A border style's wire form, as the drawing specs take it. */
export function borderStyle(style: "solid" | "dashed" | "cloudy"): NonNullable<Border["style"]> {
  switch (style) {
    case "dashed":
      return { type: "dashed", pattern: [3, 2] };
    case "cloudy":
      return { type: "cloudy", intensity: 1 };
    default:
      return { type: "solid" };
  }
}

/** The patch that sets a closed shape's fill (`null` for none); none for another kind. */
export function fillPatch(subtype: string, interior: [number, number, number] | null): SpecPatch | undefined {
  const kind = kindOf(subtype);
  if (kind.closed !== true) return undefined;
  return kind.patch === "shape" ? { type: "shape", interior } : { type: "poly", interior };
}

/** The patch that sets an outline (`border`), or ink's pen width; none for a kind without either. */
export function borderPatch(subtype: string, border: Required<Border>): SpecPatch | undefined {
  switch (kindOf(subtype).patch) {
    case "ink":
      return { type: "ink", width: border.width };
    case "shape":
      return { type: "shape", border: { ...border } };
    case "line":
      return { type: "line", border: { ...border } };
    case "poly":
      return { type: "poly", border: { ...border } };
    default:
      return undefined;
  }
}

/** The patch that sets an open line's endings; none for another kind. */
export function endsPatch(subtype: string, ends: readonly [LineEndingName, LineEndingName]): SpecPatch | undefined {
  const kind = kindOf(subtype);
  if (kind.ends !== true) return undefined;
  return kind.patch === "line" ? { type: "line", ends: [...ends] } : { type: "poly", ends: [...ends] };
}

/** The patch that turns ink's smoothing on or off; none for another kind. */
export function smoothingPatch(subtype: string, smoothing: boolean): SpecPatch | undefined {
  return kindOf(subtype).patch === "ink" ? { type: "ink", smoothing } : undefined;
}

/** Each patch type's resize: the stretched record's geometry, in user space, as its spec patch. */
const RESIZES: Readonly<Record<PatchType, (resized: PageAnnotation, toUser: ToUser) => SpecPatch | undefined>> = {
  shape: ({ geometry: g, rect }, toUser) => {
    const box = g.kind === "inner" ? g.rect : rect;
    return box === undefined ? undefined : { type: "shape", rect: boxToUser(toUser, box) };
  },
  caret: ({ rect }, toUser) => (rect === undefined ? undefined : { type: "caret", rect: boxToUser(toUser, rect) }),
  ink: ({ geometry: g }, toUser) =>
    g.kind === "ink" ? { type: "ink", strokes: g.strokes.map((s) => ({ points: usersOf(toUser, s) })) } : undefined,
  line: ({ geometry: g }, toUser) => {
    const [from, to] = g.kind === "line" ? usersOf(toUser, g.points) : [];
    return from === undefined || to === undefined ? undefined : { type: "line", from, to };
  },
  poly: ({ geometry: g }, toUser) =>
    g.kind === "vertices" ? { type: "poly", vertices: usersOf(toUser, g.points) } : undefined,
};

/**
 * The edit that gives `annotation` the shape of `resized` (the layer's stretched copy): its
 * kind's spec patch, its geometry in user space. Undefined for a kind that can't be resized.
 */
export function resizeOp(annotation: PageAnnotation, resized: PageAnnotation, toUser: ToUser): Op | undefined {
  const type = kindOf(annotation.subtype).patch;
  const patch = type === undefined ? undefined : RESIZES[type](resized, toUser);
  return patch === undefined ? undefined : updateAnnotOp(annotation.id, { spec: patch });
}

/** The line endings of an arrow's new mark: none at its start, an open arrow at its end. */
export const ARROW_ENDS: readonly [LineEndingName, LineEndingName] = ["none", "openArrow"];

/**
 * The width a pen of `width` points draws at `pressure` (0 to 1): the pen's width at the half
 * pressure a pen reports by default, a quarter of it at the lightest touch and 1.75 times it at
 * the hardest.
 */
function pressedWidth(width: number, pressure: number): number {
  return width * (0.25 + 1.5 * Math.min(Math.max(pressure, 0), 1));
}

/** An ink mark's strokes in user space, each with its points' widths when a pen gave pressures. */
export function inkStrokes(mark: Extract<Mark, { type: "ink" }>, user: ToUser, width: number) {
  return mark.strokes.map((points, i) => {
    const pressures = mark.pressures?.[i];
    return {
      points: points.map(user),
      ...(pressures !== undefined && pressures.length === points.length
        ? { widths: pressures.map((p) => pressedWidth(width, p)) }
        : {}),
    };
  });
}
