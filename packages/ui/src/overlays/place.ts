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
 * room above; then shifts it to stay `margin` inside `bounds` on every side it can.
 */
export function placeAbove(
  anchor: Box,
  width: number,
  height: number,
  bounds: Box,
  gap: number,
  margin: number,
): Placement {
  const top = bounds.top + margin;
  const bottom = bounds.top + bounds.height - margin;
  const above = anchor.top - gap - height;
  const below = anchor.top + anchor.height + gap;
  const side = above >= top || below + height > bottom ? "above" : "below";
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
