// CanvasPool (C3b): the canvas count and the total pixel cap.
import { describe, expect, it } from "vitest";
import { CanvasPool } from "./pool.js";

describe("CanvasPool", () => {
  const fake = () => ({ width: 300, height: 150, remove() {} }) as unknown as HTMLCanvasElement;
  it("gives at most `max` canvases, reuses released ones reset to 1×1", () => {
    const pool = new CanvasPool({ max: 2, create: fake });
    const a = pool.acquire();
    const b = pool.acquire();
    expect(pool.acquire()).toBeUndefined();
    if (a === undefined || b === undefined) throw new Error("expected canvases");
    a.width = 500;
    pool.release(a);
    const c = pool.acquire();
    expect(c).toBe(a);
    expect([c?.width, c?.height]).toEqual([1, 1]);
    expect(pool.peak).toBe(2);
  });

  it("scales a canvas down so all of them stay within the pixel cap", () => {
    const pool = new CanvasPool({ max: 4, maxPixels: 1_000_000, create: fake });
    const a = pool.acquire();
    if (a === undefined) throw new Error("expected a canvas");
    const size = pool.fit(a, 2000, 1000);
    expect(size.width * size.height).toBeLessThanOrEqual(1_000_000);
    expect(size.width / size.height).toBeCloseTo(2, 1);
  });
});
