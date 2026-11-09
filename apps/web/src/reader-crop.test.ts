// A crop's clip in pixels: the box's pixels, rounded outwards, kept inside the page's.
import { describe, expect, it } from "vitest";
import { cropClip } from "./reader-crop.ts";

const A4 = { width: 595.28, height: 841.89 };

describe("cropClip", () => {
  it("rounds a box's pixels outwards", () => {
    expect(cropClip([10.4, 20.6, 100.2, 50.1], 2, A4)).toEqual({ x: 20, y: 41, width: 202, height: 101 });
  });

  it("keeps a whole page's clip inside the page the renderer rounds down", () => {
    // ceil(595.28 × 2) is 1191, but the page is floor(595.28 × 2) = 1190 pixels wide.
    expect(cropClip([0, 0, A4.width, A4.height], 2, A4)).toEqual({ x: 0, y: 0, width: 1190, height: 1683 });
  });

  it("rounds outwards without a page size", () => {
    expect(cropClip([0, 0, 595.28, 841.89], 2, undefined)).toEqual({ x: 0, y: 0, width: 1191, height: 1684 });
  });
});
