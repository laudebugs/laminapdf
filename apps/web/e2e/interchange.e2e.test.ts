// Import, export, flatten and the comment summary (Paper C06, C07) in this run's browser, on
// fixtures/annotations/review.pdf: three comments deleted and brought back from an exported
// XFDF, which the saved file keeps; every comment flattened and the saved file read back; the
// summary opened in a new tab with the document unchanged; and Flatten on a file certified at P3,
// which offers only a copy. axe finds nothing serious in the dialogs.

import { readFileSync } from "node:fs";
import { env } from "node:process";
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";
import type { Browser, Download, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, serious, type Watched } from "./reader.ts";

const origin = inject("origin");
let browser: Browser;
const REVIEW = new URL("../../../fixtures/annotations/review.pdf", import.meta.url);
const CERTIFIED_P3 = new URL("../../../fixtures/signed/certified-p3.pdf", import.meta.url);

/** The reader with `file` open in Comment mode, a name for comments chosen and no pickers, so files download. */
async function open(file: URL): Promise<Watched> {
  const watched = await load(
    browser,
    origin,
    "/",
    {
      script: () => {
        Reflect.deleteProperty(window, "showOpenFilePicker");
        Reflect.deleteProperty(window, "showSaveFilePicker");
        localStorage.setItem("lamina.comment.author", "Ada");
      },
    } as never,
    { ignoreHTTPSErrors: true, viewport: { width: 1400, height: 900 }, bypassCSP: true },
  );
  const { page } = watched;
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Choose a file" }).click();
  await (await chooser).setFiles(file.pathname);
  await page.locator(".web-viewer canvas").first().waitFor({ timeout: 30_000 });
  await page.getByRole("button", { name: "Comment", exact: true }).click({ timeout: 30_000 });
  return watched;
}

const panel = (page: Page) => page.getByRole("complementary", { name: "Comments" });
const threads = (page: Page) => panel(page).locator(".paper-comment-thread");
const thread = (page: Page, note: string) =>
  threads(page).filter({ has: page.locator(".paper-comment-thread-note", { hasText: note }) });

/** Where screenshots of the dialogs go, when asked for (LAMINA_PDF_E2E_SHOTS=<dir>). */
const SHOTS = env.LAMINA_PDF_E2E_SHOTS;

/** A screenshot of the page, when screenshots were asked for. */
async function shot(page: Page, name: string): Promise<void> {
  if (SHOTS === undefined) return;
  // After the dialog's entrance and the menu's exit have finished.
  await page.waitForTimeout(600);
  await page.screenshot({ path: `${SHOTS}/${BROWSER}-${name}.png` });
}

/** Opens the Comments panel's More menu item `name`. */
async function more(page: Page, name: string): Promise<void> {
  await page.getByRole("button", { name: "Comments", exact: true }).click();
  await panel(page).getByRole("button", { name: "More comment actions" }).click();
  await page.getByRole("menuitem", { name, exact: true }).click();
}

/** Saves (a download) and reads the annotations of every page as pdf.js sees them. */
async function saved(page: Page) {
  const download: Promise<Download> = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download", exact: true }).click();
  const bytes = new Uint8Array(readFileSync(await (await download).path()));
  const task = getDocument({ data: bytes, verbosity: 0 });
  try {
    const doc = await task.promise;
    const all: { subtype: string; contentsObj?: { str: string } }[] = [];
    for (let p = 1; p <= doc.numPages; p++) all.push(...((await (await doc.getPage(p)).getAnnotations()) as never[]));
    return all;
  } finally {
    await task.destroy();
  }
}

describe(`Import, export, flatten and the summary in ${BROWSER}`, () => {
  beforeAll(async () => {
    browser = await launch();
  });
  afterAll(async () => {
    await browser?.close();
  });

  it("[cap D.13 web round-trip] exports the comments as XFDF, and importing it again brings back two deleted ones", async () => {
    const watched = await open(REVIEW);
    const { page } = watched;
    await more(page, "Export comments…");
    const dialog = page.getByRole("dialog", { name: "Export comments" });
    await dialog.waitFor();
    await shot(page, "export");
    expect(await serious(page)).toEqual([]);
    const downloading: Promise<Download> = page.waitForEvent("download");
    await dialog.getByRole("button", { name: "Save as…" }).click();
    const exported = await downloading;
    expect(exported.suggestedFilename()).toBe("review.xfdf");
    const xfdf = readFileSync(await exported.path(), "utf8");
    expect(xfdf).toContain("<xfdf");
    expect(xfdf).toContain("Check this");
    await expect.poll(() => threads(page).count()).toBe(6);

    // Two comments deleted, then the file imported: they are the two it brings back.
    for (const note of ["Second", "Struck"]) {
      await thread(page, note).locator(".paper-comment-thread-tick").click();
    }
    await panel(page).getByRole("button", { name: "Delete 2…" }).click();
    await page.getByRole("alertdialog").getByRole("button", { name: "Delete 2" }).click();
    await expect.poll(() => threads(page).count()).toBe(4);

    const chooser = page.waitForEvent("filechooser");
    await panel(page).getByRole("button", { name: "More comment actions" }).click();
    await page.getByRole("menuitem", { name: "Import comments…" }).click();
    await (await chooser).setFiles(await exported.path());
    const importing = page.getByRole("dialog", { name: "Import comments" });
    // The import counts replies and pop-ups with their comments, so it names more than two.
    await importing.getByText(/\d+ new comments will be added/).waitFor();
    await shot(page, "import");
    expect(await serious(page)).toEqual([]);
    await importing.getByRole("button", { name: /^Import \d+ comments$/ }).click();
    await expect.poll(() => threads(page).count()).toBe(6);

    const texts = (await saved(page)).map((a) => a.contentsObj?.str ?? "");
    expect(["Second", "Struck"].map((note) => texts.some((t) => t.includes(note)))).toEqual([true, true]);
    await clean(watched);
  });

  it("[cap D.14 web round-trip] flattens every comment into its page, and the saved file no longer has them as comments", async () => {
    const watched = await open(REVIEW);
    const { page } = watched;
    await more(page, "Flatten comments…");
    const dialog = page.getByRole("alertdialog", { name: "Flatten comments into the pages?" });
    await dialog.waitFor();
    await shot(page, "flatten");
    expect(await serious(page)).toEqual([]);
    await dialog.getByRole("button", { name: /^Flatten \d+ comments$/ }).click();
    await expect.poll(() => threads(page).count()).toBeLessThan(6);

    const marks = (await saved(page)).filter((a) => a.subtype !== "Popup" && a.subtype !== "Link");
    // What is left are the comments flattening can't draw (a sound), not the highlights and notes.
    expect(marks.map((a) => a.subtype)).not.toContain("Highlight");
    expect(marks.map((a) => a.subtype)).not.toContain("Text");
    await clean(watched);
  });

  it("[cap D.15 web round-trip] makes a summary in a new tab and leaves the document as it was", async () => {
    const watched = await open(REVIEW);
    const { page } = watched;
    await more(page, "Comment summary…");
    const dialog = page.getByRole("dialog", { name: "Comment summary" });
    await dialog.waitFor();
    await shot(page, "summary");
    expect(await serious(page)).toEqual([]);
    // The new tab saves by download too, as this one does.
    await page.context().addInitScript(() => Reflect.deleteProperty(window, "showSaveFilePicker"));
    const tab = page.context().waitForEvent("page");
    await dialog.getByRole("button", { name: "Create summary" }).click();
    const summary = await tab;
    await summary.locator(".web-viewer canvas").first().waitFor({ timeout: 60_000 });
    await dialog.getByText("The summary is open in a new tab.").waitFor();
    await dialog.getByRole("button", { name: "Close" }).first().click();

    // The new tab holds an unsaved document of its own: its file has the summary's words.
    const download: Promise<Download> = summary.waitForEvent("download");
    await summary.getByRole("button", { name: "More", exact: true }).click();
    await summary.getByRole("menuitem", { name: "Download a copy" }).click();
    const bytes = new Uint8Array(readFileSync(await (await download).path()));
    const task = getDocument({ data: bytes, verbosity: 0 });
    try {
      const doc = await task.promise;
      expect(doc.numPages).toBe(3);
      const text = (await (await doc.getPage(1)).getTextContent()).items
        .map((item) => ("str" in item ? item.str : ""))
        .join(" ");
      expect(text).toMatch(/Comments on|Comment summary/);
      expect(text).toContain("Check this");
    } finally {
      await task.destroy();
    }
    // The reviewed document still has its six comments and nothing unsaved.
    await expect.poll(() => threads(page).count()).toBe(6);
    await summary.close();
    await clean(watched);
  });

  it("[cap D.14 web round-trip] offers only a copy on a file certified at P3", async () => {
    const watched = await open(CERTIFIED_P3);
    const { page } = watched;
    await page.getByRole("button", { name: "Note", exact: true }).click();
    const box = await page.locator('.lamina-page[data-page-index="0"]').boundingBox();
    if (box === null) throw new Error("the page isn't drawn");
    await page.mouse.click(box.x + 100, box.y + 100);
    await more(page, "Flatten comments…");
    const dialog = page.getByRole("alertdialog", { name: "Flatten comments into the pages?" });
    await dialog.waitFor();
    await dialog.getByText("Flattening will invalidate the signature.").waitFor();
    await dialog.getByRole("button", { name: "Flatten in a copy…" }).waitFor();
    await shot(page, "flatten-certified");
    expect(await dialog.getByRole("button", { name: /^Flatten 1 comment$/ }).count()).toBe(0);
    expect(await serious(page)).toEqual([]);
    await clean(watched);
  });
});
