// Zoom: the fit modes for portrait, landscape and rotated pages, the range, and anchoring.
import { describe, expect, it } from "vitest";
import { CSS_PX_PER_PT, PageLayout } from "../layout/page-layout.js";
import { percentOfScale, pointAt, scaleFor, scrollToPoint } from "./zoom.js";

const LETTER = { width: 612, height: 792 };
const LANDSCAPE = { width: 792, height: 612 };

function sizes(n: number) {
  // Seeded variety: letter, A4, landscape, a tall strip.
  const kinds = [LETTER, { width: 595, height: 842 }, LANDSCAPE, { width: 300, height: 1200 }];
  return Array.from({ length: n }, (_, i) => kinds[(i * 7 + 3) % kinds.length] ?? LETTER);
}

describe("zoom", () => {
  const viewport = { width: 1000, height: 800 };
  it("fits the width of portrait, landscape and rotated pages", () => {
    expect(scaleFor("fit-width", LETTER, 0, viewport, 16)).toBeCloseTo((1000 - 32) / 612, 9);
    expect(scaleFor("fit-width", LANDSCAPE, 0, viewport, 16)).toBeCloseTo((1000 - 32) / 792, 9);
    expect(scaleFor("fit-width", LETTER, 90, viewport, 16)).toBeCloseTo((1000 - 32) / 792, 9);
  });

  it("fits the whole page (the tighter of width and height)", () => {
    expect(scaleFor("fit-page", LETTER, 0, viewport, 16)).toBeCloseTo((800 - 32) / 792, 9);
    expect(scaleFor("fit-page", LANDSCAPE, 0, viewport, 16)).toBeCloseTo((1000 - 32) / 792, 9);
    expect(scaleFor("fit-page", LANDSCAPE, 270, viewport, 16)).toBeCloseTo((800 - 32) / 792, 9);
  });

  it("is 96/72 CSS px per point at actual size, and clamps percentages to 10–800 %", () => {
    expect(scaleFor("actual-size", LETTER, 0, viewport, 16)).toBe(CSS_PX_PER_PT);
    expect(percentOfScale(scaleFor(5, LETTER, 0, viewport, 16))).toBeCloseTo(10, 9);
    expect(percentOfScale(scaleFor(5000, LETTER, 0, viewport, 16))).toBeCloseTo(800, 9);
  });

  it("keeps the point under the anchor through 100 % → 200 % → 50 % within 1 CSS px", () => {
    const pages = sizes(40);
    const anchor = { x: 300, y: 250 };
    let layout = new PageLayout(pages, { scale: scaleFor(100, LETTER, 0, viewport, 16) });
    let scroll = { left: 0, top: 9_000 };
    const point = pointAt(layout, viewport.width, scroll, anchor);
    for (const percent of [200, 50]) {
      layout = new PageLayout(pages, { scale: scaleFor(percent, LETTER, 0, viewport, 16) });
      scroll = scrollToPoint(layout, viewport.width, point, anchor);
      const now = pointAt(layout, viewport.width, scroll, anchor);
      expect(now.page).toBe(point.page);
      const dy = Math.abs((now.y - point.y) * (layout.heights[point.page] as number));
      const dx = Math.abs((now.x - point.x) * (layout.widths[point.page] as number));
      expect(Math.max(dx, dy)).toBeLessThanOrEqual(1);
    }
  });
});
