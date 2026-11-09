// Reader mode in the built reader (todo U4b), in this run's browser: a tagged file reads in its
// structure tree's order, its figure, formula and table drawn as crops named by their alternate
// text and never reflowed as text; leaving comes back to the page region the column was at
// (within one block); an untagged file reads approximately (todo U4c), labelled so, each block's
// "View original page" leaving at its place, and a page whose column order is doubted shown as its
// image; the settings change the column, which at its largest text and a narrow window never
// scrolls sideways; Escape leaves; and the column passes axe. No CSP violation or other origin
// throughout.

import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { chooseFile, clean, load, opened, type Watched, WITHOUT_PICKER } from "./reader.ts";

const origin = inject("origin");

/** The reader-mode corpus, beside the test corpus `chooseFile` reads. */
const REFLOW = "../reflow/";

/** The reading column. */
const column = (page: Page) => page.getByRole("region", { name: "Reader mode" });

describe(`reader mode in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  /**
   * `file` open in a window `width` wide, named `title` (its metadata title, else its file name);
   * `bypassCSP` lets axe in.
   */
  async function open(file: string, width = 1280, bypassCSP = false, title?: string): Promise<Watched> {
    const watched = await load(browser, origin, "/", WITHOUT_PICKER, {
      ignoreHTTPSErrors: true,
      viewport: { width, height: 800 },
      bypassCSP,
    });
    await chooseFile(watched, file);
    await opened(watched, title ?? file.split("/").at(-1) ?? file);
    return watched;
  }

  /** Turns reader mode on with the top bar's toggle. */
  async function enter(page: Page): Promise<void> {
    await page.getByRole("button", { name: "Reader mode" }).first().click();
    await column(page).waitFor();
  }

  it("reads a tagged file in its tree's order, with crops named by their alternate text", async () => {
    const watched = await open(`${REFLOW}mixed.pdf`);
    const { page } = watched;
    await enter(page);
    const view = column(page);
    await view.getByRole("heading", { level: 2 }).first().waitFor();
    for (const alt of ["Four coloured squares in a grid", "E equals m c squared", "Two rows of two numbers"]) {
      await expect.poll(() => view.getByRole("img", { name: alt }).count()).toBe(1);
    }
    // The column's text is the tree's, drawn in reverse on the page; the crops' content isn't in it.
    const text = (await view.locator("article").textContent()) ?? "";
    expect(text.indexOf("Hollow orchard")).toBeLessThan(text.indexOf("Stone meadow"));
    expect(text).not.toContain("mc2");
    expect(await view.getByRole("listitem").count()).toBe(2);
    // The figure's crop is drawn.
    await expect
      .poll(() =>
        view
          .locator("figure canvas")
          .first()
          .evaluate((c) => (c as HTMLCanvasElement).width),
      )
      .toBeGreaterThan(0);
    await clean(watched);
  });

  it("comes back to the page region the column was at, within one block", async () => {
    const watched = await open("../golden/tagged-report.pdf", 1280, false, "Quarterly report");
    const { page } = watched;
    const box = page.getByRole("textbox", { name: "Page" });
    await box.fill("2");
    await box.press("Enter");
    await expect.poll(() => box.inputValue()).toBe("2");
    await enter(page);
    const back = page.getByRole("button", { name: "Back to page 2" });
    await back.waitFor();
    // The column starts at the first block on page 2, the "Notes" heading, at its top.
    const notes = column(page).getByRole("heading", { name: "Notes" });
    const columnTop = (await column(page).boundingBox())?.y ?? 0;
    await expect
      .poll(async () => ((await notes.boundingBox())?.y ?? Number.POSITIVE_INFINITY) - columnTop)
      .toBeLessThan(80);
    await back.click();
    await expect.poll(() => box.inputValue()).toBe("2");
    // Back on the page, that block's text is at the view's top, within one block's height.
    const viewer = await page.locator(".web-viewer").boundingBox();
    const span = page.locator(".web-viewer").getByText("Notes", { exact: true }).first();
    await expect
      .poll(async () => ((await span.boundingBox())?.y ?? Number.POSITIVE_INFINITY) - (viewer?.y ?? 0))
      .toBeLessThan(80);
    await clean(watched);
  });

  it("reads an untagged file approximately, says so, and leaves at a block's place on its page", async () => {
    const watched = await open("../reading-order/two-columns-reversed.pdf");
    const { page } = watched;
    await enter(page);
    const view = column(page);
    await view.getByText("Approximate", { exact: true }).waitFor();
    await expect.poll(() => view.getByRole("status").textContent()).toContain("This file has no tags");
    // The second column's first paragraph: drawn first, read second.
    const blocks = view.locator("[data-block]");
    await expect.poll(() => blocks.count()).toBeGreaterThan(2);
    const block = blocks.nth(2);
    await block.hover();
    await block.getByRole("button", { name: /View original page \(page 1\)/ }).click();
    await expect.poll(() => column(page).count()).toBe(0);
    // Back on the page, the block's box is outlined for a moment.
    await page.locator(".web-reader-origin").waitFor({ state: "attached" });
    await clean(watched);
  });

  it("shows a page whose column order extraction doubts as its image, with the way to open it", async () => {
    const watched = await open("../reading-order/narrow-gutter-ordered.pdf");
    const { page } = watched;
    await enter(page);
    const view = column(page);
    await view.getByRole("img", { name: "Page 1" }).waitFor();
    await view.getByText("couldn't tell the order of this page's text").waitFor();
    await view.getByRole("button", { name: "Open page 1" }).click();
    await page.locator(".web-viewer canvas").first().waitFor({ state: "visible" });
    await expect.poll(() => column(page).count()).toBe(0);
    await clean(watched);
  });

  it("leaves on Escape", async () => {
    const watched = await open(`${REFLOW}two-columns-tagged.pdf`);
    const { page } = watched;
    await enter(page);
    await column(page).getByRole("heading").first().waitFor();
    // The column takes the focus on entering, so Escape reaches it straight away.
    await page.keyboard.press("Escape");
    await expect.poll(() => column(page).count()).toBe(0);
    await clean(watched);
  });

  it("scales its text to the largest size in a narrow window without scrolling sideways, and passes axe", async () => {
    const watched = await open(`${REFLOW}mixed.pdf`, 640, true);
    const { page } = watched;
    await enter(page);
    await column(page).getByRole("heading").first().waitFor();
    await page.getByRole("button", { name: "Reader settings" }).click();
    const larger = page.getByRole("button", { name: "Larger text" });
    while (await larger.isEnabled()) await larger.click();
    await page.getByText("32 px").waitFor();
    const article = column(page).locator("article");
    expect(await article.evaluate((a) => getComputedStyle(a).fontSize)).toBe("32px");
    const overflow = await column(page).evaluate((c) => c.scrollWidth - c.clientWidth);
    expect(overflow).toBeLessThanOrEqual(0);
    const { default: axe } = await import("axe-core");
    await page.addScriptTag({ content: axe.source });
    const serious = await page.evaluate(async () => {
      const run = (
        window as unknown as { axe: { run: (n: Node) => Promise<{ violations: { id: string; impact: string }[] }> } }
      ).axe;
      const { violations } = await run.run(document.body);
      return violations.filter((v) => v.impact === "serious" || v.impact === "critical").map((v) => v.id);
    });
    expect(serious).toEqual([]);
    await watched.page.close();
  });
});
