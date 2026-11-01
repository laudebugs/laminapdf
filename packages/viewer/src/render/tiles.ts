// Progressive rendering's geometry (C3b, UI/UX plan §5.6): a low-resolution pass of the whole
// page first, then sharp tiles at full device pixel ratio over the part of the page in view. All
// pure: the view turns these into render requests and canvas draws.
import type { PixelRect } from "@laminapdf/render";

/** A tile's side in CSS px (256–512 per the plan; device px is this × DPR). */
export const TILE_CSS = 512;
/** The low-resolution pass: at most this many device px wide, and at most half the sharp scale. */
export const LOW_MAX_WIDTH = 256;

/** The render scale (output px per pt) of the low-resolution pass for a page `widthPt` wide. */
export function lowScale(sharpScale: number, widthPt: number): number {
  return Math.min(sharpScale * 0.5, LOW_MAX_WIDTH / Math.max(1, widthPt));
}

/**
 * The tiles (device px, in the page's pixel space at the sharp scale) covering `visible`, the
 * part of the page in view in CSS px (x, y from the page's top-left), clamped to the page.
 */
export function tilesFor(
  visible: { readonly x: number; readonly y: number; readonly width: number; readonly height: number },
  pageCss: { readonly width: number; readonly height: number },
  dpr: number,
): PixelRect[] {
  const pageW = Math.max(1, Math.floor(pageCss.width * dpr));
  const pageH = Math.max(1, Math.floor(pageCss.height * dpr));
  const side = Math.max(1, Math.round(TILE_CSS * dpr));
  const x0 = Math.max(0, Math.floor((visible.x * dpr) / side) * side);
  const y0 = Math.max(0, Math.floor((visible.y * dpr) / side) * side);
  const x1 = Math.min(pageW, Math.ceil(((visible.x + visible.width) * dpr) / side) * side);
  const y1 = Math.min(pageH, Math.ceil(((visible.y + visible.height) * dpr) / side) * side);
  const tiles: PixelRect[] = [];
  for (let y = y0; y < y1; y += side) {
    for (let x = x0; x < x1; x += side) {
      tiles.push({ x, y, width: Math.min(side, pageW - x), height: Math.min(side, pageH - y) });
    }
  }
  return tiles;
}
