// Files that change on disk while open (S04): another app writing the file in place, an editor's
// atomic save (a new file renamed over it), and the file deleted. The scenario changes the file
// from the test process, as another app would, and waits for the window to follow; it reports how
// long the first reload took.
import { copyFileSync, mkdirSync, rmSync, unlinkSync } from "node:fs";
import { join } from "node:path";
import { $, browser, expect } from "@wdio/globals";
import { atomicReplace, fixture, goToPage, openAndWait, view, withText, zoomOnce } from "../app.ts";
import { workingCopies } from "../paths.ts";

const dir = join(workingCopies, "watched");
const watched = join(dir, "watched.pdf");
const example = (name: string) => fixture("fixtures", "golden", "examples", name);

/** Waits until the view shows a document of `pages` pages. */
async function waitForPageCount(pages: number): Promise<void> {
  await browser.waitUntil(async () => (await view()).pageCount === pages, {
    timeout: 15_000,
    timeoutMsg: `the window didn't reload to a ${pages}-page document`,
  });
}

describe("a file that changes on disk", () => {
  before(() => {
    rmSync(dir, { recursive: true, force: true });
    mkdirSync(dir, { recursive: true });
    copyFileSync(example("links_and_outlines.pdf"), watched);
  });

  it("should reload when written in place, at the same page and zoom", async () => {
    await openAndWait(watched);
    await zoomOnce("Zoom in");
    await goToPage(3);
    await browser.waitUntil(async () => (await view()).currentPage === 2, { timeoutMsg: "not on page 3" });
    const { percent } = await view();
    const written = Date.now();
    copyFileSync(example("flowing_report.pdf"), watched);
    await waitForPageCount(13);
    // The 1 s target (desktop plan §4) is reported, never asserted (ADR 0018).
    console.log(`File written in place: reloaded within ${Date.now() - written} ms`);
    expect([(await view()).currentPage, (await view()).percent]).toEqual([2, percent]);
    await expect(withText("Reloaded: watched.pdf changed on disk")).toExist();
  });

  it("should reload an atomic save and never say the file is missing", async () => {
    const temp = join(dir, ".watched.pdf.tmp");
    copyFileSync(example("markdown_to_pdf.pdf"), temp);
    await atomicReplace(temp, watched);
    await waitForPageCount(4);
    expect((await view()).currentPage).toBe(2);
    await expect(withText("File was moved or deleted")).not.toExist();
  });

  it("should say when the file is deleted, and keep showing it", async () => {
    unlinkSync(watched);
    await withText("File was moved or deleted").waitForExist({ timeout: 15_000 });
    expect((await view()).pageCount).toBe(4);
    // Close goes back to the start window, where the recent scenario carries on.
    await $("button=Close").click();
    await $("button=Choose a file").waitForExist();
  });
});
