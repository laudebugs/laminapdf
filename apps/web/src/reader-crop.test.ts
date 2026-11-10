// A crop's clip in pixels (the box's pixels, rounded outwards, kept inside the page's) and its
// scale (lowered to stay inside the renderer's limits).
import { describe, expect, it } from "vitest";
import { cropClip, cropScale } from "./reader-crop.ts";

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

describe("cropScale", () => {
  it("keeps the scale asked for when the crop fits the limits", () => {
    expect(cropScale([0, 0, A4.width, A4.height], 4)).toBe(4);
  });

  it("lowers the scale until a large page's crop is under the pixel limit", () => {
    // A0 at 4 pixels per point would be 9,536 × 13,484 pixels, about 129 MP.
    const box = [0, 0, 2384, 3370] as const;
    const scale = cropScale(box, 4);
    const clip = cropClip(box, scale, { width: 2384, height: 3370 });
    expect(clip.width * clip.height).toBeLessThanOrEqual(16_777_216);
  });

  it("lowers the scale until a long strip is under the side limit", () => {
    const box = [0, 0, 100, 14_400] as const;
    const clip = cropClip(box, cropScale(box, 4), undefined);
    expect(Math.max(clip.width, clip.height)).toBeLessThanOrEqual(16_384);
  });
});
