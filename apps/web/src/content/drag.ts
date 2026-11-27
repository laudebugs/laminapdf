// What a drag makes of an object's frame, apart from the DOM so Node can test it (Paper A02): a
// move snapped to the page's lines and the other objects, a handle's resize or turn, and a text
// box's width. The controller keeps the pointer's state and hands these the points.

import {
  boxAround,
  corners,
  type Guide,
  type HandleName,
  type Matrix,
  moved,
  type Point,
  resized,
  sizeOf,
  snapMove,
  turnedTo,
} from "./geometry.ts";

/** The narrowest a text box gets, in points. */
const NARROWEST_BOX = 24;

/** The frame of a move from `from` to `now`, pulled onto snap lines unless `noSnap`, with the guides it is on. */
export function movedFrame(
  frame: Matrix,
  from: Point,
  now: Point,
  page: { readonly width: number; readonly height: number } | undefined,
  others: readonly Matrix[],
  noSnap: boolean,
): { readonly frame: Matrix; readonly guides: readonly Guide[] } {
  const dx = now.x - from.x;
  const dy = now.y - from.y;
  if (page === undefined || noSnap) return { frame: moved(frame, dx, dy), guides: [] };
  const snapped = snapMove(
    boxAround(corners(frame)),
    dx,
    dy,
    page,
    others.map((o) => boxAround(corners(o))),
  );
  return { frame: moved(frame, snapped.dx, snapped.dy), guides: snapped.guides };
}

/** The frame of a text box whose `handle` (west or east) went from `from` to `now`: only its width changes, and the west handle moves its left edge. */
export function textFrame(frame: Matrix, handle: HandleName, from: Point, now: Point): Matrix {
  const { width } = sizeOf(frame);
  const dx = now.x - from.x;
  const next = Math.max(NARROWEST_BOX, width + (handle === "e" ? dx : -dx));
  const left = handle === "w" ? frame[4] + (width - next) : frame[4];
  return [next, 0, 0, frame[3], left, frame[5]];
}

/** The frame a handle's drag from `from` to `now` makes: a turn about the centre, a text box's width, or a resize; Shift keeps proportions (or turns in steps). */
export function handleFrame(
  frame: Matrix,
  isText: boolean,
  handle: HandleName | "turn",
  from: Point,
  now: Point,
  shift: boolean,
): Matrix {
  if (handle === "turn") return turnedTo(frame, now, shift);
  if (isText) return textFrame(frame, handle, from, now);
  return resized(frame, handle, now.x - from.x, now.y - from.y, shift);
}
