// Where a floating layer goes against an anchor rectangle: centred on it, above when it fits and
// below when it doesn't, and kept a margin inside the bounds (the viewport, or the canvas inside
// it). Pure numbers, so its rules are tested in Node.

/** A rectangle in viewport coordinates, in CSS px. */
export interface Box {
  readonly top: number;
  readonly left: number;
  readonly width: number;
  readonly height: number;
}

/** Where the layer goes, and which side of the anchor it ended on. */
export interface Placement {
  readonly top: number;
  readonly left: number;
  readonly side: "above" | "below";
}

/**
 * Places a layer of `width` × `height` above `anchor`, `gap` away, flipping below when there is no
 * room above; then shifts it to stay `margin` inside `bounds` on every side it can. Null when no
 * part of the anchor is inside the bounds, as when the selection has scrolled away: the layer
 * would point at nothing.
 */
export function placeAbove(
  anchor: Box,
  width: number,
  height: number,
  bounds: Box,
  gap: number,
  margin: number,
): Placement | null {
  return placeBeside(anchor, width, height, bounds, gap, margin, "above");
}

/**
 * {@link placeAbove}, but below `anchor` first, flipping above when there is no room below: a link
 * preview goes under its link (§7.27), never over it.
 */
export function placeBelow(
  anchor: Box,
  width: number,
  height: number,
  bounds: Box,
  gap: number,
  margin: number,
): Placement | null {
  return placeBeside(anchor, width, height, bounds, gap, margin, "below");
}

function placeBeside(
  anchor: Box,
  width: number,
  height: number,
  bounds: Box,
  gap: number,
  margin: number,
  prefer: "above" | "below",
): Placement | null {
  const outside =
    anchor.top + anchor.height < bounds.top ||
    anchor.top > bounds.top + bounds.height ||
    anchor.left + anchor.width < bounds.left ||
    anchor.left > bounds.left + bounds.width;
  if (outside) return null;
  const top = bounds.top + margin;
  const bottom = bounds.top + bounds.height - margin;
  const above = anchor.top - gap - height;
  const below = anchor.top + anchor.height + gap;
  const side =
    prefer === "above"
      ? above >= top || below + height > bottom
        ? "above"
        : "below"
      : below + height <= bottom || above < top
        ? "below"
        : "above";
  const wanted = side === "above" ? above : below;
  const left = anchor.left + anchor.width / 2 - width / 2;
  return {
    top: clamp(wanted, top, bottom - height),
    left: clamp(left, bounds.left + margin, bounds.left + bounds.width - margin - width),
    side,
  };
}

/** `value` within [`low`, `high`]; `low` wins when the range is empty, as when the layer is too big. */
function clamp(value: number, low: number, high: number): number {
  return Math.max(low, Math.min(value, high));
}

/** The part of `a` inside `b`. */
export function intersect(a: Box, b: Box): Box {
  const top = Math.max(a.top, b.top);
  const left = Math.max(a.left, b.left);
  return {
    top,
    left,
    width: Math.max(0, Math.min(a.left + a.width, b.left + b.width) - left),
    height: Math.max(0, Math.min(a.top + a.height, b.top + b.height) - top),
  };
}

/**
 * The box a selection toolbar is placed against: every rectangle of the selection, so the toolbar
 * goes above the first line or below the last one and never over a selected line. Null for no
 * rectangles.
 */
export function selectionBox(rects: readonly Box[]): Box | null {
  const drawn = rects.filter((r) => r.width > 0 && r.height > 0);
  if (drawn.length === 0) return null;
  const top = Math.min(...drawn.map((r) => r.top));
  const left = Math.min(...drawn.map((r) => r.left));
  const bottom = Math.max(...drawn.map((r) => r.top + r.height));
  const right = Math.max(...drawn.map((r) => r.left + r.width));
  return { top, left, width: right - left, height: bottom - top };
}
