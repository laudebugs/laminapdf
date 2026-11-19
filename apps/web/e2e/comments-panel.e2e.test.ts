// The Comments panel (Paper C04) on fixtures/annotations/review.pdf, in this run's browser: its
// threads by page with the broken-thread notice; a reply and a status on the first thread, a
// filter and a search; three threads ticked and deleted after the question; then a save, read
// back by pdf.js with the reply and the review state kept. axe finds nothing serious in the panel.

import { readFileSync } from "node:fs";
import { getDocument } from "pdfjs-dist/legacy/build/pdf.mjs";
import type { Browser, Download, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, serious, type Watched } from "./reader.ts";

const origin = inject("origin");
let browser: Browser;
const REVIEW = new URL("../../../fixtures/annotations/review.pdf", import.meta.url);

/** The reader, wide, with review.pdf open in Comment mode and a name for comments chosen. */
async function open(): Promise<Watched> {
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
  await (await chooser).setFiles(REVIEW.pathname);
  await page.locator(".web-viewer canvas").first().waitFor({ timeout: 30_000 });
  await page.getByRole("button", { name: "Comment", exact: true }).click();
  return watched;
}

const panel = (page: Page) => page.getByRole("complementary", { name: "Comments" });
const threads = (page: Page) => panel(page).locator(".paper-comment-thread");

/** The thread whose note is `note`. */
const thread = (page: Page, note: string) =>
  threads(page).filter({ has: page.locator(".paper-comment-thread-note", { hasText: note }) });

/** Saves (a download): every page's annotations as pdf.js reads them. */
async function saved(page: Page) {
  const download: Promise<Download> = page.waitForEvent("download");
  await page.getByRole("button", { name: "Download", exact: true }).click();
  const bytes = new Uint8Array(readFileSync(await (await download).path()));
  const task = getDocument({ data: bytes, verbosity: 0 });
  try {
    const doc = await task.promise;
    const all: { contentsObj?: { str: string }; inReplyTo?: string; state?: string; stateModel?: string }[] = [];
    for (let p = 1; p <= doc.numPages; p++) all.push(...((await (await doc.getPage(p)).getAnnotations()) as never[]));
    return all;
  } finally {
    await task.destroy();
  }
}

describe(`Comments panel in ${BROWSER}`, () => {
  beforeAll(async () => {
    browser = await launch();
  });
  afterAll(async () => {
    await browser?.close();
  });

  it("[cap D.10 web round-trip] [cap D.11 web round-trip] replies, sets a status, filters, searches and deletes three, and the file keeps them", async () => {
    const watched = await open();
    const { page } = watched;
    await page.getByRole("button", { name: "Comments", exact: true }).click();
    await expect.poll(() => threads(page).count()).toBe(6);
    // review.pdf's orphan and its reply cycle show on their own, under the notice.
    expect(await panel(page).getByText("Some replies point to missing comments").isVisible()).toBe(true);
    expect(await serious(page)).toEqual([]);

    // A reply and a status on "Second": its header selects it, and its thread opens.
    await thread(page, "Second").locator(".paper-comment-thread-go").click();
    const second = thread(page, "Second");
    await second.getByRole("textbox").fill("Agreed, moved it");
    await second.getByRole("button", { name: "Reply", exact: true }).click();
    await expect.poll(() => thread(page, "Second").locator(".paper-comment-reply").count()).toBe(1);
    await thread(page, "Second").getByLabel("Status").selectOption("rejected");
    await expect.poll(() => thread(page, "Second").getByText("Rejected", { exact: true }).count()).toBe(1);

    // Filter by status, then search.
    await panel(page)
      .getByRole("button", { name: /^Filter/ })
      .click();
    await page.getByRole("menuitemcheckbox", { name: /^Rejected/ }).click();
    await page.keyboard.press("Escape");
    await expect.poll(() => threads(page).count()).toBe(1);
    await panel(page).getByRole("button", { name: "Clear filters" }).first().click();
    await panel(page).getByRole("textbox", { name: "Search comments" }).fill("struck");
    await expect.poll(() => threads(page).count()).toBe(1);
    await panel(page).getByRole("textbox", { name: "Search comments" }).fill("");

    // Three ticked and deleted, after the question.
    for (const note of ["Orphan", "Ring A", "Struck"]) {
      await thread(page, note).locator(".paper-comment-thread-tick").click();
    }
    await panel(page).getByRole("button", { name: "Delete 3…" }).click();
    await page.getByRole("alertdialog").getByRole("button", { name: "Delete 3" }).click();
    await expect.poll(() => threads(page).count()).toBe(3);
    expect(await serious(page)).toEqual([]);

    const annotations = await saved(page);
    const texts = annotations.map((a) => a.contentsObj?.str ?? "");
    expect([
      texts.includes("Agreed, moved it"),
      annotations.some((a) => a.stateModel === "Review" && a.state === "Rejected"),
      texts.includes("Orphan"),
    ]).toEqual([true, true, false]);
    await clean(watched);
  });
});
