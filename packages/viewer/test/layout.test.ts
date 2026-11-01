// The page layout, zoom maths, tiles, bitmap cache and canvas pool (C3b): pure, in Node.
import { readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { BitmapCache } from "../src/canvas/cache.js";
import { CanvasPool } from "../src/canvas/pool.js";
import { CSS_PX_PER_PT, PageLayout } from "../src/layout/page-layout.js";
import { lowScale, tilesFor } from "../src/render/tiles.js";
import { percentOfScale, pointAt, scaleFor, scrollToPoint } from "../src/zoom/zoom.js";

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

  it("closes a replaced bitmap, and doesn't keep one larger than the budget", () => {
    const cache = new BitmapCache(100);
    const big = bitmap(100, 100);
    cache.set("k", 0, big, 0);
    expect(cache.size).toBe(0);
    expect((big as unknown as { closed: boolean }).closed).toBe(true);
  });
});

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

describe("dependencies", () => {
  it("imports no UI framework in src/", () => {
    const root = fileURLToPath(new URL("../src/", import.meta.url));
    const files: string[] = [];
    const walk = (dir: string) => {
      for (const name of readdirSync(dir)) {
        const path = join(dir, name);
        if (statSync(path).isDirectory()) walk(path);
        else if (path.endsWith(".ts")) files.push(path);
      }
    };
    walk(root);
    const framework = /from\s+["'](react|react-dom|preact|vue|svelte|solid-js|@angular\/[^"']+)["']/;
    expect(files.filter((f) => framework.test(readFileSync(f, "utf8")))).toEqual([]);
  });
});
