import { describe, expect, it } from "vitest";
import { DESKTOP_BUDGETS, viewerBudgets } from "./budgets.ts";

/** The decoded bytes the budgets allow: 4 bytes a pixel in both pools, plus the cache. */
const total = (b: ReturnType<typeof viewerBudgets>) =>
  4 * ((b.pool.maxPixels ?? 0) + (b.detailPool.maxPixels ?? 0)) + b.cacheBytes;

describe("viewerBudgets", () => {
  it("keeps the desktop budgets for a fine pointer, whatever the memory", () => {
    expect(viewerBudgets({ coarse: false, memoryGb: 1 })).toBe(DESKTOP_BUDGETS);
  });

  it("caps every touch canvas at 5 MP and the total under iOS Safari's lowest canvas limit when the memory is unknown", () => {
    const b = viewerBudgets({ coarse: true });
    expect([b.backdropMaxPixels, total(b) < 224 * 1024 * 1024]).toEqual([5 * 1024 * 1024, true]);
  });

  it("gives a touch device less with less memory and more with more", () => {
    const [low, unknown, high] = [1, undefined, 8].map((memoryGb) => total(viewerBudgets({ coarse: true, memoryGb })));
    expect((low as number) < (unknown as number) && (unknown as number) < (high as number)).toBe(true);
  });
});
