// Progressive rendering's geometry (UI/UX plan §5.6): a low-resolution pass of the whole
// page first, then sharp tiles at full device pixel ratio over the part of the page in view. All
// pure: the view turns these into render requests and canvas draws.
import type { PixelRect } from "@laminapdf/render";

/** A tile's side in CSS px (256–512 per the plan; device px is this × DPR). */
const TILE_CSS = 512;
/** The low-resolution pass: at most this many device px wide, and at most half the sharp scale. */
const LOW_MAX_WIDTH = 256;

/** The render scale (output px per pt) of the low-resolution pass for a page `widthPt` wide. */
export function lowScale(sharpScale: number, widthPt: number): number {
  return Math.min(sharpScale * 0.5, LOW_MAX_WIDTH / Math.max(1, widthPt));
}

/**
 * The sharp scale (device px per pt) of a page `css` CSS px in size at layout scale `layoutScale`:
 * `layoutScale × dpr`, lowered when the canvas pool could only `fit` fewer pixels. The canvas is
 * then `pagePixels` at this scale, which never exceeds `fitted` on either side.
 */
export function sharpScale(
  layoutScale: number,
  dpr: number,
  css: { readonly width: number; readonly height: number },
  fitted: { readonly width: number; readonly height: number },
): number {
  return layoutScale * Math.min(dpr, fitted.width / css.width, fitted.height / css.height);
}

/**
 * The tiles (device px, in the page's pixel space at the sharp scale) covering `visible`, the
 * part of the page in view in CSS px (x, y from the page's top-left), clamped to `page`: the
 * whole page in device px, as the renderer sizes it (`pagePixels` of `@laminapdf/render`), so no
 * tile reaches past it. `ratio` is device px per CSS px.
 */
export function tilesFor(
  visible: { readonly x: number; readonly y: number; readonly width: number; readonly height: number },
  page: { readonly width: number; readonly height: number },
  ratio: number,
): PixelRect[] {
  const side = Math.max(1, Math.round(TILE_CSS * ratio));
  const x0 = Math.max(0, Math.floor((visible.x * ratio) / side) * side);
  const y0 = Math.max(0, Math.floor((visible.y * ratio) / side) * side);
  const x1 = Math.min(page.width, Math.ceil(((visible.x + visible.width) * ratio) / side) * side);
  const y1 = Math.min(page.height, Math.ceil(((visible.y + visible.height) * ratio) / side) * side);
  const tiles: PixelRect[] = [];
  for (let y = y0; y < y1; y += side) {
    for (let x = x0; x < x1; x += side) {
      tiles.push({ x, y, width: Math.min(side, page.width - x), height: Math.min(side, page.height - y) });
    }
  }
  return tiles;
}
