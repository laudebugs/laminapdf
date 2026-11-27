// Edit mode's tools for adding content (Paper A01, A02), in this run's browser, with the real mouse
// and keyboard: text typed into a box on the page (with a symbol from the picker), a picture from the
// file picker, and a rectangle, an ellipse, a line and a polygon drawn; each selected, moved, nudged,
// resized, turned and faded, then saved and read back by pdf.js, and opened again in the reader,
// where each is found and selectable. axe finds no serious violation in the inspector or the picker.

import { readFileSync, writeFileSync } from "node:fs";
import { deflateSync } from "node:zlib";
import { getDocument, OPS } from "pdfjs-dist/legacy/build/pdf.mjs";
import type { Browser, Download, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, serious, type Watched } from "./reader.ts";

const origin = inject("origin");
let browser: Browser;
const TEXT = new URL("../../../fixtures/reading-order/two-columns-ordered.pdf", import.meta.url);
const PAGE = '.lamina-page[data-page-index="0"]';
/** Page 1's width in points. */
const WIDTH = 595.275591;

/** A `width` by `height` PNG of one colour: a logo to place. */
function png(width: number, height: number): Buffer {
  const crc = (bytes: Buffer): number => {
    let c = 0xffffffff;
    for (const b of bytes) {
      c ^= b;
      for (let k = 0; k < 8; k++) c = c & 1 ? (c >>> 1) ^ 0xedb88320 : c >>> 1;
    }
    return (c ^ 0xffffffff) >>> 0;
  };
  const chunk = (type: string, data: Buffer): Buffer => {
    const body = Buffer.concat([Buffer.from(type), data]);
    const out = Buffer.alloc(body.length + 8);
    out.writeUInt32BE(data.length, 0);
    body.copy(out, 4);
    out.writeUInt32BE(crc(body), body.length + 4);
    return out;
  };
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header.set([8, 2, 0, 0, 0], 8);
  const row = Buffer.concat([Buffer.from([0]), Buffer.alloc(width * 3, 0x55)]);
  const raw = Buffer.concat(Array.from({ length: height }, () => row));
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", header),
    chunk("IDAT", deflateSync(raw)),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/** The reader with `file` open in Edit mode, no file pickers of the browser's own. */
async function edit(file: URL | string): Promise<Watched> {
  const watched = await load(
    browser,
    origin,
    "/",
    {
      script: () => {
        Reflect.deleteProperty(window, "showOpenFilePicker");
        Reflect.deleteProperty(window, "showSaveFilePicker");
      },
    } as never,
    { ignoreHTTPSErrors: true, viewport: { width: 1400, height: 1600 }, bypassCSP: true },
  );
  const { page } = watched;
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Choose a file" }).click();
  await (await chooser).setFiles(typeof file === "string" ? file : file.pathname);
  await page.locator(".web-viewer canvas").first().waitFor({ timeout: 30_000 });
  await page.getByRole("button", { name: "Edit", exact: true }).click();
  await toolbar(page).waitFor();
  return watched;
}

const toolbar = (page: Page) => page.getByRole("toolbar", { name: "Edit tools" });
const inspectorOf = (page: Page) => page.getByRole("complementary", { name: "Added content" });

/** The viewport position of (`x`, `y`) points from page 1's top-left. */
async function at(page: Page, x: number, y: number): Promise<{ x: number; y: number }> {
  const box = await page.locator(PAGE).boundingBox();
  if (box === null) throw new Error("page 1 isn't shown");
  const scale = box.width / WIDTH;
  return { x: box.x + x * scale, y: box.y + y * scale };
}

/** A click on page 1 at (`x`, `y`) points. */
async function click(page: Page, x: number, y: number): Promise<void> {
  const point = await at(page, x, y);
  await page.mouse.move(point.x, point.y, { steps: 3 });
  await page.mouse.click(point.x, point.y);
}

/** A drag on page 1 from (`x0`, `y0`) to (`x1`, `y1`) points. */
async function drag(page: Page, [x0, y0]: [number, number], [x1, y1]: [number, number]): Promise<void> {
  const from = await at(page, x0, y0);
  const to = await at(page, x1, y1);
  await page.mouse.move(from.x, from.y);
  await page.mouse.down();
  await page.mouse.move((from.x + to.x) / 2, (from.y + to.y) / 2, { steps: 4 });
  await page.mouse.move(to.x, to.y, { steps: 4 });
  await page.mouse.up();
}

/** The count of unsaved changes the top bar says, once it says `count`. */
async function changes(page: Page, count: number): Promise<void> {
  const text = count === 1 ? "1 unsaved change" : `${count} unsaved changes`;
  await expect.poll(() => page.locator(".paper-top-bar-save-state").textContent(), { timeout: 30_000 }).toBe(text);
}

/** Saves (a download) and returns the file. */
async function save(page: Page): Promise<Uint8Array> {
  const download: Promise<Download> = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download", exact: true }).click();
  return new Uint8Array(readFileSync(await (await download).path()));
}

/** What pdf.js reads of page 1 of `bytes`: its text, and the count of pictures and paths it draws. */
async function read(bytes: Uint8Array) {
  // pdf.js takes the buffer it is given, so it reads a copy: the bytes are opened in the reader after.
  const task = getDocument({ data: bytes.slice(), verbosity: 0 });
  const doc = await task.promise;
  try {
    const page = await doc.getPage(1);
    const text = (await page.getTextContent()).items.map((i) => ("str" in i ? i.str : "")).join("");
    const ops = (await page.getOperatorList()).fnArray;
    const count = (op: number) => ops.filter((o) => o === op).length;
    return {
      text,
      images: count(OPS.paintImageXObject),
      paths: count(OPS.constructPath),
    };
  } finally {
    await task.destroy();
  }
}

/** The reader with the saved `bytes` open in Edit mode, to find what was added again. */
function reopen(bytes: Uint8Array): Promise<Watched> {
  const path = `${process.env.TMPDIR ?? "/tmp"}/content-edit-${Date.now()}-${Math.random().toString(36).slice(2)}.pdf`;
  writeFileSync(path, bytes);
  return edit(path);
}

/** The inspector's X, Y, W and H fields as numbers. */
async function geometry(page: Page): Promise<{ x: number; y: number; w: number; h: number }> {
  const field = async (name: string) =>
    Number(await inspectorOf(page).getByRole("textbox", { name, exact: true }).inputValue());
  return { x: await field("X"), y: await field("Y"), w: await field("W"), h: await field("H") };
}

/** Opens Insert symbol… from the inspector, picks the symbol named `name` and closes it. */
async function symbol(page: Page, name: string): Promise<void> {
  await inspectorOf(page).getByRole("button", { name: "Insert symbol…" }).click();
  const dialog = page.getByRole("dialog", { name: "Insert symbol" });
  await dialog.getByRole("textbox", { name: "Search symbols" }).fill(name);
  expect(await serious(page)).toEqual([]);
  await dialog.getByRole("button", { name, exact: true }).click();
  await dialog.getByRole("button", { name: "Done", exact: true }).click();
  await dialog.waitFor({ state: "hidden" });
}

describe(`Edit mode's content tools in ${BROWSER}`, () => {
  beforeAll(async () => {
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  it("[cap A.01 web round-trip] [cap A.12 web round-trip] types text into a box and a symbol from the picker, saves it, and finds it again", async () => {
    const watched = await edit(TEXT);
    const { page } = watched;
    await toolbar(page).getByRole("button", { name: "Add text", exact: true }).click();
    await drag(page, [72, 60], [330, 60]);
    const box = page.getByRole("textbox", { name: "New text box on page 1, edit text" });
    await box.waitFor();
    await box.pressSequentially("Reviewed ");
    // Inside a box being typed, the top bar says nothing is unsaved yet.
    expect(await page.locator(".paper-top-bar-save-state").textContent()).not.toMatch(/unsaved change/);
    await symbol(page, "right arrow");
    await box.pressSequentially(" done");
    expect(await serious(page)).toEqual([]);
    await box.press("Escape");
    await changes(page, 1);
    // The box is an object now: selected, with its inspector.
    const inspector = inspectorOf(page);
    await inspector.getByRole("heading", { name: "Text box" }).waitFor();
    expect(await serious(page)).toEqual([]);

    const bytes = await save(page);
    expect((await read(bytes)).text).toContain("Reviewed → done");

    // Opened again, it is found where it was: a click selects it and the inspector names it.
    const again = await reopen(bytes);
    await click(again.page, 80, 65);
    await inspectorOf(again.page).getByRole("heading", { name: "Text box" }).waitFor();
    await clean(again);
    await clean(watched);
  });

  it("[cap B.01 web write] [cap B.04 web round-trip] [cap B.09 web round-trip] places a picture from the picker, then moves, nudges, turns, fades and deletes it", async () => {
    const watched = await edit(TEXT);
    const { page } = watched;
    const chooser = page.waitForEvent("filechooser");
    await toolbar(page).getByRole("button", { name: "Image", exact: true }).click();
    await (await chooser).setFiles({ name: "northwind-logo.png", mimeType: "image/png", buffer: png(192, 64) });
    await changes(page, 1);
    const inspector = inspectorOf(page);
    await inspector.getByRole("heading", { name: "Image" }).waitFor();
    await inspector.getByText("northwind-logo.png · 192 × 64 px · PNG").waitFor();
    expect(await serious(page)).toEqual([]);

    // Moved by the arrow keys, which gather into one step; Shift moves ten points.
    const x = inspector.getByRole("textbox", { name: "X", exact: true });
    const before = Number(await x.inputValue());
    await page.locator(".lamina-viewer").first().focus();
    await page.keyboard.press("ArrowRight");
    await page.keyboard.press("Shift+ArrowRight");
    await expect.poll(async () => Number(await x.inputValue()), { timeout: 10_000 }).toBeCloseTo(before + 11, 1);
    await changes(page, 2);

    // Faded: the slider takes the value when it is let go of, one step.
    const opacity = inspector.getByRole("slider", { name: "Opacity" });
    await opacity.fill("50");
    await opacity.blur();
    await inspector.getByText("50%", { exact: true }).waitFor();

    // Turned by its field.
    const rotate = inspector.getByRole("textbox", { name: "Rotate", exact: true });
    await rotate.fill("30");
    await rotate.press("Enter");
    await expect.poll(async () => Number(await rotate.inputValue()), { timeout: 10_000 }).toBe(30);

    // Undo names the step.
    expect(await page.getByRole("button", { name: /^Undo/ }).isEnabled()).toBe(true);

    const where = await geometry(page);
    const bytes = await save(page);
    expect((await read(bytes)).images).toBeGreaterThan(0);

    // Opened again, it is found where it was, as faded as it was, and can be changed.
    const again = await reopen(bytes);
    await click(again.page, where.x + where.w / 2, where.y + where.h / 2);
    const found = inspectorOf(again.page);
    await found.getByRole("heading", { name: "Image" }).waitFor();
    await found.getByText("50%", { exact: true }).waitFor();
    expect(Math.round((await geometry(again.page)).x)).toBe(Math.round(where.x));
    await clean(again);

    await inspector.getByRole("button", { name: "Delete image", exact: true }).click();
    await inspector.waitFor({ state: "hidden" });
    await clean(watched);
  });

  it("[cap B.05 web round-trip] [cap B.02 web write] draws a rectangle, an ellipse, a line and a polygon, resizes one by a handle, and saves them", async () => {
    const watched = await edit(TEXT);
    const { page } = watched;
    const shape = async (name: string) => {
      await toolbar(page).getByRole("button", { name: "Choose shape", exact: true }).click();
      await page.getByRole("menuitemradio", { name }).click();
    };
    // Rectangle by a drag.
    await shape("Rectangle");
    await drag(page, [100, 100], [220, 160]);
    const inspector = inspectorOf(page);
    await inspector.getByRole("heading", { name: "Rectangle" }).waitFor();
    await changes(page, 1);
    // Resized by its east handle: the width field follows.
    const width = inspector.getByRole("textbox", { name: "W", exact: true });
    const wide = Number(await width.inputValue());
    const handle = page.locator('[data-content-handle="e"]:not([hidden])').first();
    const grip = await handle.boundingBox();
    if (grip === null) throw new Error("no east handle");
    await page.mouse.move(grip.x + grip.width / 2, grip.y + grip.height / 2);
    await page.mouse.down();
    await page.mouse.move(grip.x + grip.width / 2 + 40, grip.y + grip.height / 2, { steps: 4 });
    await page.mouse.up();
    await expect.poll(async () => Number(await width.inputValue()), { timeout: 10_000 }).toBeGreaterThan(wide + 10);
    await changes(page, 2);
    // An ellipse by a click: 72 points square.
    await shape("Ellipse");
    await click(page, 300, 300);
    await inspector.getByRole("heading", { name: "Ellipse" }).waitFor();
    await expect.poll(() => inspector.getByRole("textbox", { name: "W", exact: true }).inputValue()).toBe("72");
    // A line by a drag, a polygon by three clicks and Enter.
    await shape("Line");
    await drag(page, [100, 400], [250, 400]);
    await inspector.getByRole("heading", { name: "Line" }).waitFor();
    await shape("Polygon");
    await click(page, 400, 400);
    await click(page, 480, 400);
    await click(page, 440, 470);
    // The page took the press, so Enter is the page's: it closes the polygon.
    await page.keyboard.press("Enter");
    await inspector.getByRole("heading", { name: "Polygon" }).waitFor();
    await changes(page, 5);
    expect(await serious(page)).toEqual([]);

    const bytes = await save(page);
    expect((await read(bytes)).paths).toBeGreaterThanOrEqual(4);

    // Opened again, each is found by a click inside it.
    const again = await reopen(bytes);
    await click(again.page, 300 + 36, 300 + 36);
    await inspectorOf(again.page).getByRole("heading", { name: "Ellipse" }).waitFor();
    await click(again.page, 110, 130);
    await inspectorOf(again.page).getByRole("heading", { name: "Rectangle" }).waitFor();
    await clean(again);
    await clean(watched);
  });

  it("refuses a file that isn't a JPEG or PNG in a dialog that names the formats", async () => {
    const watched = await edit(TEXT);
    const { page } = watched;
    const chooser = page.waitForEvent("filechooser");
    await toolbar(page).getByRole("button", { name: "Image", exact: true }).click();
    await (await chooser).setFiles({
      name: "holiday.heic",
      mimeType: "image/heic",
      buffer: Buffer.from("not a picture"),
    });
    const dialog = page.getByRole("dialog", { name: "lamina can’t place this file" });
    await dialog.getByText("JPEG or PNG").waitFor();
    await dialog.getByRole("button", { name: "OK", exact: true }).click();
    await dialog.waitFor({ state: "hidden" });
    await clean(watched);
  });
});
