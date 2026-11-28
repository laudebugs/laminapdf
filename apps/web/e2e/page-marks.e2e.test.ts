// The Page marks dialog and Manage page marks (Paper A03) in the built reader, in this run's browser,
// with the real mouse and keyboard: a watermark on every page of a 24-page file (the preview in its
// colour, the page stepper), saved, found again in Manage page marks, edited, saved, and removed; a
// header and footer over a page number the file already shows (the overlap notice), a range past the
// last page, then a Bates number from the keyboard alone; and a certified file's note. Each mark is
// one undo step, and pdf.js reads back what each draws. axe finds no serious violation in any tab.

import { readFileSync, writeFileSync } from "node:fs";
import { Builder, init } from "@laminapdf/sdk";
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";
import type { Browser, Download, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, serious, type Watched } from "./reader.ts";

const origin = inject("origin");
let browser: Browser;
const FOOTER = new URL("../../../fixtures/content/add/footer.pdf", import.meta.url);
const CERTIFIED = new URL("../../../fixtures/signed/certified-p1.pdf", import.meta.url);

/** A file of `count` A4 pages, each with a shape that moves from page to page. */
function manyPages(count: number): Uint8Array {
  using b = new Builder();
  for (let i = 0; i < count; i++) {
    const page = b.page("A4");
    page
      .surface()
      .fillColor([(i % 10) / 10, 0.4, 0.6])
      .path((p) => p.rect([56 + (i % 20) * 20, 56 + (i % 30) * 20, 160, 100]))
      .fill();
    page.finish();
  }
  using doc = b.finish();
  return doc.save();
}

/** Writes `bytes` where the file chooser can take them. */
function written(bytes: Uint8Array): string {
  const path = `${process.env.TMPDIR ?? "/tmp"}/page-marks-${Date.now()}-${Math.random().toString(36).slice(2)}.pdf`;
  writeFileSync(path, bytes);
  return path;
}

/** The reader with `file` open in Edit mode, no file pickers of the browser's own. */
async function edit(file: URL | Uint8Array): Promise<Watched> {
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
    { ignoreHTTPSErrors: true, viewport: { width: 1400, height: 1000 }, bypassCSP: true },
  );
  const { page } = watched;
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Choose a file" }).click();
  await (await chooser).setFiles(file instanceof URL ? file.pathname : written(file));
  await page.locator(".web-viewer canvas").first().waitFor({ timeout: 30_000 });
  await page.getByRole("button", { name: "Edit", exact: true }).click();
  await toolbar(page).waitFor();
  return watched;
}

/** The modal dialog titled `title`; the Page marks menu's popover is a dialog by role too, and has the same name. */
const dialogOf = (page: Page, title: string) =>
  page.locator('section[role="dialog"]').filter({ has: page.getByRole("heading", { name: title, exact: true }) });

const toolbar = (page: Page) => page.getByRole("toolbar", { name: "Edit tools" });

/** Opens the Page marks menu and chooses `item`. */
async function choose(page: Page, item: string): Promise<void> {
  await toolbar(page).getByRole("button", { name: "Page marks", exact: true }).click();
  await page.getByRole("menuitem", { name: item, exact: true }).click();
}

/** The count of unsaved changes the top bar says, once it says `count`. */
async function changes(page: Page, count: number): Promise<void> {
  const text = count === 0 ? "No unsaved changes" : count === 1 ? "1 unsaved change" : `${count} unsaved changes`;
  await expect.poll(() => page.locator(".paper-top-bar-save-state").textContent(), { timeout: 30_000 }).toBe(text);
}

/** One unsaved change, undone in a single step and redone: Undo from the top bar, Redo from the keyboard. */
async function undoAndRedo(page: Page): Promise<void> {
  // The toast has an Undo too: the top bar's is the one pressed.
  await page.locator(".paper-top-bar").getByRole("button", { name: "Undo", exact: true }).click();
  await changes(page, 0);
  await page.keyboard.press("ControlOrMeta+Shift+Z");
  await changes(page, 1);
}

/** Saves (a download) and returns the file. */
async function save(page: Page): Promise<Uint8Array> {
  const download: Promise<Download> = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download", exact: true }).click();
  return new Uint8Array(readFileSync(await (await download).path()));
}

/** The text pdf.js reads on page `number` (1-based) of `bytes`. */
async function textOf(bytes: Uint8Array, number: number): Promise<string> {
  const task = getDocument({ data: bytes.slice(), verbosity: 0 });
  const doc = await task.promise;
  try {
    const page = await doc.getPage(number);
    return (await page.getTextContent()).items.map((i) => ("str" in i ? i.str : "")).join(" ");
  } finally {
    await task.destroy();
  }
}

/** The reader with the saved `bytes` open in Edit mode. */
const reopen = (bytes: Uint8Array): Promise<Watched> => edit(bytes);

/** How many pixels of the preview's canvas are reddish: the mark in the colour it will print. */
function reddish(page: Page): Promise<number> {
  return page.locator(".paper-marks-page canvas").evaluate((canvas) => {
    const c = canvas as HTMLCanvasElement;
    const { data } = (c.getContext("2d") as CanvasRenderingContext2D).getImageData(0, 0, c.width, c.height);
    let count = 0;
    for (let i = 0; i < data.length; i += 4) {
      if ((data[i] ?? 0) > (data[i + 1] ?? 0) + 20 && (data[i] ?? 0) > (data[i + 2] ?? 0) + 20) count++;
    }
    return count;
  });
}

describe(`The Page marks dialog in ${BROWSER}`, () => {
  beforeAll(async () => {
    await init();
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  it("[cap B.10 web round-trip] adds a watermark to every page, then edits it and removes it from Manage page marks after a save and reopen", async () => {
    const watched = await edit(manyPages(24));
    const { page } = watched;
    await choose(page, "Watermark…");
    const dialog = dialogOf(page, "Page marks");
    await dialog.waitFor();
    const add = dialog.getByRole("button", { name: "Add watermark", exact: true });
    // Nothing to add yet, and the footer says why.
    expect(await add.isDisabled()).toBe(true);
    await dialog.getByText("Type the watermark text to add it.").first().waitFor();
    await dialog.getByRole("textbox", { name: "Text", exact: true }).fill("DRAFT");
    await dialog.getByRole("button", { name: "Red", exact: true }).click();
    // The preview draws the mark in its colour over the page, and the stepper looks at another page.
    await expect.poll(() => reddish(page)).toBeGreaterThan(100);
    await dialog.getByRole("button", { name: "Next page", exact: true }).click();
    await dialog.getByText("Page 2 · 2 of 24").waitFor();
    expect(await serious(page)).toEqual([]);
    await add.click();
    await dialog.waitFor({ state: "hidden" });
    await page.getByText("Watermark added to all 24 pages").waitFor();
    await changes(page, 1);
    // One step however many pages: Undo takes the whole watermark back, Redo puts it again.
    await undoAndRedo(page);

    const bytes = await save(page);
    expect(await textOf(bytes, 1)).toContain("DRAFT");
    expect(await textOf(bytes, 24)).toContain("DRAFT");
    await clean(watched);

    // Opened again, Manage page marks finds it; Edit changes its words on every page it is on.
    const again = await reopen(bytes);
    await choose(again.page, "Manage page marks…");
    const manage = dialogOf(again.page, "Manage page marks");
    await manage.getByText("Pages 1–24").waitFor();
    expect(await serious(again.page)).toEqual([]);
    await manage.getByRole("button", { name: "Edit Watermark: DRAFT" }).click();
    const editing = dialogOf(again.page, "Edit watermark");
    await editing.waitFor();
    const words = editing.getByRole("textbox", { name: "Text", exact: true });
    expect(await words.inputValue()).toBe("DRAFT");
    await words.fill("FINAL");
    await editing.getByRole("button", { name: "Update watermark", exact: true }).click();
    await editing.waitFor({ state: "hidden" });
    await changes(again.page, 1);
    await undoAndRedo(again.page);
    const edited = await save(again.page);
    expect(await textOf(edited, 2)).toContain("FINAL");
    expect(await textOf(edited, 2)).not.toContain("DRAFT");
    await clean(again);

    // And Remove takes it off every page, as one step that Undo takes back.
    const last = await reopen(edited);
    await choose(last.page, "Manage page marks…");
    const list = dialogOf(last.page, "Manage page marks");
    await list.getByRole("button", { name: "Remove Watermark: FINAL" }).click();
    await list.waitFor({ state: "hidden" });
    await changes(last.page, 1);
    await undoAndRedo(last.page);
    expect(await textOf(await save(last.page), 1)).not.toContain("FINAL");
    await clean(last);
  });

  it("[cap C.14 web round-trip] warns of the page number a file shows, refuses a range past the last page, and adds a header and footer", async () => {
    const watched = await edit(FOOTER);
    const { page } = watched;
    await choose(page, "Header & footer…");
    const dialog = dialogOf(page, "Page marks");
    await dialog.waitFor();
    // The page shows "Page 3 of 24" at the bottom centre: the default page number, at the right, clears it.
    const notice = dialog.getByText(/already shows a page number/);
    await page.waitForTimeout(500);
    await expect.poll(() => notice.count()).toBe(0);
    const centre = dialog.getByRole("textbox", { name: "Footer, centre", exact: true });
    await centre.fill("[Page] of [Pages]");
    await notice.waitFor();
    expect(await serious(page)).toEqual([]);
    await centre.fill("");
    await expect.poll(() => notice.count()).toBe(0);
    // A token goes into the slot the caret was in.
    const left = dialog.getByRole("textbox", { name: "Footer, left", exact: true });
    await left.fill("Sheet  of ");
    await left.evaluate((input) => (input as HTMLInputElement).setSelectionRange(6, 6));
    await dialog.getByRole("button", { name: "Page, inserts the page number", exact: true }).click();
    await expect.poll(() => left.inputValue()).toBe("Sheet [Page] of ");
    // Focus comes back to the slot, with the caret after the token.
    await expect.poll(() => left.evaluate((input) => document.activeElement === input)).toBe(true);
    expect(await left.evaluate((input) => (input as HTMLInputElement).selectionStart)).toBe(12);
    await left.evaluate((input) => (input as HTMLInputElement).setSelectionRange(16, 16));
    await dialog.getByRole("button", { name: "Pages, inserts the number of pages", exact: true }).click();
    await expect.poll(() => left.inputValue()).toBe("Sheet [Page] of [Pages]");

    // A range that names a page the file doesn't have turns the field red and waits.
    await dialog.getByText("A range", { exact: true }).click();
    await dialog.getByRole("textbox", { name: "Page range" }).fill("1–30");
    await dialog
      .getByText(/no page 30/)
      .first()
      .waitFor();
    const add = dialog.getByRole("button", { name: "Add header and footer", exact: true });
    expect(await add.isDisabled()).toBe(true);
    await dialog.getByRole("textbox", { name: "Page range" }).fill("1");
    await expect.poll(() => add.isDisabled()).toBe(false);
    expect(await serious(page)).toEqual([]);
    await add.click();
    await dialog.waitFor({ state: "hidden" });
    await changes(page, 1);
    const bytes = await save(page);
    expect(await textOf(bytes, 1)).toContain("Sheet 1 of 1");
    await clean(watched);
  });

  it("[cap C.14 web round-trip] adds Bates numbers by keyboard alone, from the palette", async () => {
    const watched = await edit(manyPages(3));
    const { page } = watched;
    await page.keyboard.press("ControlOrMeta+K");
    await page.keyboard.type("bates");
    await page.keyboard.press("Enter");
    const dialog = dialogOf(page, "Page marks");
    await dialog.waitFor();
    // The Bates tab opens with the prefix in focus.
    await page.keyboard.type("NHT-");
    expect(await serious(page)).toEqual([]);
    // Tab to the primary button and press it.
    for (let i = 0; i < 40; i++) {
      const name = await page.evaluate(() => document.activeElement?.textContent ?? "");
      if (name === "Add Bates numbers") break;
      await page.keyboard.press("Tab");
    }
    await page.keyboard.press("Enter");
    await dialog.waitFor({ state: "hidden" });
    await changes(page, 1);
    const bytes = await save(page);
    expect(await textOf(bytes, 1)).toContain("NHT-000001");
    expect(await textOf(bytes, 3)).toContain("NHT-000003");
    await clean(watched);
  });

  it("[cap B.10 web write] says a certified file's save would invalidate the certification", async () => {
    const watched = await edit(CERTIFIED);
    const { page } = watched;
    await choose(page, "Watermark…");
    const dialog = dialogOf(page, "Page marks");
    await dialog.waitFor();
    await dialog.getByText("Saving will invalidate the certification.").waitFor();
    await dialog.getByRole("button", { name: "Details", exact: true }).waitFor();
    expect(await serious(page)).toEqual([]);
    // The mark is still added (Undo takes it back); the save strip then names what saving would cost.
    await dialog.getByRole("textbox", { name: "Text", exact: true }).fill("DRAFT");
    await dialog.getByRole("button", { name: "Add watermark", exact: true }).click();
    await dialog.waitFor({ state: "hidden" });
    await page
      .getByRole("region", { name: "How this file saves" })
      .getByText(/Saving will invalidate the certification/)
      .waitFor();
    await clean(watched);
  });
});
