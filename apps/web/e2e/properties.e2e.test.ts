// Editing a document's properties in the built reader (todo E1f, Paper O04) and the capability
// page (O05), in this run's browser:
// - each field of the Description tab changed, applied as one edit, saved (where the browser
//   can't write files, Save downloads), and read back from the download by Lamina PDF's SDK and by
//   pdf.js: the Info dictionary and the XMP metadata both hold the new values;
// - the creation date set, and the save setting the modification date; a file without XMP
//   metadata gets it;
// - values removed with Remove metadata… leave both, with the notice that earlier versions keep
//   them;
// - a PDF/UA file's title emptied shows the warning under the title;
// - axe finds no serious violation on the dialog or the page;
// - the page at /capabilities, its search, and Back to the open document.
// No CSP violation or other origin throughout.

import { readFileSync } from "node:fs";
import { Document, type DocumentMetadata, init } from "@laminapdf/sdk";
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";
import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, serious, type Watched } from "./reader.ts";

const origin = inject("origin");
const GOLDEN = new URL("../../../fixtures/golden/", import.meta.url);

/** No pickers: Save becomes Download, in every engine, so the saved bytes are the download's. */
const NO_PICKERS = {
  script: () => {
    Reflect.deleteProperty(window, "showOpenFilePicker");
    Reflect.deleteProperty(window, "showSaveFilePicker");
  },
  arg: [],
};

/** Chooses the More menu's item `name`. */
async function more(page: Page, name: string): Promise<void> {
  await page.getByRole("button", { name: "More" }).click();
  await page.getByRole("menuitem", { name }).click();
}

/** Opens Document properties and waits for its form. */
async function properties(page: Page) {
  await more(page, "Document properties…");
  const dialog = page.getByRole("dialog", { name: "Document properties" });
  await dialog.getByLabel("Title").waitFor();
  return dialog;
}

/** Applies the form, waits for the edit to count, saves, and returns the downloaded bytes. */
async function applyAndSave(page: Page): Promise<Uint8Array> {
  await page.getByRole("button", { name: "Apply" }).click();
  await expect.poll(() => page.locator(".paper-top-bar-save-state").textContent()).toBe("1 unsaved change");
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download", exact: true }).click();
  return new Uint8Array(readFileSync(await (await download).path()));
}

/** The saved file's metadata as Lamina PDF reads it, Info and XMP together. */
function lamina(bytes: Uint8Array): DocumentMetadata {
  using doc = Document.open(bytes);
  return doc.metadata();
}

/** The saved file's Info entries and XMP properties as pdf.js reads them, apart. */
async function pdfjs(bytes: Uint8Array): Promise<{ info: Record<string, unknown>; xmp: Record<string, unknown> }> {
  const task = getDocument({ data: bytes.slice(), verbosity: 0 });
  const doc = await task.promise;
  try {
    const { info, metadata } = await doc.getMetadata();
    return {
      info: info as Record<string, unknown>,
      xmp: metadata === null ? {} : Object.fromEntries(metadata as unknown as Iterable<[string, unknown]>),
    };
  } finally {
    await task.destroy();
  }
}

describe(`document properties in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    await init();
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  /** The reader, in a time zone half an hour off the hour, with golden file `name` open. */
  async function opened(name: string, options: { axe?: boolean } = {}): Promise<Watched> {
    const watched = await load(browser, origin, "/", NO_PICKERS, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1280, height: 900 },
      timezoneId: "Asia/Kolkata",
      ...(options.axe === true ? { bypassCSP: true } : {}),
    });
    const { page } = watched;
    const chooser = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Choose a file" }).click();
    await (await chooser).setFiles(new URL(name, GOLDEN).pathname);
    await page.locator(".web-viewer canvas").first().waitFor();
    return watched;
  }

  it("[cap J.01 web round-trip] [cap J.03 web round-trip] sets title, authors, subject, keywords, creator and producer in Info and XMP", async () => {
    const watched = await opened("metadata.pdf");
    const { page } = watched;
    const dialog = await properties(page);
    await dialog.getByLabel("Title").fill("Quarterly figures");
    await dialog.getByLabel("Author").fill("Ada Lovelace; Charles Babbage");
    await dialog.getByLabel("Subject").fill("Repairs and rent, July to September");
    await dialog.getByRole("button", { name: "Remove keyword lamina" }).click();
    await dialog.getByLabel("Keywords", { exact: true }).fill("board");
    await dialog.getByLabel("Keywords", { exact: true }).press("Enter");
    await dialog.getByLabel("Creator").fill("Report template 2.3");
    await dialog.getByLabel("Producer").fill("Lamina PDF");
    const saved = await applyAndSave(page);

    const read = lamina(saved);
    expect({ ...read, created: undefined, modified: undefined, source: undefined }).toMatchObject({
      title: "Quarterly figures",
      authors: ["Ada Lovelace", "Charles Babbage"],
      subject: "Repairs and rent, July to September",
      keywords: "pdf, xmp, info, board",
      creator: "Report template 2.3",
      producer: "Lamina PDF",
      disagreements: [],
    });
    const { info, xmp } = await pdfjs(saved);
    expect([info.Title, info.Author, info.Subject, info.Keywords, info.Creator, info.Producer]).toEqual([
      "Quarterly figures",
      "Ada Lovelace; Charles Babbage",
      "Repairs and rent, July to September",
      "pdf, xmp, info, board",
      "Report template 2.3",
      "Lamina PDF",
    ]);
    expect([
      xmp["dc:title"],
      xmp["dc:creator"],
      xmp["dc:description"],
      xmp["pdf:keywords"],
      xmp["xmp:creatortool"],
      xmp["pdf:producer"],
    ]).toEqual([
      "Quarterly figures",
      ["Ada Lovelace", "Charles Babbage"],
      "Repairs and rent, July to September",
      "pdf, xmp, info, board",
      "Report template 2.3",
      "Lamina PDF",
    ]);
    // Reopened in the reader, the dialog shows the saved values.
    const chooser = page.waitForEvent("filechooser");
    await more(page, "Choose a file");
    await page.getByRole("button", { name: "Choose a file" }).click();
    await (await chooser).setFiles({ name: "metadata.pdf", mimeType: "application/pdf", buffer: Buffer.from(saved) });
    await page.locator(".web-viewer canvas").first().waitFor();
    const reopened = await properties(page);
    expect([
      await reopened.getByLabel("Title").inputValue(),
      await reopened.getByLabel("Author").inputValue(),
      await reopened.getByRole("button", { name: "Remove keyword board" }).count(),
    ]).toEqual(["Quarterly figures", "Ada Lovelace; Charles Babbage", 1]);
    await page.keyboard.press("Escape");
    await clean(watched);
  });

  it("[cap J.02 web round-trip] sets the creation date, and the save sets the modification date, in a file that had no XMP", async () => {
    const watched = await opened("links-and-outlines.pdf");
    const { page } = watched;
    const before = Date.now();
    const dialog = await properties(page);
    await dialog.getByLabel("Created").fill("2026-10-14T10:20");
    const saved = await applyAndSave(page);

    const read = lamina(saved);
    expect([read.packet, read.created, read.disagreements]).toEqual(["parsed", "D:20261014102000+05'30'", []]);
    // Saved after `before`, to the second the date keeps.
    expect(pdfDate(read.modified ?? "")).toBeGreaterThanOrEqual(Math.floor(before / 1000) * 1000);
    const { info, xmp } = await pdfjs(saved);
    expect([info.CreationDate, xmp["xmp:createdate"]]).toEqual([
      "D:20261014102000+05'30'",
      "2026-10-14T10:20:00+05:30",
    ]);
    expect(pdfDate(String(info.ModDate))).toBe(Date.parse(String(xmp["xmp:modifydate"])));
    await clean(watched);
  });

  it("[cap J.10 web round-trip] removes the values chosen in Remove metadata… from Info and XMP", async () => {
    const watched = await opened("metadata.pdf");
    const { page } = watched;
    const dialog = await properties(page);
    await dialog.getByRole("button", { name: "Remove metadata…" }).click();
    const checklist = page.getByRole("dialog", { name: "Remove metadata" });
    await checklist.getByText("lamina-build tests").click();
    await checklist.getByText("Info and XMP carry the same values").click();
    await checklist.getByRole("button", { name: "Mark for removal" }).click();
    await dialog.getByText("Will be removed").first().waitFor();
    expect(await dialog.getByText("Will be removed").count()).toBe(2);
    // Saved incrementally, the old values stay in the file's earlier version: the notice says so.
    await dialog.getByText("Removed values stay in earlier versions.").waitFor();
    // Keep takes one back.
    await dialog.getByRole("button", { name: "Keep Subject" }).click();
    await dialog.getByRole("button", { name: "Remove metadata…" }).click();
    await checklist.getByText("Info and XMP carry the same values").click();
    await checklist.getByRole("button", { name: "Mark for removal" }).click();
    const saved = await applyAndSave(page);

    const read = lamina(saved);
    expect([read.creator, read.subject, read.title, read.disagreements]).toEqual([
      undefined,
      undefined,
      "Metadata golden: Info & XMP <in sync>",
      [],
    ]);
    const { info, xmp } = await pdfjs(saved);
    expect([info.Creator, info.Subject, xmp["xmp:creatortool"], xmp["dc:description"]]).toEqual([
      undefined,
      undefined,
      undefined,
      undefined,
    ]);
    await clean(watched);
  });

  it("warns under the title when a PDF/UA file's title is emptied, and Cancel changes nothing", async () => {
    const watched = await opened("examples/pdf_ua_report.pdf");
    const { page } = watched;
    const dialog = await properties(page);
    await dialog.getByLabel("Title").fill("");
    await dialog.getByText("PDF/UA needs a title.", { exact: false }).waitFor();
    await dialog.getByRole("button", { name: "Cancel" }).click();
    await dialog.waitFor({ state: "detached" });
    expect(await page.locator(".paper-top-bar-save-state").textContent()).not.toBe("1 unsaved change");
    await clean(watched);
  });

  it("passes axe on the dialog, the removal list and the capability page", async () => {
    const watched = await opened("metadata.pdf", { axe: true });
    const { page } = watched;
    const dialog = await properties(page);
    const problems = [...(await serious(page))];
    await dialog.getByRole("button", { name: "Remove metadata…" }).click();
    await page.getByRole("dialog", { name: "Remove metadata" }).getByRole("checkbox").first().waitFor();
    problems.push(...(await serious(page)));
    await page.keyboard.press("Escape");
    await page.keyboard.press("Escape");
    await dialog.waitFor({ state: "detached" });
    await more(page, "What Lamina PDF can do");
    await page.getByRole("heading", { name: "What Lamina PDF can read, write and round-trip" }).waitFor();
    problems.push(...(await serious(page)));
    expect(problems).toEqual([]);
    await page.close();
  });

  it("opens the capability page over the document, filters it, and goes back to the document", async () => {
    const watched = await opened("metadata.pdf");
    const { page } = watched;
    await more(page, "What Lamina PDF can do");
    await page.getByRole("heading", { name: "What Lamina PDF can read, write and round-trip" }).waitFor();
    expect(new URL(page.url()).pathname).toBe("/capabilities");
    const table = page.getByRole("table");
    const total = /^(\d+) of \1 operations in \d+ groups$/.exec((await table.locator("caption").textContent()) ?? "")?.[1];
    expect(total).toBeDefined();
    await page.getByLabel("Search operations").fill("J.10");
    await expect.poll(() => table.locator("caption").textContent()).toBe(`1 of ${total} operations in 1 group`);
    await page.getByLabel("Search operations").fill("no such operation");
    await page.getByRole("button", { name: "Clear filters" }).click();
    await page.getByRole("button", { name: "Back to metadata.pdf" }).click();
    await page
      .getByRole("heading", { name: "What Lamina PDF can read, write and round-trip" })
      .waitFor({ state: "detached" });
    expect(new URL(page.url()).pathname).toBe("/");
    // The document stayed open behind the page.
    expect(await page.locator(".web-viewer canvas").count()).toBeGreaterThan(0);
    await clean(watched);
  });

  it("opens the capability page from its address, without a document", async () => {
    const watched = await load(browser, origin, "/capabilities", NO_PICKERS, {
      ignoreHTTPSErrors: true,
      home: false,
    });
    const { page } = watched;
    await page.getByRole("heading", { name: "What Lamina PDF can read, write and round-trip" }).waitFor();
    await page.getByRole("button", { name: "Back to the reader" }).click();
    await page.getByRole("heading", { name: "Open a PDF" }).waitFor();
    expect(new URL(page.url()).pathname).toBe("/");
    await clean(watched);
  });
});

/** A PDF date's instant in milliseconds (`D:YYYYMMDDHHmmSS` with `Z` or `±HH'mm'`). */
function pdfDate(date: string): number {
  const m = /^D:(\d{4})(\d{2})(\d{2})(\d{2})(\d{2})(\d{2})(?:(Z)|([+-])(\d{2})'(\d{2})'?)?$/.exec(date);
  if (m === null) return Number.NaN;
  const [, y, mo, d, h, mi, s, , sign, oh, om] = m;
  const offset = sign === undefined ? 0 : (sign === "-" ? -1 : 1) * (Number(oh) * 60 + Number(om));
  return Date.UTC(Number(y), Number(mo) - 1, Number(d), Number(h), Number(mi), Number(s)) - offset * 60_000;
}
