// Progressive rendering's geometry: sharp tiles over the part in view, and the low pass.
import { describe, expect, it } from "vitest";
import { lowScale, tilesFor } from "./tiles.js";

describe("tiles", () => {
  it("covers the part in view with tiles at most 512 CSS px × DPR on a side, clamped to the page", () => {
    const tiles = tilesFor({ x: 0, y: 600, width: 816, height: 500 }, { width: 816, height: 1056 }, 2);
    expect(tiles.every((t) => t.width <= 1024 && t.height <= 1024)).toBe(true);
    const maxY = Math.max(...tiles.map((t) => t.y + t.height));
    expect(maxY).toBe(2112);
    expect(Math.min(...tiles.map((t) => t.y))).toBe(1024);
  });

  it("makes the low pass at most 256 px wide and half the sharp scale", () => {
    expect(lowScale(4, 612)).toBeCloseTo(256 / 612, 9);
    expect(lowScale(0.4, 612)).toBeCloseTo(0.2, 9);
  });
});
