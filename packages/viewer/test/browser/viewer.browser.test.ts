// The viewer (C3b) in Chromium, Firefox and WebKit: virtualisation over a generated 1,000-page
// document, cancelled renders never painting, zoom anchoring under ctrl+wheel, and visible pages
// reaching sharp tiles once scrolling stops.
import { Builder, init } from "@laminapdf/sdk";
import { createViewer, type Viewer, type ViewerClient } from "@laminapdf/viewer";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { viewer as client } from "./helpers.js";

/** A seeded 1,000-page document of letter, A4 and landscape pages, each with one filled box. */
function thousandPages(): Uint8Array {
  const sizes = [
    [612, 792],
    [595, 842],
    [792, 612],
  ] as const;
  const b = new Builder({
    id: new TextEncoder().encode("lamina-c3b-1000p"),
    producer: "lamina tests",
    creationDate: "D:20261031120000Z",
  });
  for (let i = 0; i < 1000; i++) {
    const [w, h] = sizes[(i * 7) % 3] ?? sizes[0];
    const page = b.page([w, h]);
    page
      .surface()
      .fillColor([((i * 37) % 255) / 255, 0.4, 0.6])
      .path((p) => p.rect([40, 40, w - 80, 120]))
      .fill();
    page.finish();
  }
  const doc = b.finish();
  const bytes = doc.save();
  doc.free();
  return bytes;
}

const frame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
async function frames(n: number): Promise<void> {
  for (let i = 0; i < n; i++) await frame();
}

let rpc: ViewerClient;
let bytes: Uint8Array;
const host = document.createElement("div");
Object.assign(host.style, { width: "800px", height: "600px", position: "fixed", top: "0", left: "0" });
document.body.append(host);

beforeAll(async () => {
  await init();
  bytes = thousandPages();
  rpc = await client();
});
afterAll(() => rpc.terminate());

async function opened(options: Partial<Parameters<typeof createViewer>[0]> = {}): Promise<Viewer> {
  const doc = await rpc.open(bytes);
  const v = createViewer({ client: rpc, ...options });
  v.mount(host);
  v.open(doc);
  await frames(2);
  return v;
}

/** The shortest page's height in CSS px at the viewer's zoom (fit width: 768 px wide pages). */
function minPageHeight(v: Viewer): number {
  // The landscape pages (792 × 612 pt) are the shortest once every page is fitted to the width.
  const width = v.element.clientWidth - 32;
  return (612 / 792) * width;
}

/** Waits (frame by frame, up to `limit` frames) until `done()`. */
async function until(done: () => boolean, limit = 600): Promise<void> {
  for (let i = 0; i < limit && !done(); i++) await frame();
  expect(done()).toBe(true);
}

describe("virtualisation", () => {
  it("never mounts more than visible + 2 × margin shells, nor more canvases than the pool, scrolling page 1 to 1,000", async () => {
    const v = await opened();
    const root = v.element;
    // The margin is 2 pages or one viewport each side, whichever is larger; the shortest page here
    // is a landscape one, so one viewport spans at most this many pages.
    let worst = 0;
    let canvases = 0;
    v.on("renderstats", (s) => {
      const { first, last } = v.getState().visibleRange;
      const pagesPerViewport = Math.ceil(root.clientHeight / minPageHeight(v)) + 1;
      const margin = Math.max(2, pagesPerViewport);
      worst = Math.max(worst, s.mounted - (last - first + 1 + 2 * margin));
      canvases = Math.max(canvases, s.canvases - s.maxCanvases);
    });
    // One viewport per frame, the whole way down.
    while (root.scrollTop + root.clientHeight < root.scrollHeight - 1) {
      root.scrollTop += root.clientHeight;
      await frame();
    }
    await frames(2);
    expect(v.getState().visibleRange.last).toBe(999);
    expect(worst).toBeLessThanOrEqual(0);
    expect(canvases).toBeLessThanOrEqual(0);
    expect(v.stats().peakCanvases).toBeLessThanOrEqual(v.stats().maxCanvases);
    expect(v.stats().drawsOnUnmounted).toBe(0);
    v.destroy();
    // About 1,700 frames of scrolling, each rendering: longer than the suite's default on a busy machine.
  }, 180_000);

  it("never paints a cancelled render while scrolling fast past 200 pages", async () => {
    const v = await opened();
    for (let page = 0; page <= 200; page += 2) {
      v.goToPage(page);
      await frame();
    }
    await until(() => Object.values(v.stats().pages).some((s) => s === "sharp"));
    // Renders were cancelled as their pages left (the case the guarantee is about), and none drew.
    expect(v.stats().aborted).toBeGreaterThan(0);
    expect(v.stats().drawsOnUnmounted).toBe(0);
    v.destroy();
  });

  it("sharpens the part of a tall zoomed page scrolled into view later", async () => {
    const v = await opened({ zoom: 400 });
    v.goToPage(3);
    await until(() => v.stats().pages[3] === "sharp");
    // Starts from the state the page is in, so a stats event that repeats it before the scroll
    // lands (Firefox sends one) isn't counted as a change.
    const states: string[] = ["sharp"];
    v.on("renderstats", (s) => {
      const state = s.pages[3];
      if (state !== undefined && states.at(-1) !== state) states.push(state);
    });
    v.element.scrollTop += v.element.clientHeight * 1.5;
    await frames(2);
    expect(v.getState().visibleRange.first).toBe(3);
    await until(() => v.stats().pages[3] === "sharp");
    // The newly revealed part had to be drawn: the page went back to `low`, then sharp again.
    expect(states).toEqual(["sharp", "low", "sharp"]);
    v.destroy();
  });

  it("brings every visible page to sharp tiles once scrolling stops", async () => {
    const v = await opened();
    v.goToPage(500);
    await frames(2);
    await until(() => {
      const { first, last } = v.getState().visibleRange;
      const pages = v.stats().pages;
      for (let i = first; i <= last; i++) if (pages[i] !== "sharp") return false;
      return true;
    });
    v.destroy();
  });
});

describe("zoom", () => {
  it("keeps the point under the cursor within 1 CSS px through ctrl+wheel 100 % → 200 % → 50 %", async () => {
    const v = await opened({ zoom: 100 });
    v.goToPage(10);
    await frames(2);
    const rect = v.element.getBoundingClientRect();
    const cursor = { x: 333, y: 222 };
    const point = v.pointAt(cursor.x, cursor.y);
    if (point === undefined) throw new Error("no point");
    for (const factor of [2, 0.25]) {
      const deltaY = -Math.log(factor) / 0.01;
      v.element.dispatchEvent(
        new WheelEvent("wheel", {
          ctrlKey: true,
          deltaY,
          clientX: rect.left + cursor.x,
          clientY: rect.top + cursor.y,
          bubbles: true,
          cancelable: true,
        }),
      );
      await frames(2);
      const at = v.positionOf(point);
      expect(Math.abs((at?.y ?? 0) - cursor.y)).toBeLessThanOrEqual(1);
      // Across, the anchor holds while the pages are wider than the viewport; narrower ones are
      // centred, so there is no horizontal scroll to keep it with.
      if (v.element.scrollWidth > v.element.clientWidth)
        expect(Math.abs((at?.x ?? 0) - cursor.x)).toBeLessThanOrEqual(1);
    }
    expect(Math.round(v.getState().percent)).toBe(50);
    v.destroy();
  });

  it("reports no page sharp while a zoom's canvases are stretched, then sharpens them", async () => {
    const v = await opened({ zoom: "actual-size" });
    await until(() => v.stats().pages[0] === "sharp");
    const reported: string[] = [];
    v.on("renderstats", (stats) => reported.push(stats.pages[0] ?? ""));
    v.setZoom(200);
    // The canvas still holds the old zoom's pixels, stretched: not sharp until the new pass
    // paints, and `renderstats` says so at once.
    expect(v.stats().pages[0]).not.toBe("sharp");
    expect(reported[0]).toBe("low");
    await until(() => v.stats().pages[0] === "sharp");
    v.destroy();
  });

  // A trackpad pinch pauses longer than the settle delay between steps, so each pause repaints the
  // page for its new zoom. That repaint used to fill the canvas with the low-resolution pass, so the
  // page flickered from sharp to blurry at every pause. It now keeps the sharp pixels it had, scaled.
  it("keeps the page's sharp pixels, scaled, when it repaints for a new zoom", async () => {
    const v = await opened({ zoom: 100, dpr: () => 1 });
    await until(() => v.stats().pages[0] === "sharp");
    const canvas = () => v.element.querySelector<HTMLCanvasElement>('.lamina-page[data-page-index="0"] canvas');
    // The sharpest edge along a row through page 0's filled box (40 pt from the left, 40–160 pt down).
    const edge = () => {
      const c = canvas();
      if (c === null) return 0;
      const s = c.width / 612;
      const y = Math.round(100 * s);
      const d = c.getContext("2d")?.getImageData(0, y, Math.round(80 * s), 1).data ?? new Uint8ClampedArray();
      let max = 0;
      for (let i = 4; i < d.length; i += 4) {
        const g =
          Math.abs((d[i] ?? 0) - (d[i - 4] ?? 0)) +
          Math.abs((d[i + 1] ?? 0) - (d[i - 3] ?? 0)) +
          Math.abs((d[i + 2] ?? 0) - (d[i - 2] ?? 0));
        max = Math.max(max, g);
      }
      return max;
    };
    const sharp = edge();
    // The repaint for the new zoom draws, then reports the page `low` at once: read it right then.
    let repainted = -1;
    v.on("renderstats", (stats) => {
      // The repainted canvas: 1224 px wide at 150 % (816 before it).
      if (repainted < 0 && stats.pages[0] === "low" && (canvas()?.width ?? 0) > 1000) repainted = edge();
    });
    v.setZoom(150);
    await until(() => repainted >= 0);
    // The low pass smears the edge over several pixels; the old sharp pixels, scaled 1.5×, keep it hard.
    expect(repainted).toBeGreaterThan(sharp * 0.5);
    v.destroy();
  });

  it("repaints in the new orientation when the view rotates at the same zoom", async () => {
    const v = await opened({ zoom: "actual-size" });
    await until(() => v.stats().pages[0] === "sharp");
    const canvas = () => v.element.querySelector<HTMLCanvasElement>('.lamina-page[data-page-index="0"] canvas');
    const portrait = canvas()?.width ?? 0;
    v.rotate(90);
    await until(() => v.stats().pages[0] === "sharp" && (canvas()?.width ?? 0) !== portrait);
    // Page 0 is letter portrait: turned, the canvas is as wide as the page was tall.
    const turned = canvas();
    expect((turned?.width ?? 0) > (turned?.height ?? 0)).toBe(true);
    v.destroy();
  });

  it("doesn't spin through frames for pages the pool can't give a canvas", async () => {
    const v = await opened({ zoom: 10, pool: { max: 2 } });
    await frames(30);
    let stats = 0;
    v.on("renderstats", () => stats++);
    await frames(30);
    // With nothing changing, frames stop: no endless pass-and-reschedule loop.
    expect(stats).toBeLessThan(5);
    v.destroy();
  });

  it("zooms with + / - / 0 and rotates the view", async () => {
    const v = await opened({ zoom: 100 });
    v.element.dispatchEvent(new KeyboardEvent("keydown", { key: "+", bubbles: true }));
    expect(Math.round(v.getState().percent)).toBe(125);
    v.element.dispatchEvent(new KeyboardEvent("keydown", { key: "0", bubbles: true }));
    expect(Math.round(v.getState().percent)).toBe(100);
    v.rotate(90);
    expect(v.getState().rotation).toBe(90);
    expect(() => v.rotate(45)).toThrow(RangeError);
    v.destroy();
  });
});

describe("detail canvas", () => {
  const detail = (v: Viewer, page: number) =>
    v.element.querySelector<HTMLCanvasElement>(`.lamina-page[data-page-index="${page}"] .lamina-page-detail`);
  /** Device px per CSS px of page `page`'s detail canvas, as laid out. */
  const detailRatio = (v: Viewer, page: number) => {
    const canvas = detail(v, page);
    return canvas === null ? 0 : canvas.width / canvas.getBoundingClientRect().width;
  };

  it("draws the page in view at the full pixel ratio at 400 % and 800 %", async () => {
    for (const zoom of [400, 800]) {
      const v = await opened({ zoom, dpr: () => 2 });
      // The widest pages (landscape) set the content's width: centre the view on page 0.
      v.element.scrollLeft = (v.element.scrollWidth - v.element.clientWidth) / 2;
      await frames(2);
      await until(() => v.stats().pages[0] === "sharp");
      expect(v.stats().detailCanvases).toBeGreaterThan(0);
      expect(detailRatio(v, 0)).toBeCloseTo(2, 2);
      v.destroy();
    }
  });

  it("needs no detail canvas while the page fits its canvas at the full ratio", async () => {
    const v = await opened({ zoom: 100, dpr: () => 1 });
    await until(() => v.stats().pages[0] === "sharp");
    expect(v.stats().detailCanvases).toBe(0);
    expect(detail(v, 0)).toBeNull();
    v.destroy();
  });

  it("hands detail canvases on to the pages scrolled into view, past more pages than the pool holds", async () => {
    const v = await opened({ zoom: 200, dpr: () => 2, detailPool: { max: 1 } });
    for (let page = 0; page < 6; page++) {
      v.goToPage(page);
      await frames(2);
      v.element.scrollLeft = (v.element.scrollWidth - v.element.clientWidth) / 2;
      await until(() => v.stats().pages[page] === "sharp");
      expect(detailRatio(v, page)).toBeCloseTo(2, 2);
      expect(v.stats().detailCanvases).toBe(1);
    }
    v.destroy();
  });

  it("doesn't spin through frames for a page the detail pool can't serve", async () => {
    const v = await opened({ zoom: 400, dpr: () => 2, detailPool: { max: 0 } });
    v.element.scrollLeft = (v.element.scrollWidth - v.element.clientWidth) / 2;
    await frames(30);
    let stats = 0;
    v.on("renderstats", () => stats++);
    await frames(30);
    // The page shows its backdrop and waits for a detail canvas, rather than retry every frame.
    expect(stats).toBeLessThan(5);
    expect(v.stats().pages[0]).toBe("low");
    v.destroy();
  });

  it("moves with the view at 400 %: sharp again after scrolling down and across", async () => {
    const v = await opened({ zoom: 400, dpr: () => 2 });
    await until(() => v.stats().pages[0] === "sharp");
    v.element.scrollTop += v.element.clientHeight * 2;
    v.element.scrollLeft += v.element.clientWidth;
    await frames(2);
    const page = v.getState().visibleRange.first;
    await until(() => v.stats().pages[page] === "sharp");
    expect(detailRatio(v, page)).toBeCloseTo(2, 2);
    expect(v.stats().drawsOnUnmounted).toBe(0);
    v.destroy();
  });
});

describe("layers", () => {
  it("mounts registered layers on each mounted page and unmounts them with it", async () => {
    const v = await opened();
    const mounted = new Set<number>();
    v.addLayer({
      name: "text",
      onMount: (page, el) => {
        mounted.add(page);
        expect(el.className).toBe("lamina-layer-text");
      },
      onUnmount: (page) => mounted.delete(page),
    });
    expect(mounted.size).toBe(v.stats().mounted);
    v.goToPage(600);
    await frames(3);
    expect([...mounted].every((p) => p >= 590)).toBe(true);
    v.destroy();
  });
});
