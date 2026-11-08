// Progressive rendering's geometry: sharp tiles over the part in view, and the low pass.
import { DEFAULT_LIMITS, pagePixels, planTile, type RenderPage } from "@laminapdf/render";
import { describe, expect, it } from "vitest";
import { BACKDROP_MAX_PIXELS, backdropRatio, bounds, contains, lowScale, sharpScale, tilesFor } from "./tiles.js";

describe("tiles", () => {
  it("covers the part in view with tiles at most 512 CSS px × DPR on a side, clamped to the page", () => {
    const tiles = tilesFor({ x: 0, y: 600, width: 816, height: 500 }, { width: 1632, height: 2112 }, 2);
    expect(tiles.every((t) => t.width <= 1024 && t.height <= 1024)).toBe(true);
    const maxY = Math.max(...tiles.map((t) => t.y + t.height));
    expect(maxY).toBe(2112);
    expect(Math.min(...tiles.map((t) => t.y))).toBe(1024);
  });

  // The renderer refuses a tile that reaches past the page as it sizes it (f32, floored), and the
  // page stayed blurry where that tile and the ones after it belonged. At 153 %, 175 % and 250 %
  // on a 2× screen, the canvas came out one pixel wider than the renderer's page. The sweep sizes
  // the canvas as the viewer does: the pool's fit (rounded, or cut down by its pixel cap by `cap`),
  // then sharpScale and pagePixels.
  it("plans only tiles the renderer accepts, at any zoom, pixel ratio and pool cap", () => {
    const pages = [
      { width: 612, height: 792, rotate: 0 as const },
      { width: 595.28, height: 841.89, rotate: 0 as const },
    ];
    const zooms = Array.from({ length: 113 }, (_, i) => 10 + 7 * i);
    const cases = pages.flatMap((page) =>
      [1, 1.25, 1.5, 1.75, 2, 2.2, 2.5, 3].flatMap((dpr) =>
        [1, 0.96, 0.88, 0.66].flatMap((cap) =>
          zooms.flatMap((zoom) => ([0, 90] as const).map((rotation) => ({ page, dpr, cap, zoom, rotation }))),
        ),
      ),
    );
    for (const { page, dpr, cap, zoom, rotation } of cases) {
      expectWholePage(page, (zoom / 100) * (96 / 72), dpr, cap, rotation);
    }
  });

  it("keeps a backdrop at the full ratio until the page passes the pixel cap, then fits the cap", () => {
    const letter = (zoom: number) => ({ width: 816 * zoom, height: 1056 * zoom });
    expect(backdropRatio(2, letter(1))).toBe(2);
    const big = letter(4);
    const ratio = backdropRatio(2, big);
    expect(ratio).toBeLessThan(2);
    expect(big.width * ratio * big.height * ratio).toBeCloseTo(BACKDROP_MAX_PIXELS, -2);
  });

  // The detail canvas is the bounds of the sharp tiles in view: it holds every one of them, stays
  // inside the page, and its size follows the viewport, not the zoom.
  it("covers the tiles in view with a detail rectangle the size of the viewport, at any zoom", () => {
    const page = { width: 612, height: 792, rotate: 0 as const };
    const viewport = { width: 1440, height: 900 };
    for (const dpr of [1, 2, 3]) {
      for (let zoom = 150; zoom <= 800; zoom += 50) {
        expectDetail(page, (zoom / 100) * (96 / 72), dpr, viewport);
      }
    }
  });

  it("makes the low pass at most 256 px wide and half the sharp scale", () => {
    expect(lowScale(4, 612)).toBeCloseTo(256 / 612, 9);
    expect(lowScale(0.4, 612)).toBeCloseTo(0.2, 9);
  });
});

/**
 * Sizes the canvas of `page` at layout scale `cssScale` as the viewer does, with the pool fitting
 * `cap` of the pixels asked for (1: all of them, rounded), then checks that the canvas stays within
 * the pool's fit and that the tiles over the whole page all pass the renderer's plan and reach its edges.
 */
function expectWholePage(page: RenderPage, cssScale: number, dpr: number, cap: number, rotation: 0 | 90): void {
  const [w, h] = rotation === 90 ? [page.height, page.width] : [page.width, page.height];
  const css = { width: w * cssScale, height: h * cssScale };
  const fit = (side: number) => Math.max(1, cap === 1 ? Math.round(side * dpr) : Math.floor(side * dpr * cap));
  const fitted = { width: fit(css.width), height: fit(css.height) };
  const scale = sharpScale(cssScale, dpr, css, fitted);
  const size = pagePixels(page, scale, rotation);
  expect(size.width).toBeLessThanOrEqual(fitted.width);
  expect(size.height).toBeLessThanOrEqual(fitted.height);
  const tiles = tilesFor({ x: 0, y: 0, ...css }, size, scale / cssScale);
  for (const clip of tiles) {
    expect(() => planTile(page, { page: 0, scale, rotation, clip }, DEFAULT_LIMITS)).not.toThrow();
  }
  expect(Math.max(...tiles.map((t) => t.x + t.width))).toBe(size.width);
  expect(Math.max(...tiles.map((t) => t.y + t.height))).toBe(size.height);
}

/** The detail rectangle over a viewport-sized window in the middle of `page` holds its tiles and fits the page. */
function expectDetail(page: RenderPage, cssScale: number, dpr: number, viewport: { width: number; height: number }) {
  const scale = cssScale * dpr;
  const size = pagePixels(page, scale);
  const css = { width: page.width * cssScale, height: page.height * cssScale };
  const width = Math.min(viewport.width, css.width);
  const height = Math.min(viewport.height, css.height);
  const visible = { x: (css.width - width) / 2, y: (css.height - height) / 2, width, height };
  const tiles = tilesFor(visible, size, dpr);
  const rect = bounds(tiles);
  expect(rect).toBeDefined();
  if (rect === undefined) return;
  expect(contains({ x: 0, y: 0, ...size }, rect)).toBe(true);
  for (const clip of tiles) {
    expect(contains(rect, clip)).toBe(true);
    expect(() => planTile(page, { page: 0, scale, clip }, DEFAULT_LIMITS)).not.toThrow();
  }
  // At most one tile more than the window on each axis: the size follows the viewport.
  const side = Math.round(512 * dpr);
  expect(rect.width).toBeLessThanOrEqual(width * dpr + 2 * side);
  expect(rect.height).toBeLessThanOrEqual(height * dpr + 2 * side);
}
