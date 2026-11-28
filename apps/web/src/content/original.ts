// What Edit mode knows of the objects a page already drew (Paper A05): which ones Select can pick
// and how the pick is outlined, what a click or a marquee lands on, and the `EditContent` edits that
// move, delete, crop and restack a pick. Apart from the DOM, so Node tests it. An object is named
// by its ordinal and hash (`ObjectRef`), which change with every edit to the page, so a pick is
// read again after each one and followed by its place in the painting order.

import type { ContentEdit, ObjectRef, PageObject } from "@laminapdf/sdk";
import {
  type Box,
  boxAround,
  contains,
  corners,
  dragBox,
  frameOf,
  invert,
  type Matrix,
  multiply,
  type Point,
} from "./geometry.ts";
import type { Item } from "./objects.ts";

/** What a page object draws, as the engine names it. */
type OriginalKind = PageObject["kind"];

/** One object the page already drew, as Select sees it. */
export interface Original {
  readonly page: number;
  readonly ref: ObjectRef;
  readonly kind: OriginalKind;
  /** Its box in page space, before clipping. */
  readonly box: Box;
  /** Why it can't be changed here: the engine's reason, or `unsupported` for a kind that isn't edited yet. */
  readonly readOnly: string | undefined;
  /** It covers nearly the whole page (a background): a click goes through it unless Alt is held. */
  readonly background: boolean;
  /** The alternative text of the figure that draws it, when the file has any. */
  readonly alt: string | undefined;
}

/** A pick: one or more objects of one page, the group's box, and what the group is as a whole. */
export interface Picked {
  readonly page: number;
  readonly objects: readonly Original[];
  /** The box around them all. */
  readonly box: Box;
}

/** How a pick is named in the inspector and the announcements. */
export type PickKind = "image" | "shape" | "text" | "mixed";

/** A hit's slop around a thin object, in points. */
const SLOP = 2;

/** The kinds Select moves, resizes and deletes today. */
const EDITABLE: ReadonlySet<OriginalKind> = new Set(["path", "text", "image"]);

/** Whether two boxes are the same within half a point. */
function sameBox(a: Box, b: Box): boolean {
  return a.every((v, i) => Math.abs(v - (b[i] ?? 0)) < 0.5);
}

/** How much of the page an object covers before it counts as a background. */
const BACKGROUND = 0.9;

/** `box` cut to the page, or undefined when none of it is on the page. */
function onPage(box: Box, size: { readonly width: number; readonly height: number }): Box | undefined {
  const clipped: Box = [
    Math.max(box[0], 0),
    Math.max(box[1], 0),
    Math.min(box[2], size.width),
    Math.min(box[3], size.height),
  ];
  return clipped[2] < clipped[0] || clipped[3] < clipped[1] ? undefined : clipped;
}

/**
 * Page `page`'s objects as Select lists them, in painting order. Left out: those that draw
 * nothing, those wholly off the page (a hidden object takes no click), and a form that is one of
 * the added `items` (Lamina PDF's own objects are the controller's). A box is cut to the page. A
 * kind that isn't edited yet (a form, a shading) is listed as read-only, so it is outlined and
 * says why instead of being missed.
 */
export function originalsOn(
  page: number,
  objects: readonly PageObject[],
  added: readonly Item[],
  size: { readonly width: number; readonly height: number },
): Original[] {
  const ours = added.map((item) => boxAround(corners(item.frame)));
  const out: Original[] = [];
  for (const object of objects) {
    const bounds = object.bounds;
    const box = bounds === undefined ? undefined : onPage(bounds, size);
    if (bounds === undefined || box === undefined) continue;
    if (object.kind === "form" && ours.some((mine) => sameBox(mine, bounds))) continue;
    const area = (box[2] - box[0]) * (box[3] - box[1]);
    out.push({
      page,
      ref: object.object,
      kind: object.kind,
      box,
      readOnly: object.readOnly ?? (EDITABLE.has(object.kind) ? undefined : "unsupported"),
      background: area >= BACKGROUND * size.width * size.height,
      alt: object.alt,
    });
  }
  return out;
}

/** A pick of `objects` on `page`, if there are any. */
export function pickOf(page: number, objects: readonly Original[]): Picked | undefined {
  if (objects.length === 0) return undefined;
  return {
    page,
    objects,
    box: boxAround(
      objects.flatMap((o) => [
        { x: o.box[0], y: o.box[1] },
        { x: o.box[2], y: o.box[3] },
      ]),
    ),
  };
}

/** A box as the frame that is its rectangle (never flatter than a thousandth of a point). */
export function boxFrame(box: Box): Matrix {
  return frameOf(box[0], box[1], Math.max(box[2] - box[0], 1e-3), Math.max(box[3] - box[1], 1e-3));
}

/**
 * The topmost object under `point`: boxes are checked from the last drawn back, a few points of
 * slop around each. A background is passed over unless `through` (Alt held) says to take it.
 */
export function hitOriginal(originals: readonly Original[], point: Point, through = false): Original | undefined {
  for (let i = originals.length - 1; i >= 0; i--) {
    const object = originals[i];
    if (object === undefined || (object.background && !through)) continue;
    if (contains(boxFrame(object.box), point, SLOP)) return object;
  }
  return undefined;
}

/** The objects a marquee from `a` to `b` takes in: those whose box it holds whole, that can be changed and aren't a background. */
export function inMarquee(originals: readonly Original[], a: Point, b: Point): Original[] {
  const [x0, y0, x1, y1] = dragBox(a, b);
  return originals.filter(
    (o) =>
      o.readOnly === undefined && !o.background && o.box[0] >= x0 && o.box[1] >= y0 && o.box[2] <= x1 && o.box[3] <= y1,
  );
}

/** What a pick is: all images, all shapes, all text, or a mix. */
export function pickKind(pick: Picked): PickKind {
  const kinds = new Set(pick.objects.map((o) => (o.kind === "image" ? "image" : o.kind === "text" ? "text" : "shape")));
  const [only] = [...kinds];
  return kinds.size === 1 && only !== undefined ? (only as PickKind) : "mixed";
}

/** Why a pick can't be changed: the first object's reason, if any object has one. */
export function pickRefusal(pick: Picked): string | undefined {
  return pick.objects.find((o) => o.readOnly !== undefined)?.readOnly;
}

/**
 * The matrix that carries what drew in frame `from` into frame `to`: where an object drew a point
 * `p`, it now draws `matrix(p)` (the `Transform` edit's own form).
 */
export function carrying(from: Matrix, to: Matrix): Matrix | undefined {
  const inverse = invert(from);
  return inverse === undefined ? undefined : multiply(to, inverse);
}

/** The edits that move every object of `pick` as its group's frame goes from `from` to `to`. */
export function transformEdits(pick: Picked, from: Matrix, to: Matrix): ContentEdit[] {
  const matrix = carrying(from, to);
  if (matrix === undefined) return [];
  return pick.objects.map((o) => ({ transform: { object: o.ref, matrix: [...matrix] } }) as ContentEdit);
}

/** The edits that delete every object of `pick`. */
export function deleteEdits(pick: Picked): ContentEdit[] {
  return pick.objects.map((o) => ({ delete: { object: o.ref } }) as ContentEdit);
}

/** The edit that cuts an image to `rect` (page space). */
export function cropEdit(object: Original, rect: Box): ContentEdit {
  return { crop: { object: object.ref, rect: [...rect] } } as ContentEdit;
}

/** Where a pick goes in the stacking order. */
export type Arrange = "front" | "forward" | "backward" | "back";

/**
 * The order `moving` (ordinals) leave `order` (every ordinal of the page, painting order) in after
 * `how`: to the front or back, or one step past the nearest object that isn't moving. The one
 * rule for stacking: `reorderEdits` makes the edits that reach it, and a pick follows its
 * objects through it.
 */
export function restacked(order: readonly number[], moving: readonly number[], how: Arrange): number[] {
  const moves = new Set(moving);
  const picked = order.filter((n) => moves.has(n));
  const rest = order.filter((n) => !moves.has(n));
  if (how === "front") return [...rest, ...picked];
  if (how === "back") return [...picked, ...rest];
  const next = [...order];
  const forward = how === "forward";
  // The topmost first going up and the bottom-most first going down, so each passes what it must.
  for (const n of forward ? [...picked].reverse() : picked) {
    const at = next.indexOf(n);
    const others = forward ? next.slice(at + 1) : next.slice(0, at).reverse();
    const passed = others.find((m) => !moves.has(m));
    if (passed === undefined) continue;
    next.splice(at, 1);
    next.splice(next.indexOf(passed) + (forward ? 1 : 0), 0, n);
  }
  return next;
}

/**
 * The edits that restack `pick` within `objects` (every object the page draws, in painting order,
 * ours or not) as `restacked` says: each moved object goes after the one now below it, or to the
 * back. An object that stays above what it was above makes no edit.
 */
export function reorderEdits(pick: Picked, objects: readonly PageObject[], how: Arrange): ContentEdit[] {
  const ordinals = objects.map((o) => ordinalOf(o.object));
  const moving = pick.objects.map((o) => ordinalOf(o.ref));
  const next = restacked(ordinals, moving, how);
  const refOf = (n: number) => objects.find((o) => ordinalOf(o.object) === n)?.object;
  return next.flatMap((n, i) => {
    const object = refOf(n);
    const below = next[i - 1];
    if (object === undefined || !moving.includes(n) || below === ordinals[ordinals.indexOf(n) - 1]) return [];
    const to = below === undefined ? "back" : { after: refOf(below) };
    return [{ reorder: { object, to } } as ContentEdit];
  });
}

/** The pick of the objects with `ordinals` among `now`, on the pick's page. */
export function followed(page: number, ordinals: readonly number[], now: readonly Original[]): Picked | undefined {
  const found = now.filter((o) => ordinals.includes(ordinalOf(o.ref)));
  return pickOf(page, found);
}

/** An object name's ordinal. */
export function ordinalOf(ref: ObjectRef): number {
  return ref.ordinal;
}
