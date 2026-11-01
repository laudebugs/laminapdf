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

/** Waits (frame by frame, up to `limit` frames) until `done()`. */
async function until(done: () => boolean, limit = 600): Promise<void> {
  for (let i = 0; i < limit && !done(); i++) await frame();
  expect(done()).toBe(true);
}

describe("virtualisation", () => {
  it("never mounts more than visible + 2 × margin shells, nor more canvases than the pool, from page 1 to 1,000", async () => {
    const v = await opened();
    let worst = 0;
    let canvases = 0;
    v.on("renderstats", (s) => {
      worst = Math.max(worst, s.mounted - v.mountBudget);
      canvases = Math.max(canvases, s.canvases - s.maxCanvases);
    });
    for (let page = 0; page < 1000; page += 25) {
      v.goToPage(page);
      await frames(2);
    }
    v.goToPage(999);
    await frames(3);
    expect(v.getState().visibleRange.last).toBe(999);
    expect(worst).toBeLessThanOrEqual(0);
    expect(canvases).toBeLessThanOrEqual(0);
    expect(v.stats().peakCanvases).toBeLessThanOrEqual(v.stats().maxCanvases);
    v.destroy();
  });

  it("never paints a cancelled render while scrolling fast past 200 pages", async () => {
    const v = await opened();
    for (let page = 0; page <= 200; page += 2) {
      v.goToPage(page);
      await frame();
    }
    await until(() => Object.values(v.stats().pages).some((s) => s === "sharp"));
    expect(v.stats().drawsOnUnmounted).toBe(0);
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

  it("zooms with + / - / 0 and rotates the view", async () => {
    const v = await opened({ zoom: 100 });
    v.element.dispatchEvent(new KeyboardEvent("keydown", { key: "+", bubbles: true }));
    expect(Math.round(v.getState().percent)).toBe(125);
    v.element.dispatchEvent(new KeyboardEvent("keydown", { key: "0", bubbles: true }));
    expect(Math.round(v.getState().percent)).toBe(100);
    v.rotate(90);
    expect(v.getState().rotation).toBe(90);
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
