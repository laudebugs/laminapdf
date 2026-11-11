// PageLayout: prefix sums, pageAt and rotation, with its cost counted in steps.
import { describe, expect, it } from "vitest";
import { keptTop, PageLayout } from "./page-layout.js";

const LETTER = { width: 612, height: 792 };
const LANDSCAPE = { width: 792, height: 612 };

function sizes(n: number) {
  // Seeded variety: letter, A4, landscape, a tall strip.
  const kinds = [LETTER, { width: 595, height: 842 }, LANDSCAPE, { width: 300, height: 1200 }];
  return Array.from({ length: n }, (_, i) => kinds[(i * 7 + 3) % kinds.length] ?? LETTER);
}

describe("PageLayout", () => {
  it("builds 10,000 pages in one pass of n steps, and finds a page in log2(n) steps", () => {
    const layout = new PageLayout(sizes(10_000), { scale: 1 });
    expect(layout.operations).toBe(10_000);
    layout.operations = 0;
    layout.pageAt(layout.height / 2);
    expect(layout.operations).toBeLessThanOrEqual(Math.ceil(Math.log2(10_000)));
  });

  it("keeps prefix sums: each top is the previous top plus its height and the gap", () => {
    const layout = new PageLayout(sizes(50), { scale: 1.5, gap: 10 });
    for (let i = 1; i < 50; i++) {
      expect(layout.offsetOf(i)).toBeCloseTo(layout.offsetOf(i - 1) + (layout.heights[i - 1] as number) + 10, 9);
    }
    expect(layout.height).toBeCloseTo(layout.offsetOf(49) + (layout.heights[49] as number) + 10, 9);
  });

  it("finds the page at an offset, giving gaps to the page above", () => {
    const layout = new PageLayout([LETTER, LETTER, LETTER], { scale: 1, gap: 10 });
    expect(layout.pageAt(0)).toBe(0);
    expect(layout.pageAt(10 + 792 + 5)).toBe(0);
    expect(layout.pageAt(10 + 792 + 10)).toBe(1);
    expect(layout.pageAt(1e9)).toBe(2);
  });

  it("swaps width and height for a 90° view rotation", () => {
    const layout = new PageLayout([LETTER], { scale: 1, rotation: 90 });
    expect([layout.widths[0], layout.heights[0]]).toEqual([792, 612]);
  });
});

describe("keptTop", () => {
  const four = [LETTER, LETTER, LETTER, LETTER];
  // Page 1 turned a quarter: 612 pt tall instead of 792.
  const turned = [LETTER, LANDSCAPE, LETTER, LETTER];

  it("keeps a page below a turned one under the viewport's top: its offset moves up by the height lost", () => {
    const before = new PageLayout(four, { scale: 1, gap: 10 });
    const after = new PageLayout(turned, { scale: 1, gap: 10 });
    const top = before.offsetOf(2) + 100;
    expect(keptTop(before, after, top)).toBe(after.offsetOf(2) + 100);
    expect(after.offsetOf(2)).toBe(before.offsetOf(2) - (792 - 612));
  });

  it("keeps a page above a turned one exactly where it was", () => {
    const before = new PageLayout(four, { scale: 1, gap: 10 });
    const after = new PageLayout(turned, { scale: 1, gap: 10 });
    expect(keptTop(before, after, 300)).toBe(300);
  });

  it("scales the distance into the turned page itself by its new height", () => {
    const before = new PageLayout(four, { scale: 2, gap: 10 });
    const after = new PageLayout(turned, { scale: 2, gap: 10 });
    const top = before.offsetOf(1) + (before.heights[1] as number) / 2;
    expect(keptTop(before, after, top)).toBeCloseTo(after.offsetOf(1) + (after.heights[1] as number) / 2, 9);
  });

  it("leaves the offset alone when the layouts hold different pages", () => {
    const before = new PageLayout(four, { scale: 1 });
    const after = new PageLayout([LETTER], { scale: 1 });
    expect(keptTop(before, after, 1234)).toBe(1234);
  });
});
