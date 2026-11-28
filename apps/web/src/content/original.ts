// What Edit mode knows of the objects a page already drew (Paper A05): which ones Select can pick
// and how the pick is outlined, what a click or a marquee lands on, and the `EditContent` edits that
// move, delete, crop and restack a pick. Apart from the DOM, so Node tests it. An object is named
// by its ordinal and hash (`ObjectRef`), which change with every edit to the page, so a pick is
// read again after each one and followed by its place in the painting order.

import type { ContentEdit, ObjectRef, PageObject } from "@laminapdf/sdk";
import { type Box, boxAround, corners, frameOf, invert, type Matrix, multiply, type Point } from "./geometry.ts";
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

/**
 * Page `page`'s objects as Select lists them, in painting order: those that draw nothing are left
 * out, and so is a form that is one of the added `items` (Lamina PDF's own objects are the
 * controller's). A kind that isn't edited yet (a form, a shading) is listed as read-only, so it is
 * outlined and says why instead of being missed.
 */
export function originalsOn(page: number, objects: readonly PageObject[], added: readonly Item[]): Original[] {
  const ours = added.map((item) => boxAround(corners(item.frame)));
  const out: Original[] = [];
  for (const object of objects) {
    const box = object.bounds;
    if (box === undefined) continue;
    if (object.kind === "form" && ours.some((mine) => sameBox(mine, box))) continue;
    out.push({
      page,
      ref: object.object,
      kind: object.kind,
      box,
      readOnly: object.readOnly ?? (EDITABLE.has(object.kind) ? undefined : "unsupported"),
    });
  }
  return out;
}

/** The box of a pick of `objects`. */
function unionOf(objects: readonly Original[]): Box {
  const [first] = objects;
  if (first === undefined) return [0, 0, 0, 0];
  return boxAround(objects.flatMap((o) => [pointOf(o.box, 0), pointOf(o.box, 1)]));
}

function pointOf(box: Box, corner: 0 | 1): Point {
  return corner === 0 ? { x: box[0], y: box[1] } : { x: box[2], y: box[3] };
}

/** A pick of `objects` on `page`, if there are any. */
export function pickOf(page: number, objects: readonly Original[]): Picked | undefined {
  return objects.length === 0 ? undefined : { page, objects, box: unionOf(objects) };
}

/** Whether `point` is in `box`, `slop` points outside counting. */
function inBox(box: Box, point: Point, slop: number): boolean {
  return point.x >= box[0] - slop && point.x <= box[2] + slop && point.y >= box[1] - slop && point.y <= box[3] + slop;
}

/** The topmost object under `point`: boxes are checked from the last drawn back, a few points of slop around each. */
export function hitOriginal(originals: readonly Original[], point: Point): Original | undefined {
  for (let i = originals.length - 1; i >= 0; i--) {
    const object = originals[i];
    if (object !== undefined && inBox(object.box, point, SLOP)) return object;
  }
  return undefined;
}

/** The objects a marquee from `a` to `b` takes in: those whose box it holds whole, and can be changed. */
export function inMarquee(originals: readonly Original[], a: Point, b: Point): Original[] {
  const [x0, y0, x1, y1] = boxAround([a, b]);
  return originals.filter(
    (o) => o.readOnly === undefined && o.box[0] >= x0 && o.box[1] >= y0 && o.box[2] <= x1 && o.box[3] <= y1,
  );
}

/** What a pick is: all images, all shapes, all text, or a mix. */
export function pickKind(pick: Picked): PickKind {
  const kinds = new Set(pick.objects.map((o) => (o.kind === "path" ? "shape" : o.kind === "image" ? "image" : "text")));
  const [only] = [...kinds];
  return kinds.size === 1 && only !== undefined ? (only as PickKind) : "mixed";
}

/** Why a pick can't be changed: the first object's reason, if any object has one. */
export function pickRefusal(pick: Picked): string | undefined {
  return pick.objects.find((o) => o.readOnly !== undefined)?.readOnly;
}

/** A box as the frame that is its rectangle (never flatter than a thousandth of a point). */
export function boxFrame(box: Box): Matrix {
  return frameOf(box[0], box[1], Math.max(box[2] - box[0], 1e-3), Math.max(box[3] - box[1], 1e-3));
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
  return pick.objects.map((o) => ({ transform: { object: o.ref, matrix: [...matrix] } }) as unknown as ContentEdit);
}

/** The edits that delete every object of `pick`. */
export function deleteEdits(pick: Picked): ContentEdit[] {
  return pick.objects.map((o) => ({ delete: { object: o.ref } }) as unknown as ContentEdit);
}

/** The edit that cuts an image to `rect` (page space). */
export function cropEdit(object: Original, rect: Box): ContentEdit {
  return { crop: { object: object.ref, rect: [...rect] } } as unknown as ContentEdit;
}

/** Where a pick goes in the stacking order. */
export type Arrange = "front" | "forward" | "backward" | "back";

/**
 * The edits that restack `pick` within `page`'s `objects` (every object the page draws, in
 * painting order, ours or not): to the front or back, or one step past the nearest object that
 * isn't in the pick. A step with nothing to pass makes no edit.
 */
export function reorderEdits(pick: Picked, objects: readonly PageObject[], how: Arrange): ContentEdit[] {
  const names = new Set(pick.objects.map((o) => key(o.ref)));
  const order = objects.filter((o) => names.has(key(o.object)));
  const edit = (object: ObjectRef, to: unknown): ContentEdit => ({ reorder: { object, to } }) as unknown as ContentEdit;
  if (how === "front" || how === "back") return order.map((o) => edit(o.object, how));
  const edits: ContentEdit[] = [];
  if (how === "forward") {
    // From the topmost down, so each lands above the one that moved before it.
    for (const o of [...order].reverse()) {
      const at = objects.findIndex((x) => key(x.object) === key(o.object));
      const over = objects.slice(at + 1).find((x) => !names.has(key(x.object)));
      if (over !== undefined) edits.push(edit(o.object, { after: over.object }));
    }
    return edits;
  }
  for (const o of order) {
    const at = objects.findIndex((x) => key(x.object) === key(o.object));
    const under = objects
      .slice(0, at)
      .reverse()
      .find((x) => !names.has(key(x.object)));
    if (under !== undefined) edits.push(edit(o.object, { before: under.object }));
  }
  return edits;
}

/** A name as a map key. */
function key(ref: ObjectRef): string {
  return JSON.stringify(ref);
}

/**
 * The ordinals `moving` have after `how` restacks them among `order` (every ordinal of the page, in
 * painting order): the same steps `reorderEdits` makes, so a pick can follow its objects.
 */
export function restacked(order: readonly number[], moving: readonly number[], how: Arrange): number[] {
  const moves = new Set(moving);
  const picked = order.filter((n) => moves.has(n));
  const rest = order.filter((n) => !moves.has(n));
  if (how === "front") return [...rest, ...picked];
  if (how === "back") return [...picked, ...rest];
  const next = [...order];
  const take = how === "forward" ? [...picked].reverse() : picked;
  for (const n of take) {
    const at = next.indexOf(n);
    const to = how === "forward" ? next.findIndex((m, i) => i > at && !moves.has(m)) : lastBefore(next, at, moves);
    if (to < 0) continue;
    next.splice(at, 1);
    next.splice(how === "forward" ? to : to, 0, n);
  }
  return next;
}

/** The index of the last entry of `list` before `at` that isn't in `skip`, or -1. */
function lastBefore(list: readonly number[], at: number, skip: ReadonlySet<number>): number {
  for (let i = at - 1; i >= 0; i--) if (!skip.has(list[i] ?? -1)) return i;
  return -1;
}

/** The pick of the objects with `ordinals` among `now`, on the pick's page. */
export function followed(page: number, ordinals: readonly number[], now: readonly Original[]): Picked | undefined {
  const found = now.filter((o) => ordinals.includes(ordinalOf(o.ref)));
  return pickOf(page, found);
}

/** An object name's ordinal. */
export function ordinalOf(ref: ObjectRef): number {
  return (ref as unknown as { ordinal: number }).ordinal;
}
