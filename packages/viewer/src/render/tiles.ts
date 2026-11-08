// Progressive rendering's geometry (UI/UX plan §5.6): a low-resolution pass of the whole
// page first, then sharp tiles at full device pixel ratio over the part of the page in view. All
// pure: the view turns these into render requests and canvas draws.
import type { PixelRect } from "@laminapdf/render";

/** A tile's side in CSS px (256–512 per the plan; device px is this × DPR). */
const TILE_CSS = 512;
/** The low-resolution pass: at most this many device px wide, and at most half the sharp scale. */
const LOW_MAX_WIDTH = 256;

/**
 * The most device pixels a page's backdrop canvas holds: well below Safari's 16.7 MP per canvas,
 * and small enough that the mounted pages share the pool. A page larger than this at the full
 * pixel ratio keeps its backdrop at a lower one and gets a detail canvas over the part in view.
 */
export const BACKDROP_MAX_PIXELS = 8 * 1024 * 1024;

/**
 * Device px per CSS px of the backdrop of a page `css` CSS px in size: `dpr`, or less so that the
 * whole page fits {@link BACKDROP_MAX_PIXELS}.
 */
export function backdropRatio(dpr: number, css: { readonly width: number; readonly height: number }): number {
  const pixels = css.width * dpr * css.height * dpr;
  return pixels <= BACKDROP_MAX_PIXELS ? dpr : dpr * Math.sqrt(BACKDROP_MAX_PIXELS / pixels);
}

/** The smallest rectangle covering every one of `rects`; undefined when there are none. */
export function bounds(rects: readonly PixelRect[]): PixelRect | undefined {
  const [first] = rects;
  if (first === undefined) return undefined;
  let [x0, y0, x1, y1] = [first.x, first.y, first.x + first.width, first.y + first.height];
  for (const r of rects) {
    x0 = Math.min(x0, r.x);
    y0 = Math.min(y0, r.y);
    x1 = Math.max(x1, r.x + r.width);
    y1 = Math.max(y1, r.y + r.height);
  }
  return { x: x0, y: y0, width: x1 - x0, height: y1 - y0 };
}

/** Whether `outer` contains all of `inner`. */
export function contains(outer: PixelRect, inner: PixelRect): boolean {
  return (
    inner.x >= outer.x &&
    inner.y >= outer.y &&
    inner.x + inner.width <= outer.x + outer.width &&
    inner.y + inner.height <= outer.y + outer.height
  );
}

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
  if (visible.width <= 0 || visible.height <= 0) return [];
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
