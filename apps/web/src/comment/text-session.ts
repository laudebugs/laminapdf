// The text a box is being typed into (Paper C03), apart from the DOM so it runs in Node: where a
// new text box, callout or typewriter text goes, and the one edit a typing session makes, from
// the box opening to editing ending. A new box left blank makes nothing; a box emptied
// is deleted; typing, a wider box and a moved leader line are one `updateAnnot`, so undo takes the
// whole session back in one step. Text the standard fonts lack names the fallback font.

import type { AnnotFont, Op, RichText, TextBox } from "@laminapdf/sdk";
import { isoToPdfDate, type LineEndingName, laterMessage, type TextKind } from "@laminapdf/ui/editing";
import type { Box, Point } from "./marks.ts";
import { type AnnotChange, boxToUser, deleteAnnotOp, kindName, type ToUser, updateAnnotOp } from "./ops.ts";
import { isBlank, needsFallback } from "./rich-text.ts";

/** How a box's text looks: what a new box takes from its tool, or an existing one holds. */
export interface TextLook {
  readonly font: AnnotFont;
  /** Points. */
  readonly size: number;
  readonly align: "left" | "center" | "right";
  /** The text's colour, and the border's and leader line's: RGB 0 to 1. */
  readonly colour: readonly [number, number, number];
  /** The box's background; none without. */
  readonly fill?: readonly [number, number, number];
  /** The border's width in points: 0 for none. */
  readonly border: number;
  /** A callout's line ending (`/LE`). */
  readonly end?: LineEndingName;
}

/** A box open for typing. */
export interface TextEdit {
  readonly page: number;
  readonly kind: TextKind;
  /** The annotation's ID; undefined for a box not yet made. */
  readonly id: string | undefined;
  /** The box the text is laid out in, page space. */
  readonly rect: Box;
  /** A callout's leader line in page space: the point it points at, the knee, the box's edge. */
  readonly line?: readonly Point[];
  readonly text: RichText;
  readonly look: TextLook;
  /** The box's fallback fonts, by their blob IDs. */
  readonly fallbacks: readonly string[];
}

/** What the box holds when editing ends. */
export interface TextNow {
  readonly text: RichText;
  readonly rect: Box;
  readonly line?: readonly Point[];
  /** The look the inspector gave the box while it was typed into; the box's own when left out. */
  readonly look?: TextLook;
}

/** What a box's edit needs besides the box: the page's mapping and turn, the fallback font, and the comment's author. */
export interface TextContext {
  /** The page's ID, which `addAnnot` takes. */
  readonly pageId: string;
  readonly toUser: ToUser;
  /** The page's `/Rotate`: a new box is upright as the page is shown. */
  readonly rotation: number;
  /** The fallback font's blob ID, once the document has it. */
  readonly fallback: string | undefined;
  readonly author?: string | undefined;
  readonly created: Date;
}

/** A new box's size before it is typed into, points: wide enough for a few words, one line tall. */
const NEW_BOX: Readonly<Record<TextKind, readonly [number, number]>> = {
  textBox: [180, 24],
  callout: [160, 24],
  typewriter: [120, 16],
};

/** Smaller than this, in points, a drag is a click and places a box of the default size. */
const SMALLEST = 8;

/** The box a press at `from` and a release at `to` place: dragged, or of the default size at the press. */
export function placedBox(kind: TextKind, from: Point, to: Point): Box {
  const [w, h] = NEW_BOX[kind];
  const [x0, y0] = [Math.min(from[0], to[0]), Math.min(from[1], to[1])];
  const [x1, y1] = [Math.max(from[0], to[0]), Math.max(from[1], to[1])];
  if (kind === "typewriter" || x1 - x0 < SMALLEST) return [from[0], from[1], from[0] + w, from[1] + h];
  return [x0, y0, x1, Math.max(y1, y0 + h)];
}

/** How far a callout's box sits from the point it points at when only clicked, points. */
const CALLOUT_OFFSET: Point = [48, -48];
/** The knee's distance from the box, points. */
const KNEE = 16;

/**
 * A callout's box and line from a press at the point it points at, `at`, and a release where the
 * box goes, `to` (a click puts it up and to the right). The line runs from `at` to the middle of
 * the box's nearer side, with its knee `KNEE` points out from that side.
 */
export function placedCallout(at: Point, to: Point): { rect: Box; line: Point[] } {
  const far = Math.hypot(to[0] - at[0], to[1] - at[1]) >= SMALLEST;
  const [w, h] = NEW_BOX.callout;
  const corner: Point = far ? to : [at[0] + CALLOUT_OFFSET[0], at[1] + CALLOUT_OFFSET[1] - h];
  const rect: Box = [corner[0], corner[1], corner[0] + w, corner[1] + h];
  return { rect, line: calloutLine(at, rect) };
}

/** The leader line from `at` to the box `rect`'s nearer side, through a knee. */
export function calloutLine(at: Point, rect: Box): Point[] {
  const [x0, y0, x1, y1] = rect;
  const [mx, my] = [(x0 + x1) / 2, (y0 + y1) / 2];
  if (at[0] < x0 || at[0] > x1) {
    const left = at[0] < x0;
    const edge: Point = [left ? x0 : x1, my];
    return [at, [edge[0] + (left ? -KNEE : KNEE), my], edge];
  }
  const above = at[1] < y0;
  const edge: Point = [mx, above ? y0 : y1];
  return [at, [mx, edge[1] + (above ? -KNEE : KNEE)], edge];
}

/** The spec's intent for a kind: a callout's line, ending in an open arrow unless `end` says otherwise. */
function intentOf(
  kind: TextKind,
  line: readonly Point[] | undefined,
  toUser: ToUser,
  end: LineEndingName = "openArrow",
) {
  if (kind === "typewriter") return { type: "typewriter" as const };
  if (kind === "callout" && line !== undefined && line.length >= 2) {
    return { type: "callout" as const, line: line.map((p) => toUser(p)), end };
  }
  return { type: "textBox" as const };
}

const sameColour = (a: readonly number[] | undefined, b: readonly number[] | undefined) =>
  (a === undefined) === (b === undefined) && (a ?? []).every((v, i) => Math.abs(v - (b?.[i] ?? 0)) < 0.001);

/** What changed of a box's look, as its spec patch's fields and `updateAnnot`'s colour. */
function lookChange(before: TextLook, now: TextLook, kind: TextKind) {
  return {
    spec: {
      ...(now.font === before.font ? {} : { font: now.font }),
      ...(now.size === before.size ? {} : { size: now.size }),
      ...(now.align === before.align ? {} : { align: now.align }),
      ...(sameColour(now.fill, before.fill) ? {} : { fill: now.fill === undefined ? null : rgb(now.fill) }),
      ...(now.border === before.border || kind === "typewriter" ? {} : { border: { width: now.border } }),
    },
    ...(sameColour(now.colour, before.colour) ? {} : { color: rgb(now.colour) }),
    endChanged: kind === "callout" && (now.end ?? "openArrow") !== (before.end ?? "openArrow"),
  };
}

/** The fallback fonts `text` needs: the box's own, and the fallback font when the text has Arabic. */
function fallbacksFor(text: RichText, own: readonly string[], fallback: string | undefined): string[] | undefined {
  if (needsFallback(text) && fallback !== undefined && !own.includes(fallback)) return [...own, fallback];
  return own.length > 0 ? [...own] : undefined;
}

const sameBox = (a: Box, b: Box) => a.every((v, i) => Math.abs(v - (b[i] ?? 0)) < 0.01);
const sameLine = (a: readonly Point[] | undefined, b: readonly Point[] | undefined) =>
  (a ?? []).length === (b ?? []).length &&
  (a ?? []).every((p, i) => sameBox([...p, 0, 0], [...(b?.[i] ?? [0, 0]), 0, 0]));
const sameText = (a: RichText, b: RichText) => JSON.stringify(a) === JSON.stringify(b);

/** The edit that ends typing into `edit` with `now`, and its name in the history; none when nothing changed. */
export function finishedEdit(
  edit: TextEdit,
  now: TextNow,
  context: TextContext,
): { readonly ops: Op[]; readonly label: string } | undefined {
  const kind = kindName("FreeText").toLocaleLowerCase();
  const blank = isBlank(now.text);
  if (edit.id === undefined) {
    if (blank) return undefined;
    return { ops: [addTextOp(edit, now, context)], label: laterMessage("comment.step.add", { kind }) };
  }
  if (blank) return { ops: [deleteAnnotOp(edit.id)], label: laterMessage("comment.step.delete", { kind }) };
  const change = changeOf(edit, now, context);
  return change === undefined
    ? undefined
    : {
        ops: [updateAnnotOp(edit.id, change)],
        label: laterMessage("comment.step.change", { kind }),
      };
}

/** What `updateAnnot` changes of the existing box `edit` to make it `now`; none when nothing changed. */
function changeOf(
  edit: TextEdit,
  now: TextNow,
  context: TextContext,
): Omit<AnnotChange, "id" | "modified"> | undefined {
  const fallbacks = fallbacksFor(now.text, edit.fallbacks, context.fallback);
  const look = lookChange(edit.look, now.look ?? edit.look, edit.kind);
  const reLined = edit.kind === "callout" && (!sameLine(now.line, edit.line) || look.endChanged);
  const spec = {
    ...(sameText(now.text, edit.text) ? {} : { text: now.text }),
    ...(sameBox(now.rect, edit.rect) ? {} : { rect: boxToUser(context.toUser, now.rect) }),
    ...(reLined
      ? { intent: intentOf("callout", now.line ?? edit.line, context.toUser, (now.look ?? edit.look).end) }
      : {}),
    ...(fallbacks === undefined ? {} : { fallbacks }),
    ...look.spec,
  };
  const changesSpec = Object.keys(spec).length > 0;
  if (!changesSpec && look.color === undefined) return undefined;
  return {
    ...(changesSpec ? { spec: { type: "freeText" as const, ...spec } } : {}),
    ...(look.color === undefined ? {} : { color: look.color }),
  };
}

/** RGB as an annotation colour. */
const rgb = ([r, g, b]: readonly [number, number, number]): [number, number, number] => [r, g, b];

/** The `addAnnot` that makes the new box `edit` holding `now`. */
export function addTextOp(edit: TextEdit, now: TextNow, context: TextContext): Op {
  const look = now.look ?? edit.look;
  const fallbacks = fallbacksFor(now.text, edit.fallbacks, context.fallback);
  const typewriter = edit.kind === "typewriter";
  const date = context.created;
  const spec = {
    type: "freeText" as const,
    rect: boxToUser(context.toUser, now.rect),
    text: now.text,
    font: look.font,
    size: look.size,
    align: look.align,
    intent: intentOf(edit.kind, now.line, context.toUser, look.end),
    ...(look.fill === undefined || typewriter ? {} : { fill: rgb(look.fill) }),
    border: { width: typewriter ? 0 : look.border },
    ...(context.rotation === 0 ? {} : { rotation: context.rotation }),
    ...(fallbacks === undefined ? {} : { fallbacks }),
  };
  const when = isoToPdfDate(date.toISOString()) ?? "";
  return {
    op: "addAnnot",
    args: {
      page: context.pageId,
      spec,
      common: {
        color: rgb(look.colour),
        ...(context.author === undefined || context.author === "" ? {} : { author: context.author }),
        created: when,
        modified: when,
      },
    },
  };
}

/** A box read back from the document (`textBox`) as a box open for typing, in page space. */
export function editOf(id: string, box: TextBox): TextEdit {
  const { spec } = box;
  const intent = spec.intent ?? { type: "textBox" };
  const kind: TextKind =
    intent.type === "callout" ? "callout" : intent.type === "typewriter" ? "typewriter" : "textBox";
  const [x0, y0, x1, y1] = box.rect;
  const fill = spec.fill;
  return {
    page: box.page,
    kind,
    id,
    rect: [x0, y0, x1, y1],
    ...(kind === "callout" ? { line: box.line.map(([x, y]) => [x, y] as const) } : {}),
    text: spec.text,
    look: {
      font: typeof spec.font === "string" ? spec.font : "helvetica",
      size: spec.size ?? 12,
      align: spec.align ?? "left",
      colour: rgbOf(box.color),
      ...(fill === undefined || fill === null ? {} : { fill: rgbOf(fill) }),
      border: kind === "typewriter" ? 0 : (spec.border?.width ?? 1),
      ...(intent.type === "callout" ? { end: intent.end ?? "openArrow" } : {}),
    },
    fallbacks: [...(spec.fallbacks ?? [])],
  };
}

/** An annotation colour as RGB: gray and CMYK converted, black when none. */
function rgbOf(colour: readonly number[] | undefined): readonly [number, number, number] {
  if (colour === undefined) return [0, 0, 0];
  if (colour.length === 1) return [colour[0] ?? 0, colour[0] ?? 0, colour[0] ?? 0];
  if (colour.length === 4) {
    const [c = 0, m = 0, y = 0, k = 0] = colour;
    return [(1 - c) * (1 - k), (1 - m) * (1 - k), (1 - y) * (1 - k)];
  }
  return [colour[0] ?? 0, colour[1] ?? 0, colour[2] ?? 0];
}
