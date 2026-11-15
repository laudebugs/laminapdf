// Edits that insert, delete or move pages, in Chromium, Firefox and WebKit: the worker reports the
// new page order, and the viewer follows its pages by ID. After each edit, and after undo and redo,
// every page draws its own content at its own size, its text layer shows its own words, find lands
// on the right pages, Back and Forward reach the right pages, and the history names the pages a
// step touched where they are now. A move that keeps the page count redraws too.

import { Builder, Font, init, type Op } from "@laminapdf/sdk";
import { createViewer, ThumbnailStrip, type Viewer, type ViewerClient } from "@laminapdf/viewer";
import { afterEach, beforeAll, describe, expect, it } from "vitest";
import { testViewer } from "./helpers.js";

/** Four pages, each its own size, colour (a box at its top-left) and word. */
const PAGES = [
  { size: [612, 792], colour: [1, 0, 0], word: "Alpha" },
  { size: [420, 595], colour: [0, 0.6, 0], word: "Bravo" },
  { size: [792, 612], colour: [0, 0, 1], word: "Charlie" },
  { size: [500, 500], colour: [0, 0, 0], word: "Delta" },
] as const;

/** The box on every page, in page points from the top-left: x, y, width, height. */
const BOX = [20, 20, 120, 60] as const;

function fourPages(): Uint8Array {
  const b = new Builder({ id: new TextEncoder().encode("lamina-e2l-pages"), producer: "lamina tests" });
  const font = Font.standard("Helvetica");
  for (const { size, colour, word } of PAGES) {
    const page = b.page([...size]);
    const s = page.surface();
    s.fillColor([...colour])
      .path((p) => p.rect([...BOX]))
      .fill();
    s.fillColor([0, 0, 0]).text(word, { font, size: 24, at: [40, 200] });
    page.finish();
  }
  const doc = b.finish();
  const bytes = doc.save();
  doc.free();
  return bytes;
}

let bytes: Uint8Array;
let rpc: ViewerClient | undefined;
let view: Viewer | undefined;
let host: HTMLElement | undefined;
let strip: ThumbnailStrip | undefined;
let side: HTMLElement | undefined;

beforeAll(async () => {
  await init({ text: true });
  bytes = fourPages();
});

afterEach(() => {
  strip?.destroy();
  side?.remove();
  view?.destroy();
  host?.remove();
  rpc?.terminate();
  [rpc, view, host, strip, side] = [undefined, undefined, undefined, undefined, undefined];
});

/** A viewer 800 × 600 px and a thumbnail strip beside it, over the four pages. */
async function showing(): Promise<{ client: ViewerClient; v: Viewer; ids: string[] }> {
  rpc = await testViewer("pdfium");
  const opened = await rpc.open(bytes);
  host = document.createElement("div");
  Object.assign(host.style, { width: "800px", height: "600px", position: "fixed", top: "0", left: "0" });
  side = document.createElement("div");
  Object.assign(side.style, { width: "160px", height: "600px", position: "fixed", top: "0", left: "820px" });
  document.body.append(host, side);
  view = createViewer({ client: rpc });
  view.mount(host);
  view.open(opened);
  strip = new ThumbnailStrip(rpc, view);
  strip.mount(side);
  strip.open(opened);
  await expect.poll(() => view?.getState().pageCount).toBe(4);
  return { client: rpc, v: view, ids: opened.pages.map((p) => p.id) };
}

/** The words of the pages as the viewer shows them, in order: what each page's drawing and text say it is. */
async function shown(client: ViewerClient, v: Viewer): Promise<string[]> {
  const sizes = v.pageSizes();
  const words: string[] = [];
  for (let page = 0; page < sizes.length; page++) {
    const { runs } = await client.pageText(page);
    words.push(
      runs
        .map((run) => run.text)
        .join("")
        .trim(),
    );
  }
  return words;
}

/** Each page's size as the viewer lays it out. */
const sizes = (v: Viewer) => v.pageSizes().map((s) => [s.width, s.height]);
const sizeOf = (word: string) => [...(PAGES.find((p) => p.word === word)?.size ?? [612, 792])];

/** The colour at page `page`'s box, as the worker draws it now. */
async function colourAt(client: ViewerClient, page: number): Promise<number[]> {
  const { bitmap, width, height } = await client.render({ page, scale: 0.5 });
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext("2d") as CanvasRenderingContext2D;
  ctx.drawImage(bitmap, 0, 0);
  bitmap.close();
  const [x, y, w, h] = BOX;
  return [...ctx.getImageData(Math.floor((x + w / 2) * 0.5), Math.floor((y + h / 2) * 0.5), 1, 1).data].slice(0, 3);
}

/** Which page's colour `rgb` is: the nearest of the four, or "?" for white (a blank page). */
function wordOfColour(rgb: readonly number[]): string {
  const distance = (colour: readonly number[]) =>
    colour.reduce((sum, c, k) => sum + Math.abs(c * 255 - (rgb[k] ?? 0)), 0);
  if (distance([1, 1, 1]) < 60) return "?";
  const nearest = [...PAGES].sort((a, b) => distance(a.colour) - distance(b.colour))[0];
  return nearest?.word ?? "?";
}

/** Checks that the pages show `words`, in order: drawn, sized, read and listed in the strip. */
async function expectPages(client: ViewerClient, v: Viewer, words: string[]): Promise<void> {
  expect(v.getState().pageCount).toBe(words.length);
  expect(client.document?.pageCount).toBe(words.length);
  expect(sizes(v)).toEqual(words.map(sizeOf));
  expect(await shown(client, v)).toEqual(words);
  expect(await drawn(client, words.length)).toEqual(words.map((w) => (w === "" ? "?" : w)));
  // The strip lays out the new list: its first thumbnail has the first page's shape.
  const [w, h] = sizeOf(words[0] ?? "");
  await expect.poll(() => firstThumbnailShape(), { timeout: 20_000 }).toBe(Math.round(((w ?? 1) / (h ?? 1)) * 10));
}

/** Which page each of the first `count` pages draws as, by its box's colour. */
async function drawn(client: ViewerClient, count: number): Promise<string[]> {
  const words: string[] = [];
  for (let page = 0; page < count; page++) words.push(wordOfColour(await colourAt(client, page)));
  return words;
}

/** The first thumbnail's width over its height, times ten and rounded; undefined before it is laid out. */
function firstThumbnailShape(): number | undefined {
  const thumb = document.querySelector<HTMLElement>('.lamina-thumbnail[data-page-index="0"] .lamina-thumbnail-page');
  return thumb === null ? undefined : Math.round((thumb.offsetWidth / thumb.offsetHeight) * 10);
}

/** The page find's matches of `word` are on. */
async function found(v: Viewer, word: string): Promise<number[]> {
  v.find.find(word);
  await expect.poll(() => v.find.getState().done, { timeout: 20_000 }).toBe(true);
  return v.find.results().map((r) => r.page);
}

const del = (pages: string[]): Op => ({ op: "deletePages", args: { pages, references: "retarget" } }) as Op;
const move = (pages: string[], to: unknown): Op => ({ op: "movePages", args: { pages, to } }) as Op;
const reverse = (pages: string[]): Op => ({ op: "reversePages", args: { pages } }) as Op;
const insert = (after: string, count: number): Op =>
  ({
    op: "insertBlankPages",
    args: { at: { after }, count, size: { width: 612, height: 792 }, rotate: 0 },
  }) as Op;

describe("edits that change the page list", () => {
  it("deletes a page, then undoes and redoes it, every page showing its own content", async () => {
    const { client, v, ids } = await showing();
    await v.edits.apply([del([ids[1] as string])], "Delete page 2");
    await expectPages(client, v, ["Alpha", "Charlie", "Delta"]);
    expect(await found(v, "Charlie")).toEqual([1]);

    await v.edits.undo();
    await expectPages(client, v, ["Alpha", "Bravo", "Charlie", "Delta"]);
    expect(await found(v, "Charlie")).toEqual([2]);

    await v.edits.redo();
    await expectPages(client, v, ["Alpha", "Charlie", "Delta"]);
  });

  it("moves a page and keeps the count: the view redraws the moved pages", async () => {
    const { client, v, ids } = await showing();
    await v.edits.apply([move([ids[3] as string], "start")], "Move page 4");
    await expectPages(client, v, ["Delta", "Alpha", "Bravo", "Charlie"]);
    expect(await found(v, "Delta")).toEqual([0]);
    await expect.poll(() => v.stats().pages[0], { timeout: 20_000 }).toBe("sharp");

    await v.edits.undo();
    await expectPages(client, v, ["Alpha", "Bravo", "Charlie", "Delta"]);
    expect(await found(v, "Delta")).toEqual([3]);
  });

  it("reverses the pages, then undoes and redoes it", async () => {
    const { client, v, ids } = await showing();
    await v.edits.apply([reverse(ids)], "Reverse pages");
    await expectPages(client, v, ["Delta", "Charlie", "Bravo", "Alpha"]);
    await v.edits.undo();
    await expectPages(client, v, ["Alpha", "Bravo", "Charlie", "Delta"]);
    await v.edits.redo();
    await expectPages(client, v, ["Delta", "Charlie", "Bravo", "Alpha"]);
  });

  it("inserts blank pages, then undoes and redoes it", async () => {
    const { client, v, ids } = await showing();
    await v.edits.apply([insert(ids[0] as string, 2)], "Insert 2 pages");
    await expectPages(client, v, ["Alpha", "", "", "Bravo", "Charlie", "Delta"]);
    expect(await found(v, "Bravo")).toEqual([3]);
    await v.edits.undo();
    await expectPages(client, v, ["Alpha", "Bravo", "Charlie", "Delta"]);
    await v.edits.redo();
    await expectPages(client, v, ["Alpha", "", "", "Bravo", "Charlie", "Delta"]);
  });

  it("takes Back and Forward to the pages they went to before the move", async () => {
    const { v, ids } = await showing();
    v.jumpToPage(3);
    await expect.poll(() => v.getState().currentPage).toBe(3);
    // Delta, where the view is, moves to the front; Alpha, where Back goes, moves to second.
    await v.edits.apply([move([ids[3] as string], "start")], "Move page 4");
    expect(v.getState().currentPage).toBe(0);
    expect(v.history.back()).toBe(true);
    await expect.poll(() => v.getState().currentPage).toBe(1);
    expect(v.history.forward()).toBe(true);
    await expect.poll(() => v.getState().currentPage).toBe(0);
  });

  it("names the pages a step touched where they are now", async () => {
    const { v, ids } = await showing();
    const charlie = ids[2] as string;
    await v.edits.apply([{ op: "rotatePage", args: { page: charlie, rotate: 90 } } as Op], "Rotate page 3");
    expect(v.edits.getState().history.steps[0]?.pages).toEqual([2]);
    await v.edits.apply([move([charlie], "start")], "Move page 3");
    expect(v.edits.getState().history.steps[0]?.pages).toEqual([0]);
    expect(v.edits.getState().history.steps[0]?.pageIds).toEqual([charlie]);
  });

  it("keeps the pictures of a page that moved", async () => {
    const { client, v, ids } = await showing();
    v.goToPage(2);
    await expect.poll(() => v.stats().pages[2], { timeout: 20_000 }).toBe("sharp");
    // The page at the top of the view, drawn, goes to the front: the view follows it there.
    const top = v.getState().visibleRange.first;
    await expect.poll(() => v.stats().pages[top], { timeout: 20_000 }).toBe("sharp");
    const started: number[] = [];
    client.on("render-start", ({ page }) => started.push(page));
    await v.edits.apply([move([ids[top] as string], "start")], "Move a page");
    expect(v.getState().visibleRange.first).toBe(0);
    await expect.poll(() => v.stats().pages[0], { timeout: 20_000 }).toBe("sharp");
    expect(started).not.toContain(0);
  });

  it("follows an edit made through the client, whoever made it", async () => {
    const { client, v, ids } = await showing();
    await client.applyEdit([del([ids[0] as string])]);
    await expectPages(client, v, ["Bravo", "Charlie", "Delta"]);
  });
});
