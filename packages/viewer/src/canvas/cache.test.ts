// BitmapCache: farthest-first eviction, and closing what it evicts or replaces.
import { describe, expect, it } from "vitest";
import { BitmapCache } from "./cache.js";

/** A stand-in for an ImageBitmap of `w` × `h` that records `close()`. */
function bitmap(w: number, h: number): ImageBitmap & { closed: boolean } {
  const b = {
    width: w,
    height: h,
    closed: false,
    close: () => {
      b.closed = true;
    },
  };
  return b as unknown as ImageBitmap & { closed: boolean };
}

describe("BitmapCache", () => {
  it("evicts the bitmaps farthest from the current page first, and closes them", () => {
    const cache = new BitmapCache(3 * 100 * 100 * 4);
    const far = bitmap(100, 100);
    cache.set("p0", 0, far, 5);
    cache.set("p5", 5, bitmap(100, 100), 5);
    cache.set("p6", 6, bitmap(100, 100), 5);
    cache.set("p4", 4, bitmap(100, 100), 5);
    expect(cache.size).toBe(3);
    expect(cache.get("p0")).toBeUndefined();
    expect((far as unknown as { closed: boolean }).closed).toBe(true);
    expect(cache.bytes).toBe(3 * 100 * 100 * 4);
  });

  it("doesn't keep (nor close) a bitmap larger than the budget, and closes a replaced one", () => {
    const cache = new BitmapCache(100 * 100 * 4);
    const big = bitmap(200, 200);
    expect(cache.set("big", 0, big, 0)).toBe(false);
    expect(cache.size).toBe(0);
    expect(big.closed).toBe(false);
    const old = bitmap(10, 10);
    cache.set("k", 0, old, 0);
    cache.set("k", 0, bitmap(10, 10), 0);
    expect(old.closed).toBe(true);
  });
});
