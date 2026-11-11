// Edits on screen in Chromium, Firefox and WebKit: an edit through the document worker reopens
// the render document from the edited file, so a turned page draws turned and at its new size;
// the viewer and its thumbnails redraw that page alone, and the other pages keep what they drew.
// A metadata edit reopens and redraws nothing. A failed edit, a failed reopen or a failed undo
// leaves the pages drawing as before, and a restarted worker draws the file as it was. Files read
// range by range (from a link, or from their OPFS copy) take edits too.

import { Builder, Font, init, type Op } from "@laminapdf/sdk";
import { createViewer, ThumbnailStrip, type Viewer, type ViewerClient } from "@laminapdf/viewer";
import { afterEach, beforeAll, describe, expect, it } from "vitest";
import { addressOf, hook, next, read, rejection, testViewer } from "./helpers.js";

const LETTER = [612, 792] as const;
/** The red box on every page, in page points from the top-left: x, y, width, height. */
const BOX = [40, 40, 160, 80] as const;

/**
 * Three letter pages, each with a red box at its top-left and its number below it; the first lies
 * on its side, so a page turned later is never wider than the widest page already there.
 */
function threePages(): Uint8Array {
  const b = new Builder({ id: new TextEncoder().encode("lamina-e1-redraw"), producer: "lamina tests" });
  const font = Font.standard("Helvetica");
  for (let i = 0; i < 3; i++) {
    const page = b.page(i === 0 ? [LETTER[1], LETTER[0]] : [...LETTER]);
    const s = page.surface();
    s.fillColor([1, 0, 0])
      .path((p) => p.rect([...BOX]))
      .fill();
    s.fillColor([0, 0, 0]).text(`Page ${i + 1}`, { font, size: 24, at: [60, 200] });
    page.finish();
  }
  const doc = b.finish();
  const bytes = doc.save();
  doc.free();
  return bytes;
}

/** Turns page `id` a quarter clockwise. */
const turn = (id: string): Op[] => [{ op: "rotatePage", args: { page: id, rotate: 90 } } as Op];

/** The RGBA pixel of `source` at (`fx`, `fy`), fractions of its width and height. */
function pixel(source: CanvasImageSource & { width: number; height: number }, fx: number, fy: number): number[] {
  const canvas = document.createElement("canvas");
  canvas.width = source.width;
  canvas.height = source.height;
  const ctx = canvas.getContext("2d");
  if (ctx === null) throw new Error("no 2d context");
  ctx.drawImage(source, 0, 0);
  const x = Math.floor(fx * source.width);
  const y = Math.floor(fy * source.height);
  return [...ctx.getImageData(x, y, 1, 1).data];
}

const isRed = ([r, g, b]: number[]) => (r ?? 0) > 200 && (g ?? 0) < 80 && (b ?? 0) < 80;
const isWhite = ([r, g, b]: number[]) => (r ?? 0) > 240 && (g ?? 0) > 240 && (b ?? 0) > 240;

/**
 * Where the box's centre is, as fractions of the displayed page: at the top-left unturned; a
 * quarter turn clockwise takes a point (x, y) of a page h points tall to (h − y, x).
 */
const [bx, by, bw, bh] = BOX;
const UPRIGHT = [(bx + bw / 2) / LETTER[0], (by + bh / 2) / LETTER[1]] as const;
const TURNED = [(LETTER[1] - (by + bh / 2)) / LETTER[1], (bx + bw / 2) / LETTER[0]] as const;

let bytes: Uint8Array;
let rpc: ViewerClient | undefined;
let view: Viewer | undefined;
let host: HTMLElement | undefined;
let strip: ThumbnailStrip | undefined;
let side: HTMLElement | undefined;

beforeAll(async () => {
  await init({ text: true });
  bytes = threePages();
});

afterEach(() => {
  strip?.destroy();
  side?.remove();
  view?.destroy();
  host?.remove();
  rpc?.terminate();
  [rpc, view, host, strip, side] = [undefined, undefined, undefined, undefined, undefined];
});

async function opened(): Promise<ViewerClient> {
  rpc = await testViewer("pdfium");
  await rpc.open(bytes);
  return rpc;
}

/**
 * A viewer 800 × 400 px with page 1 at the top, once that page is sharp, and a thumbnail strip
 * beside it once its three thumbnails are drawn.
 */
async function showing(client: ViewerClient): Promise<Viewer> {
  host = document.createElement("div");
  Object.assign(host.style, { width: "800px", height: "400px", position: "fixed", top: "0", left: "0" });
  side = document.createElement("div");
  Object.assign(side.style, { width: "160px", height: "600px", position: "fixed", top: "0", left: "820px" });
  document.body.append(host, side);
  const opened = client.document as NonNullable<ViewerClient["document"]>;
  view = createViewer({ client });
  view.mount(host);
  view.open(opened);
  strip = new ThumbnailStrip(client, view);
  strip.mount(side);
  strip.open(opened);
  await expect.poll(() => view?.getState().pageCount).toBe(3);
  view.goToPage(1);
  await sharp(view, 1);
  await expect.poll(() => thumbnails().filter((t) => t !== null).length, { timeout: 20_000 }).toBe(3);
  return view;
}

/** Each page's thumbnail canvas, or null while it has none. */
function thumbnails(): (HTMLCanvasElement | null)[] {
  return [0, 1, 2].map((i) => document.querySelector(`.lamina-thumbnail[data-page-index="${i}"] canvas`));
}

/** The `invalidate` events the viewer fires from now on. */
function invalidated(v: Viewer): number[][] {
  const events: number[][] = [];
  v.on("invalidate", ({ pages }) => events.push(pages.map((p) => p.index)));
  return events;
}

async function sharp(v: Viewer, page: number): Promise<void> {
  await expect.poll(() => v.stats().pages[page], { timeout: 20_000 }).toBe("sharp");
}

/** The pages the worker starts drawing from now on. */
function rendered(client: ViewerClient): number[] {
  const pages: number[] = [];
  client.on("render-start", ({ page }) => pages.push(page));
  return pages;
}

function canvasOf(page: number): HTMLCanvasElement {
  const canvas = document.querySelector<HTMLCanvasElement>(`.lamina-page[data-page-index="${page}"] canvas`);
  if (canvas === null) throw new Error(`page ${page} has no canvas`);
  return canvas;
}

describe("redraw after an edit", () => {
  it("draws a page turned through the worker at its new size, from the edited file", async () => {
    const client = await opened();
    const before = await client.render({ page: 1, scale: 1 });
    expect([before.width, before.height]).toEqual([612, 792]);
    expect(isRed(pixel(before.bitmap, ...UPRIGHT))).toBe(true);
    before.bitmap.close();
    const first = await hook(client, "__renderDocument");
    const id = client.document?.pages[1]?.id as string;

    const result = await client.applyEdit(turn(id));

    expect(result.pages).toHaveLength(1);
    expect(result.pages[0]).toMatchObject({ index: 1, id, width: 792, height: 612, rotation: 90 });
    expect(client.document?.pages[1]).toMatchObject({ width: 792, height: 612 });
    expect(await hook(client, "__renderDocument")).not.toBe(first);
    const after = await client.render({ page: 1, scale: 1 });
    expect([after.width, after.height]).toEqual([792, 612]);
    expect(isRed(pixel(after.bitmap, ...TURNED))).toBe(true);
    expect(isWhite(pixel(after.bitmap, ...UPRIGHT))).toBe(true);
    after.bitmap.close();
    const other = await client.render({ page: 0, scale: 1 });
    expect([other.width, other.height]).toEqual([792, 612]);
    other.bitmap.close();
    // The text comes from the edited model too: the number, once at the left, is now at the right.
    const text = await client.pageText(1);
    expect(text.quads[0]).toBeGreaterThan(792 / 2);
  });

  it("redraws only the turned page in the viewer, at its new size, and keeps the others' bitmaps", async () => {
    const client = await opened();
    const v = await showing(client);
    const pages = rendered(client);
    const page0 = canvasOf(0);

    await v.applyEdit(turn(client.document?.pages[1]?.id as string));
    await sharp(v, 1);

    expect(new Set(pages)).toEqual(new Set([1]));
    expect(canvasOf(0)).toBe(page0);
    // Page 2 stays mounted below it. Nothing drew pages 0 and 2 again, in the view or the
    // thumbnails: each kept its bitmaps.
    expect(document.querySelector('.lamina-page[data-page-index="2"]')).not.toBeNull();
    // Page 1's thumbnail is drawn again, turned.
    await expect
      .poll(() => (thumbnails()[1]?.width ?? 0) > (thumbnails()[1]?.height ?? 0), { timeout: 20_000 })
      .toBe(true);
    expect(isRed(pixel(thumbnails()[1] as HTMLCanvasElement, ...TURNED))).toBe(true);
    const shell = document.querySelector<HTMLElement>('.lamina-page[data-page-index="1"]') as HTMLElement;
    expect(shell.offsetWidth / shell.offsetHeight).toBeCloseTo(792 / 612, 2);
    // Page 1 stays at the top of the view.
    expect(v.getState().currentPage).toBe(1);
    const canvas = canvasOf(1);
    expect(isRed(pixel(canvas, ...TURNED))).toBe(true);
    expect(isWhite(pixel(canvas, ...UPRIGHT))).toBe(true);
    // Its text layer read the page again: a span over the number, now at the page's right.
    await expect
      .poll(() => document.querySelectorAll('.lamina-page[data-page-index="1"] .lamina-text-layer span').length)
      .toBeGreaterThan(0);
  });

  it("reopens nothing and redraws nothing for an edit of the metadata", async () => {
    const client = await opened();
    const v = await showing(client);
    const first = await hook(client, "__renderDocument");
    const pages = rendered(client);
    const events = invalidated(v);

    const result = await v.applyEdit([{ op: "setInfo", args: { patch: { title: "Edited" } } } as Op]);

    expect(result.pages).toEqual([]);
    expect(events).toEqual([]);
    expect(await hook(client, "__renderDocument")).toBe(first);
    expect((await client.properties()).title).toBe("Edited");
    await new Promise((r) => setTimeout(r, 200));
    expect(pages).toEqual([]);
    expect(v.stats().pages[1]).toBe("sharp");
  });

  it("changes nothing when an edit fails, and the pages go on drawing", async () => {
    const client = await opened();
    const v = await showing(client);
    const first = await hook(client, "__renderDocument");

    const error = await rejection(v.applyEdit(turn("p999.0")));

    expect(error).toMatchObject({ code: expect.any(String) });
    expect(await hook(client, "__renderDocument")).toBe(first);
    expect(client.document?.pages[1]).toMatchObject({ width: 612, height: 792 });
    const page = await client.render({ page: 1, scale: 1 });
    expect([page.width, page.height]).toEqual([612, 792]);
    expect(isRed(pixel(page.bitmap, ...UPRIGHT))).toBe(true);
    page.bitmap.close();
    expect(v.stats().pages[1]).toBe("sharp");
  });

  it("keeps drawing the page as it was when the reopen fails, and the next edit still shows", async () => {
    const client = await opened();
    const v = await showing(client);
    const first = await hook(client, "__renderDocument");
    const id = client.document?.pages[1]?.id as string;
    await hook(client, "__failNextReopen");

    const error = await rejection(v.applyEdit(turn(id)));

    expect(String((error as Error).message)).toContain("reopen fail");
    expect(await hook(client, "__renderDocument")).toBe(first);
    expect(client.document?.pages[1]).toMatchObject({ width: 612, height: 792, rotation: 0 });
    expect(v.stats().pages[1]).toBe("sharp");
    // The model was turned back, so turning it again is a new edit, which shows.
    const result = await v.applyEdit(turn(id));
    expect(result.pages[0]).toMatchObject({ width: 792, height: 612 });
    await sharp(v, 1);
    expect(isRed(pixel(canvasOf(1), ...TURNED))).toBe(true);
  });

  it("says the document must be opened again when the edit can't be shown or undone", async () => {
    const client = await opened();
    const v = await showing(client);
    const id = client.document?.pages[1]?.id as string;
    await hook(client, "__failNextReopen");
    await hook(client, "__failNextUndo");

    const error = await rejection(v.applyEdit(turn(id)));

    expect(String((error as Error).message)).toContain("open it again");
    // Every edit after it is refused the same way, and the pages go on drawing as they were.
    expect(String(((await rejection(v.applyEdit(turn(id)))) as Error).message)).toContain("open it again");
    const page = await client.render({ page: 1, scale: 1 });
    expect([page.width, page.height]).toEqual([612, 792]);
    page.bitmap.close();
  });

  it("draws the file as it was after the worker restarts, the edit gone", async () => {
    const client = await opened();
    const v = await showing(client);
    await v.applyEdit(turn(client.document?.pages[1]?.id as string));
    await sharp(v, 1);
    const restarted = next(client, "restarted");

    void hook(client, "__crash").catch(() => undefined);
    expect(await restarted).toMatchObject({ reopened: true });

    await sharp(v, 1);
    const shell = document.querySelector<HTMLElement>('.lamina-page[data-page-index="1"]') as HTMLElement;
    expect(shell.offsetWidth / shell.offsetHeight).toBeCloseTo(612 / 792, 2);
    expect(isRed(pixel(canvasOf(1), ...UPRIGHT))).toBe(true);
    await expect
      .poll(() => (thumbnails()[1]?.width ?? 1) < (thumbnails()[1]?.height ?? 0), { timeout: 20_000 })
      .toBe(true);
  });

  it("edits a file read range by range: from a link, and from its OPFS copy", async () => {
    const path = "fixtures/corpus/outline.pdf";
    const sources = [
      { url: addressOf(path) },
      new File([(await read(path)) as BlobPart], "outline.pdf", { lastModified: 1_700_000_000_001 }),
    ];
    for (const source of sources) {
      rpc?.terminate();
      rpc = await testViewer("pdfium");
      const opened = await rpc.open(source, { copyAbove: 0, keepCopy: false });
      const before = opened.pages[0] as NonNullable<(typeof opened.pages)[number]>;
      const result = await rpc.applyEdit(turn(before.id));
      expect(result.pages[0]).toMatchObject({ width: before.height, height: before.width });
      const tile = await rpc.render({ page: 0, scale: 0.5 });
      expect(tile.width > tile.height).toBe(before.height > before.width);
      tile.bitmap.close();
    }
  });
});
