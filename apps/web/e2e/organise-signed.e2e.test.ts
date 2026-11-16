// The choices on signed files in Organise (todo E2t, Paper O02, O03, O06), in this run's browser,
// with no file pickers, so every copy and save is a download. On a certified file a refused move
// offers Save a copy (here Download a copy), whose copy has the move and no certification while the
// open file stays as it was; Details says what the certification or the usage rights allow. On an
// approval-signed file, Remove signature and delete deletes the signature's page, and the next save
// asks once and removes the signature. Closing a document whose save would invalidate the certification
// offers the copy in place of Save, then closes. Each new surface passes axe.

import { readFileSync } from "node:fs";
import { Document, init } from "@laminapdf/sdk";
import type { Browser, Download, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, serious, type Watched } from "./reader.ts";

const origin = inject("origin");
let browser: Browser;
const CERTIFIED = new URL("../../../fixtures/signed/certified-p2-outline.pdf", import.meta.url);
const SIGNED_PAGE_2 = new URL("../../../fixtures/signed/approval-organised.pdf", import.meta.url);
const USAGE_RIGHTS = new URL("../../../fixtures/signed/usage-rights.pdf", import.meta.url);

/** No pickers: saves and copies download, in every engine. */
const NO_PICKERS = {
  script: () => {
    Reflect.deleteProperty(window, "showOpenFilePicker");
    Reflect.deleteProperty(window, "showSaveFilePicker");
  },
  arg: [],
};

/** Where a tile is pressed: off its centre, which React Aria takes for a screen reader's press. */
const OFF_CENTRE = { x: 20, y: 30 };

/** The grid's page IDs, in order. */
function order(page: Page): Promise<string[]> {
  return page
    .locator(".paper-page-tile")
    .evaluateAll((tiles) => tiles.map((t) => (t as HTMLElement).dataset.pageId ?? ""));
}

/** What the top bar says about saving. */
function saveState(page: Page): Promise<string | null> {
  return page.locator(".paper-top-bar-save-state").textContent();
}

/** The reader, wide, with `file` open in Organise; axe can run in it. */
async function organise(file: URL): Promise<Watched> {
  const watched = await load(browser, origin, "/", NO_PICKERS, {
    ignoreHTTPSErrors: true,
    viewport: { width: 1400, height: 900 },
    bypassCSP: true,
  });
  const { page } = watched;
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Choose a file" }).click();
  await (await chooser).setFiles(file.pathname);
  await page.locator(".web-viewer canvas").first().waitFor({ timeout: 30_000 });
  await page.getByRole("button", { name: "Organise", exact: true }).click();
  await page.locator(".paper-page-tile").first().waitFor();
  return watched;
}

/** Selects the tile at `index` alone. */
async function pick(page: Page, index: number): Promise<void> {
  await page.locator(".paper-page-tile").nth(index).click({ position: OFF_CENTRE });
}

/** Moves page 3 one place back with the keyboard, which a certification refuses: the banner says so. */
async function refusedMove(page: Page) {
  await pick(page, 2);
  await page.keyboard.press("ControlOrMeta+Shift+ArrowLeft");
  const banner = page.getByRole("alert").filter({ hasText: "would invalidate the certification" });
  await banner.waitFor();
  return banner;
}

/** The PDF a download wrote. */
async function opened(download: Promise<Download>): Promise<Document> {
  return Document.open(new Uint8Array(readFileSync(await (await download).path())));
}

describe(`signed files' choices in Organise in ${BROWSER}`, () => {
  beforeAll(async () => {
    await init({ extract: true });
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  it("downloads a copy with a refused move and without the certification, and leaves the open file as it was", async () => {
    const watched = await organise(CERTIFIED);
    const { page } = watched;
    const before = await order(page);
    const banner = await refusedMove(page);
    expect(await serious(page)).toEqual([]);

    await banner.getByRole("button", { name: "Download a copy" }).click();
    const ask = page.getByRole("alertdialog", { name: "Download a copy without the certification?" });
    await ask.getByText("The copy downloads with your changes").waitFor();
    expect(await serious(page)).toEqual([]);
    const download = page.waitForEvent("download");
    await ask.getByRole("button", { name: "Remove signature and download copy" }).click();
    using copy = await opened(download);
    expect({
      order: copy.pages.map((p) => p.id),
      signatures: copy.plan().signatures.length,
      bookmarks: copy.outline().length,
    }).toEqual({ order: [before[0], before[2], before[1]], signatures: 0, bookmarks: 3 });

    // The open file has no change: no step to undo, its pages in their order, and a copy of it
    // still certified.
    await expect.poll(() => banner.count()).toBe(0);
    expect(await saveState(page)).not.toMatch(/unsaved/);
    expect(await order(page)).toEqual(before);
    expect(await page.getByRole("button", { name: "Undo" }).isDisabled()).toBe(true);
    const original = page.waitForEvent("download");
    await page.getByRole("button", { name: "More", exact: true }).click();
    await page.getByRole("menuitem", { name: "Download a copy" }).click();
    using same = await opened(original);
    expect(same.plan().signatures.map((s) => s.kind)).toEqual(["certification"]);
    await clean(watched);
  });

  it("names the certification and what it allows in Details", async () => {
    const watched = await organise(CERTIFIED);
    const { page } = watched;
    const banner = await refusedMove(page);
    await banner.getByRole("button", { name: "Details" }).click();
    const details = page.getByRole("dialog", { name: "What the file's signatures allow" });
    await details.getByText("Certification", { exact: true }).waitFor();
    await details.getByText("Allows form filling and signing only.").waitFor();
    expect(await serious(page)).toEqual([]);
    await page.keyboard.press("Escape");
    await expect.poll(() => details.count()).toBe(0);
    await clean(watched);
  });

  it("names the usage rights and what they allow in Details", async () => {
    const watched = await organise(USAGE_RIGHTS);
    const { page } = watched;
    await pick(page, 0);
    await page.keyboard.press("r");
    const banner = page.getByRole("alert").filter({ hasText: "would invalidate the usage rights" });
    await banner.getByRole("button", { name: "Details" }).click();
    const details = page.getByRole("dialog", { name: "What the file's signatures allow" });
    await details.getByText("Usage rights", { exact: true }).waitFor();
    await details.getByText("Changing pages invalidates these rights.", { exact: false }).waitFor();
    expect(await serious(page)).toEqual([]);
    await clean(watched);
  });

  it("deletes a signed page with Remove signature and delete, and the next save asks once and removes the signature", async () => {
    const watched = await organise(SIGNED_PAGE_2);
    const { page } = watched;
    const before = await order(page);
    await pick(page, 1);
    await page.keyboard.press("Delete");
    const question = page.getByRole("alertdialog", { name: "Page 2 holds a signature" });
    await question.waitFor();
    expect(await serious(page)).toEqual([]);
    await question.getByRole("button", { name: "Remove signature and delete" }).click();
    await expect.poll(() => saveState(page)).toBe("1 unsaved change");
    await expect.poll(() => order(page)).toEqual([before[0], before[2]]);
    await page.getByText("Saving will invalidate the signature.").waitFor();

    const download = page.waitForEvent("download");
    await page.getByRole("button", { name: "Download", exact: true }).click();
    const ask = page.getByRole("alertdialog", { name: "Remove the signature and save?" });
    await ask.getByRole("button", { name: "Remove signature and save" }).click();
    // Asked once: a second question would hold the save, which ends only once it is written.
    await expect.poll(() => saveState(page)).toBe("No unsaved changes");
    await expect.poll(() => page.getByRole("alertdialog").count()).toBe(0);
    using saved = await opened(download);
    expect({ order: saved.pages.map((p) => p.id), signatures: saved.plan().signatures.length }).toEqual({
      order: [before[0], before[2]],
      signatures: 0,
    });
    await clean(watched);
  });

  it("closing a document whose save would invalidate the certification downloads a copy, then closes", async () => {
    const watched = await organise(CERTIFIED);
    const { page } = watched;
    const before = await order(page);
    const banner = await refusedMove(page);
    await banner.getByRole("button", { name: "Apply anyway…" }).click();
    await expect.poll(() => saveState(page)).toBe("1 unsaved change");
    // The strip that says saving would invalidate the certification offers the copy too.
    const strip = page.getByRole("region", { name: "How this file saves" });
    await strip.getByRole("button", { name: "Download a copy" }).waitFor();

    await page.getByRole("button", { name: "Close certified-p2-outline.pdf" }).click();
    const close = page.getByRole("alertdialog", { name: "Save changes to certified-p2-outline.pdf?" });
    await close.getByText("Saving this file would invalidate its certification.", { exact: false }).waitFor();
    expect(await close.getByRole("button", { name: "Save", exact: true }).count()).toBe(0);
    expect(await serious(page)).toEqual([]);
    await close.getByRole("button", { name: "Download a copy" }).click();
    const ask = page.getByRole("alertdialog", { name: "Download a copy without the certification?" });
    const download = page.waitForEvent("download");
    await ask.getByRole("button", { name: "Remove signature and download copy" }).click();
    using copy = await opened(download);
    expect({ order: copy.pages.map((p) => p.id), signatures: copy.plan().signatures.length }).toEqual({
      order: [before[0], before[2], before[1]],
      signatures: 0,
    });
    await page.getByRole("heading", { name: "Open a PDF" }).waitFor();
    // The file itself was never written: it is as signed as it was.
    using file = Document.open(new Uint8Array(readFileSync(CERTIFIED)));
    expect(file.plan().signatures.map((s) => s.kind)).toEqual(["certification"]);
    await clean(watched);
  });
});
