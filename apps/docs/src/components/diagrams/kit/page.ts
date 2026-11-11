/** US Letter, in points: the page the page-space and reading-order sketches draw. */
export const LETTER = { w: 612, h: 792 } as const;

/** A rect in points from a page's top-left corner. */
export interface Rect {
  x0: number;
  y0: number;
  x1: number;
  y1: number;
}

/** `r` drawn on a page whose top-left corner is at (x, y), `s` drawing units to a point. */
export function scaleRect(x: number, y: number, s: number, r: Rect) {
  return { x: x + r.x0 * s, y: y + r.y0 * s, w: (r.x1 - r.x0) * s, h: (r.y1 - r.y0) * s };
}
