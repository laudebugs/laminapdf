// Organise's dialogs across files in the built reader (todo E2m, Paper O07 and O08), in this run's
// browser, as a person uses them: insert two pages of another PDF after a page, by range; insert
// from an encrypted PDF after typing its password (the pages come in, the result isn't
// encrypted); replace a page with a file's page; merge two files into the document; split it
// every two pages (a folder the person picks in Chromium, its picker stubbed; one zip in Firefox
// and WebKit, which can't write folders); and extract the selected pages as one file. Each edit is
// one undo step, saved and read back by Lamina PDF's SDK. The dialogs pass axe, and nothing breaks
// the CSP or reaches another origin.

import { readFileSync } from "node:fs";
import { Builder, Document, init } from "@laminapdf/sdk";
import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, serious, type Watched } from "./reader.ts";

const origin = inject("origin");

/**
 * No open or save pickers, so files come through the file input and saves download, in every
 * engine. In Chromium a folder picker stands in for the real one (which needs a person): the files
 * written into it are kept on `window.writtenFiles`, by name, as their sizes.
 */
const PICKERS = {
  script: (stub: number[]) => {
    Reflect.deleteProperty(window, "showOpenFilePicker");
    Reflect.deleteProperty(window, "showSaveFilePicker");
    if (stub[0] !== 1) return;
    const written: Record<string, number> = {};
    const folder = {
      kind: "directory",
      name: "Q3 parts",
      async getFileHandle(name: string, options?: { create?: boolean }) {
        if (!(name in written) && options?.create !== true) throw new DOMException("missing", "NotFoundError");
        return {
          createWritable: async () => ({
            write: async (chunk: Uint8Array) => {
              written[name] = chunk.length;
            },
            close: async () => undefined,
            abort: async () => undefined,
          }),
        };
      },
    };
    Object.assign(window, { writtenFiles: written, showDirectoryPicker: async () => folder });
  },
  arg: [BROWSER === "chromium" ? 1 : 0],
};

/** Off a tile's centre, which React Aria takes for a screen reader's press (see organise.e2e.test.ts). */
const OFF_CENTRE = { x: 20, y: 30 };

/** `count` pages of `width` × 792 points (so each file's pages are told apart by their width). */
function pdf(count: number, width: number): Buffer {
  using b = new Builder();
  for (let i = 0; i < count; i++) b.page([width, 792]).finish();
  using doc = b.finish();
  return Buffer.from(doc.save());
}

const ENCRYPTED = new URL("../../../fixtures/encrypted/r4-aes-128-user.pdf", import.meta.url);

/** Opens `bytes` in the reader and switches to Organise. */
async function organise(page: Page, bytes: Buffer, count: number): Promise<void> {
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Choose a file" }).click();
  await (await chooser).setFiles({ name: "Q3 report.pdf", mimeType: "application/pdf", buffer: bytes });
  await page.locator(".web-viewer canvas").first().waitFor({ timeout: 30_000 });
  await page.getByRole("button", { name: "Organise", exact: true }).click();
  await page
    .locator(".paper-page-tile")
    .nth(count - 1)
    .waitFor();
}

/** Chooses `item` in the toolbar's menu `menu` (Insert ▾ or More page actions). */
async function command(page: Page, menu: "Insert" | "More page actions", item: string): Promise<void> {
  await page.getByRole("button", { name: menu, exact: true }).click();
  await page.getByRole("menuitem", { name: item }).click();
}

/** Picks `files` in the file chooser the next step opens. */
async function picks(
  page: Page,
  step: () => Promise<void>,
  files: Parameters<import("playwright").FileChooser["setFiles"]>[0],
) {
  const chooser = page.waitForEvent("filechooser");
  await step();
  await (await chooser).setFiles(files);
}

/** Waits until the top bar counts `count` unsaved changes. */
async function unsaved(page: Page, count: number): Promise<void> {
  const text = count === 1 ? "1 unsaved change" : `${count} unsaved changes`;
  await expect.poll(() => page.locator(".paper-top-bar-save-state").textContent()).toBe(text);
}

/** Downloads the document as edited and opens it with the SDK. */
async function saved(page: Page): Promise<Document> {
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download", exact: true }).click();
  return Document.open(new Uint8Array(readFileSync(await (await download).path())));
}

/** The pages' widths, rounded, in order. */
const widths = (doc: Document) => doc.pages.map((p) => Math.round(p.width));

function opening(browser: Browser): Promise<Watched> {
  return load(browser, origin, "/", PICKERS, { ignoreHTTPSErrors: true, viewport: { width: 1400, height: 900 } });
}

describe(`organising across files in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    await init({ extract: true });
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  it("[cap C.02 web round-trip] [cap C.16 web round-trip] inserts pages from a file, from an encrypted one, and replaces a page", async () => {
    const watched = await opening(browser);
    const { page } = watched;
    await organise(page, pdf(4, 612), 4);

    // Pages 1–2 of a three-page file, after page 2.
    await page.locator(".paper-page-tile").nth(1).click({ position: OFF_CENTRE });
    await picks(page, () => command(page, "Insert", "Insert pages from a file…"), {
      name: "appendix.pdf",
      mimeType: "application/pdf",
      buffer: pdf(3, 400),
    });
    const insert = page.getByRole("dialog", { name: "Insert pages from a file" });
    await expect.poll(() => insert.getByRole("checkbox", { name: "Page 3" }).count()).toBe(1);
    await insert.getByRole("textbox", { name: "Pages to insert" }).fill("1–2");
    await expect.poll(() => insert.getByText("The new pages will be pages 3–4.").count()).toBe(1);
    await insert.getByRole("button", { name: "Insert 2 pages" }).click();
    await unsaved(page, 1);
    await expect.poll(() => page.locator(".paper-page-tile").count()).toBe(6);

    // The first page of an encrypted file, at the start: its password is asked for in the card.
    await page.locator(".paper-page-tile").nth(0).click({ position: OFF_CENTRE });
    await picks(page, () => command(page, "Insert", "Insert pages from a file…"), ENCRYPTED.pathname);
    // The dialog keeps the last file until it is chosen again.
    await insert.getByRole("button", { name: "Choose another…" }).waitFor();
    await picks(page, () => insert.getByRole("button", { name: "Choose another…" }).click(), ENCRYPTED.pathname);
    const password = insert.getByLabel("This file is protected. Password");
    await password.fill("wrong");
    await insert.getByRole("button", { name: "Open", exact: true }).click();
    await expect.poll(() => insert.getByText("That password didn’t open it. Try again.").count()).toBe(1);
    await password.fill("user-pw");
    await insert.getByRole("button", { name: "Open", exact: true }).click();
    await insert.getByRole("textbox", { name: "Pages to insert" }).fill("1");
    await insert.getByRole("combobox", { name: "Insert" }).selectOption({ label: "At the start" });
    await insert.getByRole("button", { name: "Insert 1 page" }).click();
    await unsaved(page, 2);

    // Replace the last page with page 3 of a third file.
    await page.locator(".paper-page-tile").nth(6).click({ position: OFF_CENTRE });
    await picks(page, () => command(page, "Insert", "Replace pages…"), {
      name: "fix.pdf",
      mimeType: "application/pdf",
      buffer: pdf(3, 300),
    });
    const replace = page.getByRole("dialog", { name: "Replace pages" });
    await replace.getByRole("textbox", { name: "Pages to insert" }).fill("3");
    await replace.getByRole("button", { name: "Replace 1 page with 1" }).click();
    await unsaved(page, 3);

    using doc = await saved(page);
    const all = widths(doc);
    expect([all.length, all.slice(1)]).toEqual([7, [612, 612, 400, 400, 612, 300]]);
    expect(doc.isEncrypted).toBe(false);
    await clean(watched);
  });

  it("[cap C.06 web round-trip] merges two files into the document, in the order chosen", async () => {
    const watched = await opening(browser);
    const { page } = watched;
    await organise(page, pdf(2, 612), 2);
    await command(page, "More page actions", "Merge documents…");
    const merge = page.getByRole("dialog", { name: "Merge documents" });
    await picks(page, () => merge.getByRole("button", { name: "Add files…" }).click(), [
      { name: "a.pdf", mimeType: "application/pdf", buffer: pdf(1, 400) },
      { name: "b.pdf", mimeType: "application/pdf", buffer: pdf(2, 300) },
    ]);
    await merge.getByRole("button", { name: "Move b.pdf up" }).click();
    expect(await serious(page)).toEqual([]);
    await merge.getByRole("button", { name: "Merge 3 files" }).click();
    await unsaved(page, 1);
    using doc = await saved(page);
    expect(widths(doc)).toEqual([612, 612, 300, 300, 400]);
    await clean(watched);
  });

  it("[cap C.07 web write] [cap C.05 web write] [cap H.10 web write] splits every two pages, and extracts the selected pages", async () => {
    const watched = await opening(browser);
    const { page } = watched;
    await organise(page, pdf(5, 612), 5);
    await command(page, "More page actions", "Split document…");
    const split = page.getByRole("dialog", { name: "Split document" });
    await split.getByRole("textbox", { name: "Pages in each file" }).fill("2");
    const preview = split.getByRole("list", { name: "You’ll get 3 files" });
    await expect.poll(() => preview.textContent()).toContain("Q3 report 3.pdf5");
    expect(await serious(page)).toEqual([]);
    if (BROWSER === "chromium") {
      await split.getByRole("button", { name: "Split into 3 files" }).click();
      await expect
        .poll(() => page.evaluate(() => Object.keys((window as unknown as { writtenFiles: object }).writtenFiles)))
        .toEqual(["Q3 report 1.pdf", "Q3 report 2.pdf", "Q3 report 3.pdf"]);
    } else {
      const download = page.waitForEvent("download");
      await split.getByRole("button", { name: "Split into 3 files" }).click();
      const zip = await download;
      expect(zip.suggestedFilename()).toBe("Q3 report parts.zip");
      const bytes = readFileSync(await zip.path());
      expect([bytes[0], bytes[1]]).toEqual([0x50, 0x4b]);
    }
    await expect.poll(() => split.count()).toBe(0);

    // Extract pages 2–3 as one file; the document doesn't change.
    await page.locator(".paper-page-tile").nth(1).click({ position: OFF_CENTRE });
    await page
      .locator(".paper-page-tile")
      .nth(2)
      .click({ position: OFF_CENTRE, modifiers: ["Shift"] });
    await command(page, "More page actions", "Extract pages…");
    const extract = page.getByRole("dialog", { name: "Extract pages" });
    expect(await extract.getByRole("textbox", { name: "Ranges" }).inputValue()).toBe("2–3");
    expect(await serious(page)).toEqual([]);
    const download = page.waitForEvent("download");
    await extract.getByRole("button", { name: "Save 1 file" }).click();
    const file = await download;
    expect(file.suggestedFilename()).toBe("Q3 report pages 2-3.pdf");
    using part = Document.open(new Uint8Array(readFileSync(await file.path())));
    expect(part.pageCount).toBe(2);
    expect(await page.locator(".paper-top-bar-save-state").textContent()).not.toMatch(/unsaved/);
    await clean(watched);
  });

  it("passes axe in Insert pages from a file", async () => {
    const watched = await opening(browser);
    const { page } = watched;
    await organise(page, pdf(2, 612), 2);
    await picks(page, () => command(page, "Insert", "Insert pages from a file…"), {
      name: "appendix.pdf",
      mimeType: "application/pdf",
      buffer: pdf(2, 400),
    });
    const insert = page.getByRole("dialog", { name: "Insert pages from a file" });
    await insert.getByRole("checkbox", { name: "Page 2" }).waitFor();
    expect(await serious(page)).toEqual([]);
    await page.keyboard.press("Escape");
    await clean(watched);
  });
});
