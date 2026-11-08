// Progressive rendering's geometry: sharp tiles over the part in view, and the low pass.
import { DEFAULT_LIMITS, pagePixels, planTile, type RenderPage } from "@laminapdf/render";
import { describe, expect, it } from "vitest";
import { lowScale, tilesFor } from "./tiles.js";

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
  // on a 2× screen, CSS width × ratio came out one pixel wider than the renderer's page.
  it("plans only tiles the renderer accepts, at any zoom and pixel ratio", () => {
    const pages = [
      { width: 612, height: 792, rotate: 0 as const },
      { width: 595.28, height: 841.89, rotate: 0 as const },
    ];
    const ratios = [1, 1.25, 1.5, 1.75, 2, 2.2, 2.5, 3, 0.66, 0.88, 1.92];
    for (const page of pages) {
      for (const ratio of ratios) {
        for (let zoom = 10; zoom <= 800; zoom += 7) {
          for (const rotation of [0, 90] as const) expectWholePage(page, (zoom / 100) * (96 / 72), ratio, rotation);
        }
      }
    }
  });

  it("makes the low pass at most 256 px wide and half the sharp scale", () => {
    expect(lowScale(4, 612)).toBeCloseTo(256 / 612, 9);
    expect(lowScale(0.4, 612)).toBeCloseTo(0.2, 9);
  });
});

/** The tiles over the whole page at `cssScale` × `ratio` all pass the renderer's plan and reach its edges. */
function expectWholePage(page: RenderPage, cssScale: number, ratio: number, rotation: 0 | 90): void {
  const scale = cssScale * ratio;
  const size = pagePixels(page, scale, rotation);
  const [w, h] = rotation === 90 ? [page.height, page.width] : [page.width, page.height];
  const tiles = tilesFor({ x: 0, y: 0, width: w * cssScale, height: h * cssScale }, size, ratio);
  for (const clip of tiles) {
    expect(() => planTile(page, { page: 0, scale, rotation, clip }, DEFAULT_LIMITS)).not.toThrow();
  }
  expect(Math.max(...tiles.map((t) => t.x + t.width))).toBe(size.width);
  expect(Math.max(...tiles.map((t) => t.y + t.height))).toBe(size.height);
}
