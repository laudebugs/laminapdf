// Moving through a document (C6a) in Chromium, Firefox and WebKit: the thumbnails strip over a
// generated 1,000-page document (mounting only what it shows, rendering at background priority
// behind the main view, click and keyboard going to a page, page labels in its names), and the
// outline and named destinations of the links-and-outlines reference PDF landing on their pages.
import { Builder, init } from "@laminapdf/sdk";
import { createViewer, ThumbnailStrip, type Viewer, type ViewerClient } from "@laminapdf/viewer";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { hook, next, read, testViewer } from "./helpers.js";

/** A 1,000-page document of letter pages, each with one box. */
function thousandPages(): Uint8Array {
  const b = new Builder({ id: new TextEncoder().encode("lamina-c6a-1000p"), producer: "lamina tests" });
  for (let i = 0; i < 1000; i++) {
    const page = b.page([612, 792]);
    page
      .surface()
      .fillColor([0.2, 0.4, 0.6])
      .path((p) => p.rect([40, 40, 200, 120]))
      .fill();
    page.finish();
  }
  const doc = b.finish();
  const bytes = doc.save();
  doc.free();
  return bytes;
}

const frame = () => new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));

/** Waits (frame by frame, up to `limit` frames) until `done()`. */
async function until(done: () => boolean, limit = 600): Promise<void> {
  for (let i = 0; i < limit && !done(); i++) await frame();
  expect(done()).toBe(true);
}

function box(width: number, height: number, left = 0): HTMLElement {
  const el = document.createElement("div");
  Object.assign(el.style, {
    width: `${width}px`,
    height: `${height}px`,
    position: "fixed",
    top: "0",
    left: `${left}px`,
  });
  document.body.append(el);
  return el;
}

let rpc: ViewerClient;
let thousand: Uint8Array;
const side = box(160, 600);
const main = box(800, 600, 170);
side.style.display = "flex";

beforeAll(async () => {
  await init();
  thousand = thousandPages();
  rpc = await testViewer();
});
afterAll(() => rpc.terminate());

async function opened(bytes: Uint8Array): Promise<{ viewer: Viewer; strip: ThumbnailStrip }> {
  const doc = await rpc.open(bytes);
  const viewer = createViewer({ client: rpc });
  viewer.mount(main);
  viewer.open(doc);
  const strip = new ThumbnailStrip(rpc, viewer);
  strip.element.style.flex = "1";
  strip.mount(side);
  strip.open(doc);
  await frame();
  await frame();
  return { viewer, strip };
}

function close({ viewer, strip }: { viewer: Viewer; strip: ThumbnailStrip }): void {
  strip.destroy();
  viewer.destroy();
}

describe("the thumbnails strip", () => {
  it("never mounts more than the thumbnails in view and 4 either side, scrolling 1,000 pages", async () => {
    const view = await opened(thousand);
    const { strip } = view;
    // A letter page 120 px wide is 155 px tall, plus its caption and gap: about 4 in 600 px.
    const bound = Math.ceil(600 / (155 + 32)) + 1 + 2 * 4;
    let peak = 0;
    for (let top = 0; top < strip.element.scrollHeight; top += 1500) {
      strip.element.scrollTop = top;
      await frame();
      await frame();
      peak = Math.max(peak, strip.element.querySelectorAll("[role=option]").length);
    }
    expect(peak).toBeLessThanOrEqual(bound);
    close(view);
  });

  it("renders behind the main view's visible pages", async () => {
    const view = await opened(thousand);
    try {
      await rendersBehind(view);
    } finally {
      close(view);
    }
  });

  /** Holds the worker while thumbnails queue, sends a visible render, and checks it ran first. */
  async function rendersBehind(view: { viewer: Viewer; strip: ThumbnailStrip }): Promise<void> {
    const strip = view.strip.element;
    const painted = () => strip.querySelectorAll("canvas").length === strip.querySelectorAll("[role=option]").length;
    // The main view renders at visible priority too. Once the worker has started no render for
    // 300 ms, the main view is done, so only the strip's thumbnails and the test's render queue
    // while the worker is held.
    const quiet = async () => {
      let last = -1;
      for (let i = 0; i < 50; i++) {
        const started = ((await hook(rpc, "__renderLog", [])) as number[]).length;
        if (started === last) return;
        last = started;
        await new Promise((resolve) => setTimeout(resolve, 300));
      }
    };
    // The worker is held while thumbnails queue and the visible render is sent. A slow machine can
    // take longer to queue them than the hold lasts; then the order proves nothing, so the hold
    // doubles and it runs again.
    for (const [attempt, hold] of [1500, 3000, 6000].entries()) {
      await until(painted);
      await quiet();
      // A thumbnail's scale: its canvas's pixel width over the page's width in points (US Letter).
      const thumbnail = (strip.querySelector("canvas")?.width ?? 0) / 612;
      const before = ((await hook(rpc, "__renderLog", [])) as number[]).length;
      const released = hook(rpc, "__busy", [hold]) as Promise<number>;
      strip.scrollTop = 3000 * (attempt + 1);
      // Thumbnails mounted without a canvas yet have their renders queued. Once the hold is over
      // they may paint before this sees them, which the check below catches: so it never fails.
      for (
        let i = 0;
        i < 600 && !(strip.querySelectorAll("[role=option]").length > strip.querySelectorAll("canvas").length);
        i++
      ) {
        await frame();
      }
      const visible = rpc.render({ page: 0, scale: 0.777 }, { priority: "visible" });
      const sent = Date.now();
      const heldUntil = await released;
      (await visible).bitmap.close();
      // The log is read once the queued thumbnails have painted: a document call such as the
      // log's own outranks a background render, so reading it sooner would come before them.
      await until(painted);
      const queued = ((await hook(rpc, "__renderLog", [])) as number[]).slice(before);
      // Sent with a margin before the hold ended: it was queued behind the hold.
      if (sent < heldUntil - 100) {
        // Thumbnails queued first, but the visible render ran before any of them.
        const isThumbnail = (scale: number) => Math.abs(scale - thumbnail) < 0.01;
        const visibleAt = queued.indexOf(0.777);
        expect(queued.some(isThumbnail)).toBe(true);
        expect(visibleAt).toBeGreaterThanOrEqual(0);
        expect(queued.slice(0, visibleAt).filter(isThumbnail)).toEqual([]);
        return;
      }
    }
    throw new Error("the hold ended before the visible render was sent, every time");
  }

  it("goes to a page on a click", async () => {
    const view = await opened(thousand);
    const third = view.strip.element.querySelector<HTMLElement>('[data-page-index="2"]');
    third?.click();
    await until(() => view.viewer.getState().currentPage === 2);
    await until(() => third?.getAttribute("aria-selected") === "true");
    close(view);
  });

  it("moves focus with the arrow keys and goes to the page on Enter", async () => {
    const view = await opened(thousand);
    const first = view.strip.element.querySelector<HTMLElement>('[tabindex="0"]');
    first?.focus();
    const key = (k: string) =>
      document.activeElement?.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true }));
    key("ArrowDown");
    key("ArrowDown");
    expect((document.activeElement as HTMLElement).dataset.pageIndex).toBe("2");
    key("Enter");
    await until(() => view.viewer.getState().currentPage === 2);
    key("End");
    expect((document.activeElement as HTMLElement).dataset.pageIndex).toBe("999");
    close(view);
  });

  it("names each thumbnail by its page label", async () => {
    const view = await opened(await read("fixtures/golden/links-and-outlines.pdf"));
    const names = [...view.strip.element.querySelectorAll("[role=option]")].map((el) => el.getAttribute("aria-label"));
    expect(names).toEqual(["Page i (1 of 4)", "Page ii (2 of 4)", "Page 1 (3 of 4)", "Page 2 (4 of 4)"]);
    close(view);
  });
});

describe("destinations", () => {
  it("reads the outline in the worker", async () => {
    await rpc.open(await read("fixtures/golden/links-and-outlines.pdf"));
    const outline = await rpc.outline();
    expect(outline.map((n) => [n.title, n.target?.page])).toEqual([
      ["Title page", 0],
      ["Contents", 1],
      ["Chapters", undefined],
      ["Website", undefined],
    ]);
  });

  it("lands a named destination on its page", async () => {
    const view = await opened(await read("fixtures/golden/links-and-outlines.pdf"));
    const target = await rpc.namedDestination("chapter-2");
    expect(target?.page).toBe(3);
    if (target !== undefined) view.viewer.goToDestination(target);
    await until(() => view.viewer.getState().visibleRange.last === 3);
    close(view);
  });
});

describe("restoring a position", () => {
  it("opens at a kept position without painting another page first, and Back goes to the start", async () => {
    const doc = await rpc.open(thousand);
    expect(doc.fingerprint).toMatch(/^fp1:[0-9a-f]{64}$/);
    const viewer = createViewer({ client: rpc });
    viewer.mount(main);
    const started: number[] = [];
    const stop = rpc.on("render-start", ({ page }) => started.push(page));
    try {
      // Restored in the task that opens the document, as the app does.
      viewer.open(doc);
      viewer.restore({ point: { page: 500, x: 0, y: 0.25 }, zoom: 150, rotation: 90 });
      await until(() => started.length > 0 && viewer.getState().visibleRange.first >= 499);
      expect(started.filter((page) => page < 490)).toEqual([]);
      const state = viewer.getState();
      expect([state.zoom, state.rotation]).toEqual([150, 90]);
      // The kept point sits a gap (16 px) below the viewport's top, as a history jump puts it;
      // a pixel more for the scroll's rounding.
      const top = viewer.positionOf({ page: 500, x: 0, y: 0.25 })?.y ?? Number.NaN;
      expect(top).toBeGreaterThanOrEqual(0);
      expect(top).toBeLessThanOrEqual(17);
      // A position kept now comes back the same.
      const kept = viewer.viewPosition();
      expect(kept?.point.page).toBe(500);
      expect(kept?.point.y).toBeCloseTo(0.25, 2);
      expect(viewer.history.getState().back?.point.page).toBe(0);
      expect(viewer.history.back()).toBe(true);
      await until(() => viewer.getState().visibleRange.first === 0);
    } finally {
      stop();
      viewer.destroy();
    }
  });

  it("clamps a position past the document's end to its last page", async () => {
    const doc = await rpc.open(await read("fixtures/golden/links-and-outlines.pdf"));
    const viewer = createViewer({ client: rpc });
    viewer.mount(main);
    try {
      viewer.open(doc);
      viewer.restore({ point: { page: 99, x: 2, y: -1 }, zoom: "fit-width", rotation: 0 });
      // The last page is shown; it can't scroll to the viewport's top, so the view stops at the end.
      await until(() => viewer.getState().visibleRange.last === doc.pageCount - 1);
      expect(viewer.getState().scrollTop).toBeGreaterThan(0);
    } finally {
      viewer.destroy();
    }
  });
});

describe("a view opened while hidden", () => {
  it("starts a new document at its top, not where the last one was scrolled", async () => {
    const host = box(800, 600, 170);
    const viewer = createViewer({ client: rpc });
    viewer.mount(host);
    try {
      viewer.open(await rpc.open(thousand));
      viewer.goToPage(300);
      await until(() => viewer.getState().visibleRange.first >= 299);
      host.style.display = "none";
      viewer.open(await rpc.open(await read("fixtures/golden/links-and-outlines.pdf")));
      host.style.display = "";
      await until(() => viewer.getState().scrollTop === 0 && viewer.getState().visibleRange.first === 0);
    } finally {
      viewer.destroy();
      host.remove();
    }
  });

  it("restores a kept position once it shows, painting no other page first", async () => {
    const host = box(800, 600, 170);
    host.style.display = "none";
    const viewer = createViewer({ client: rpc });
    viewer.mount(host);
    const started: number[] = [];
    const stop = rpc.on("render-start", ({ page }) => started.push(page));
    try {
      viewer.open(await rpc.open(thousand));
      viewer.restore({ point: { page: 700, x: 0, y: 0 }, zoom: 100, rotation: 0 });
      // While hidden it already reports where it will be.
      expect(viewer.viewPosition()?.point.page).toBe(700);
      await frame();
      host.style.display = "";
      await until(() => started.length > 0 && viewer.getState().visibleRange.first >= 699);
      expect(started.filter((page) => page < 690)).toEqual([]);
      expect(viewer.history.getState().back?.point.page).toBe(0);
    } finally {
      stop();
      viewer.destroy();
      host.remove();
    }
  });
});

describe("a restore while hidden after another document", () => {
  it("leaves Back at page 1, not where the last document was scrolled", async () => {
    const host = box(800, 600, 170);
    const viewer = createViewer({ client: rpc });
    viewer.mount(host);
    try {
      viewer.open(await rpc.open(thousand));
      viewer.goToPage(300);
      await until(() => viewer.getState().visibleRange.first >= 299);
      host.style.display = "none";
      viewer.open(await rpc.open(thousand));
      viewer.restore({ point: { page: 700, x: 0, y: 0 }, zoom: 100, rotation: 0 });
      host.style.display = "";
      await until(() => viewer.getState().visibleRange.first >= 699);
      expect(viewer.history.getState().back?.point.page).toBe(0);
    } finally {
      viewer.destroy();
      host.remove();
    }
  });
});

describe("the thumbnails strip after a crash", () => {
  it("draws the thumbnails a crash interrupted once the worker restarts, and reports no error", async () => {
    // Its own client: the crash restarts the worker the other tests share.
    const client = await testViewer();
    const errors: unknown[] = [];
    const reported = (e: ErrorEvent) => {
      errors.push(e.error ?? e.message);
      e.preventDefault();
    };
    window.addEventListener("error", reported);
    const doc = await client.open(thousand);
    const viewer = createViewer({ client });
    viewer.mount(main);
    viewer.open(doc);
    const strip = new ThumbnailStrip(client, viewer);
    strip.element.style.flex = "1";
    strip.mount(side);
    strip.open(doc);
    const list = strip.element;
    const painted = () => list.querySelectorAll("canvas").length === list.querySelectorAll("[role=option]").length;
    try {
      await until(painted);
      // The worker is held while the thumbnails scrolled to queue their renders; the crash runs
      // first when the hold ends (document calls go before background renders) and rejects them.
      const held = hook(client, "__busy", [1500]).catch(() => undefined);
      list.scrollTop = 6000;
      await until(() => !painted());
      // Thumbnails still waiting for their render when the worker crashed: the ones it interrupted.
      let waiting = 0;
      const off = client.on("crashed", () => {
        waiting = list.querySelectorAll("[role=option]").length - list.querySelectorAll("canvas").length;
      });
      const restarted = next(client, "restarted");
      void hook(client, "__crash").catch(() => undefined);
      await held;
      expect(await restarted).toMatchObject({ reopened: true });
      off();
      expect(waiting).toBeGreaterThan(0);
      await until(painted);
      expect(errors).toEqual([]);
    } finally {
      window.removeEventListener("error", reported);
      strip.destroy();
      viewer.destroy();
      client.terminate();
    }
  });

  it("reports no error for the thumbnail that was drawing when the worker stopped", async () => {
    // Its own client: the stop restarts the worker the other tests share.
    const client = await testViewer();
    const errors: unknown[] = [];
    const reported = (e: ErrorEvent) => {
      errors.push(e.error ?? e.message);
      e.preventDefault();
    };
    window.addEventListener("error", reported);
    const doc = await client.open(await read("fixtures/corpus/hello.pdf"));
    const viewer = createViewer({ client });
    viewer.mount(main);
    viewer.open(doc);
    const strip = new ThumbnailStrip(client, viewer);
    try {
      await until(() => viewer.stats().pages[0] === "sharp");
      // The main view is drawn, so the next render is the thumbnail's: it holds the worker once
      // drawn, still the render running, and is the one a restart doesn't send again.
      await hook(client, "__holdNextRender", [3000]);
      const started = next(client, "render-start");
      strip.mount(side);
      strip.open(doc);
      await started;
      const restarted = next(client, "restarted");
      void client.stop();
      expect(await restarted).toMatchObject({ reason: "cancelled", reopened: true });
      expect(strip.element.querySelectorAll("canvas").length).toBe(0);
      expect(errors).toEqual([]);
    } finally {
      window.removeEventListener("error", reported);
      strip.destroy();
      viewer.destroy();
      client.terminate();
    }
  });
});
