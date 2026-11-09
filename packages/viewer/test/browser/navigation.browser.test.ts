// Moving through a document (C6a) in Chromium, Firefox and WebKit: the thumbnails strip over a
// generated 1,000-page document (mounting only what it shows, rendering at background priority
// behind the main view, click and keyboard going to a page, page labels in its names), and the
// outline and named destinations of the links-and-outlines reference PDF landing on their pages.
import { Builder, init } from "@laminapdf/sdk";
import { createViewer, ThumbnailStrip, type Viewer, type ViewerClient } from "@laminapdf/viewer";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { hook, read, testViewer } from "./helpers.js";

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
    // Let the first thumbnails finish, then hold the worker while more queue.
    const strip = view.strip.element;
    await until(() => strip.querySelectorAll("canvas").length === strip.querySelectorAll("[role=option]").length);
    const before = ((await hook(rpc, "__renderLog", [])) as number[]).length;
    const busy = hook(rpc, "__busy", [200]);
    view.strip.element.scrollTop = 3000;
    await frame();
    await frame();
    const visible = rpc.render({ page: 0, scale: 0.777 }, { priority: "visible" });
    await busy;
    (await visible).bitmap.close();
    const queued = ((await hook(rpc, "__renderLog", [])) as number[]).slice(before);
    // Thumbnails queued first, but the visible render ran first.
    expect(queued.length).toBeGreaterThan(1);
    expect(queued[0]).toBe(0.777);
    close(view);
  });

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
