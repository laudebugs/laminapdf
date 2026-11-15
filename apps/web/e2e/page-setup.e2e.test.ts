// Organise's Page setup ▾ in the built reader (todo E2g, Paper O09 and O10), in this run's browser,
// as a person uses it: crop a page by its margins, with the notice that the cut edges stay in the
// file; resize one to A4; give one its own trim box; make one landscape; and set page labels
// (roman front matter, then 1, 2, 3), which the thumbnails show at once. Each is one undo step.
// Saved (Save downloads where the browser can't write files) and read back by Lamina PDF's SDK,
// the file holds each change. The dialogs pass axe, and nothing breaks the CSP or reaches
// another origin.

import { readFileSync } from "node:fs";
import { Builder, Document, init } from "@laminapdf/sdk";
import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, serious } from "./reader.ts";

const origin = inject("origin");

/** No pickers: Save becomes Download, in every engine, so the saved bytes are the download's. */
const NO_PICKERS = {
  script: () => {
    Reflect.deleteProperty(window, "showOpenFilePicker");
    Reflect.deleteProperty(window, "showSaveFilePicker");
  },
  arg: [],
};

/** Off a tile's centre, which React Aria takes for a screen reader's press (see organise.e2e.test.ts). */
const OFF_CENTRE = { x: 20, y: 30 };

/** Six US Letter pages, each with a box near its top edge. */
function sixPages(): Buffer {
  using b = new Builder();
  for (let i = 0; i < 6; i++) {
    const page = b.page("Letter");
    page
      .surface()
      .fillColor([0.2, 0.4, 0.6])
      .path((p) => p.rect([56, 20, 200, 40]))
      .fill();
    page.finish();
  }
  using doc = b.finish();
  return Buffer.from(doc.save());
}

/** Opens `bytes` in the reader and switches to Organise. */
async function organise(page: Page, bytes: Buffer): Promise<void> {
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Choose a file" }).click();
  await (await chooser).setFiles({ name: "six.pdf", mimeType: "application/pdf", buffer: bytes });
  await page.locator(".web-viewer canvas").first().waitFor({ timeout: 30_000 });
  await page.getByRole("button", { name: "Organise", exact: true }).click();
  await page.locator(".paper-page-tile").nth(5).waitFor();
}

/** Selects the tile at `index` alone. */
async function pick(page: Page, index: number): Promise<void> {
  await page.locator(".paper-page-tile").nth(index).click({ position: OFF_CENTRE });
}

/** Opens Page setup ▾ and chooses `item`. */
async function setup(page: Page, item: string): Promise<void> {
  await page.getByRole("button", { name: "Page setup" }).click();
  await page.getByRole("menuitem", { name: item }).click();
}

/** Waits until the top bar counts `count` unsaved changes. */
async function unsaved(page: Page, count: number): Promise<void> {
  const text = count === 1 ? "1 unsaved change" : `${count} unsaved changes`;
  await expect.poll(() => page.locator(".paper-top-bar-save-state").textContent()).toBe(text);
}

/** The captions under the thumbnails, in order. */
function captions(page: Page): Promise<string[]> {
  return page.locator(".paper-page-tile-number").allTextContents();
}

describe(`page setup in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    await init({ extract: true });
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  it("[cap C.09 web round-trip] [cap C.10 web round-trip] [cap C.11 web round-trip] [cap C.12 web round-trip] [cap I.10 web round-trip] crops, resizes, sets boxes, turns and labels pages through the dialogs", async () => {
    const watched = await load(browser, origin, "/", NO_PICKERS, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1400, height: 900 },
      // Millimetres, whatever the machine's locale.
      locale: "en-GB",
    });
    const { page } = watched;
    await organise(page, sixPages());

    // Crop page 1: 10 mm off the top, and the notice that the edge stays in the file.
    await pick(page, 0);
    await setup(page, "Crop pages…");
    const crop = page.getByRole("dialog", { name: "Crop pages" });
    await expect.poll(() => crop.getByRole("status").textContent()).toContain("Cropped content stays in the file.");
    await crop.getByRole("textbox", { name: "Top" }).fill("10");
    await expect.poll(() => crop.getByText(/^Visible area 216 × 269 mm/).count()).toBe(1);
    await crop.getByRole("button", { name: "Crop page", exact: true }).click();
    await unsaved(page, 1);

    // Resize page 2 to A4, scaling its content.
    await pick(page, 1);
    await setup(page, "Resize pages…");
    const resize = page.getByRole("dialog", { name: "Resize pages" });
    await resize.getByRole("combobox", { name: "New size" }).selectOption({ label: "A4" });
    await resize.getByRole("button", { name: "Resize page", exact: true }).click();
    await unsaved(page, 2);

    // Give page 3 its own trim box, half an inch in from each edge.
    await pick(page, 2);
    await setup(page, "Page boxes…");
    const boxes = page.getByRole("dialog", { name: "Page boxes" });
    const trim = boxes.getByRole("group", { name: "Trim box" });
    await trim.getByText("Same as crop box").click();
    await expect.poll(() => trim.getByRole("checkbox").isChecked()).toBe(false);
    for (const [side, value] of [
      ["left", "36"],
      ["bottom", "36"],
      ["right", "576"],
      ["top", "756"],
    ]) {
      await trim.getByRole("textbox", { name: `Trim box, ${side}` }).fill(value as string);
    }
    await boxes.getByRole("button", { name: "Apply boxes" }).click();
    await unsaved(page, 3);

    // Make page 4 landscape.
    await pick(page, 3);
    await setup(page, "Make landscape");
    await unsaved(page, 4);

    // Labels: i, ii for the first two pages, then 1, 2, 3 from page 3.
    await setup(page, "Page labels…");
    const labels = page.getByRole("dialog", { name: "Page labels" });
    await labels.getByRole("combobox", { name: "Range 1, style" }).selectOption({ label: "i, ii, iii" });
    await labels.getByRole("button", { name: "Add range" }).click();
    await labels.getByRole("textbox", { name: "Range 2, from page" }).fill("3");
    const preview = labels.getByRole("list", { name: "Labels by page" });
    await expect.poll(() => preview.textContent()).toContain("3 → 1");
    await labels.getByRole("button", { name: "Apply labels" }).click();
    await unsaved(page, 5);
    await expect.poll(() => captions(page)).toEqual(["i", "ii", "1", "2", "3", "4"]);

    const download = page.waitForEvent("download");
    await page.getByRole("button", { name: "Download", exact: true }).click();
    using doc = Document.open(new Uint8Array(readFileSync(await (await download).path())));
    const pages = doc.pages;
    expect(pages.map((p) => p.label)).toEqual(["i", "ii", "1", "2", "3", "4"]);
    // 10 mm is 28.35 pt off the top of the crop box.
    expect(doc.pageBoxes(0).crop[3]).toBeCloseTo(792 - (10 * 72) / 25.4, 1);
    expect([pages[1]?.width, pages[1]?.height].map((n) => Math.round(n ?? 0))).toEqual([595, 842]);
    expect([doc.pageBoxes(2).trim, doc.pageBoxes(2).own.trim]).toEqual([[36, 36, 576, 756], true]);
    expect([pages[3]?.rotation, (pages[3]?.width ?? 0) > (pages[3]?.height ?? 0)]).toEqual([90, true]);
    await clean(watched);
  });

  it("passes axe in each dialog, and refuses labels that start twice on one page", async () => {
    const watched = await load(browser, origin, "/", NO_PICKERS, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1400, height: 900 },
      bypassCSP: true,
    });
    const { page } = watched;
    await organise(page, sixPages());
    await pick(page, 0);
    for (const [item, name] of [
      ["Crop pages…", "Crop pages"],
      ["Resize pages…", "Resize pages"],
      ["Page boxes…", "Page boxes"],
      ["Page labels…", "Page labels"],
    ] as const) {
      await setup(page, item);
      const dialog = page.getByRole("dialog", { name });
      await dialog.waitFor();
      expect(await serious(page)).toEqual([]);
      if (name !== "Page labels") {
        await page.keyboard.press("Escape");
        await expect.poll(() => dialog.count()).toBe(0);
      }
    }
    const labels = page.getByRole("dialog", { name: "Page labels" });
    await labels.getByRole("button", { name: "Add range" }).click();
    // Focus went to the new row's first field; two rows can't both start on page 1.
    await expect
      .poll(() => page.evaluate(() => document.activeElement?.getAttribute("aria-label")))
      .toBe("Range 2, from page");
    await labels.getByRole("textbox", { name: "Range 2, from page" }).fill("1");
    // Both rows say so, beside their From fields.
    await expect.poll(() => labels.getByText("Two ranges can't start on page 1").count()).toBe(2);
    expect(await labels.getByRole("button", { name: "Apply labels" }).isDisabled()).toBe(true);
    expect(await serious(page)).toEqual([]);
    await page.keyboard.press("Escape");
    expect(await page.locator(".paper-top-bar-save-state").textContent()).not.toMatch(/unsaved/);
    await clean(watched);
  });
});
