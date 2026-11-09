// Find in document in the built reader (todo C5b), in this run's browser, over the
// links-and-outlines reference PDF, whose four pages say "chapter" four times (twice as "Chapter"):
// Ctrl+F or ⌘F opens the find bar with the focus in its field; typing shows the first highlight
// and the count; Enter and Shift+Enter move between matches across pages; Match case and Whole
// words change the count; Escape closes the bar, clears the highlights and gives the focus back.
// No CSP violation or other origin throughout.

import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load as loadPage, type Watched, WITHOUT_PICKER } from "./reader.ts";

const origin = inject("origin");
const FILE = new URL("../../../fixtures/golden/links-and-outlines.pdf", import.meta.url);

/** The count inside the find field ("2 of 4", "No matches"). */
const count = (page: Page) => page.locator(".paper-find-count").textContent();

describe(`find in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  /** The reference PDF open in a wide window, find open with `query` typed. */
  async function finding(query: string): Promise<Watched> {
    const watched = await loadPage(browser, origin, "/", WITHOUT_PICKER, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1280, height: 800 },
    });
    const { page } = watched;
    const chooser = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Choose a file" }).click();
    await (await chooser).setFiles(FILE.pathname);
    await page.locator(".web-viewer canvas").first().waitFor();
    await page.keyboard.press("ControlOrMeta+F");
    const field = page.getByRole("textbox", { name: "Find in document" });
    await expect.poll(() => field.evaluate((el) => el === document.activeElement)).toBe(true);
    await field.fill(query);
    return watched;
  }

  it("shows the first highlight and the count as you type", async () => {
    const watched = await finding("chapter");
    const { page } = watched;
    await expect.poll(() => count(page), { timeout: 15_000 }).toBe("1 of 4");
    await expect.poll(() => page.locator(".lamina-find-current").count()).toBeGreaterThan(0);
    await expect(page.getByText("Searched all 4 pages")).toBeDefined();
    await clean(watched);
  });

  it("moves between matches across pages with Enter and Shift+Enter, scrolling each into view", async () => {
    const watched = await finding("chapter");
    const { page } = watched;
    await expect.poll(() => count(page), { timeout: 15_000 }).toBe("1 of 4");
    const shown = () => page.getByRole("textbox", { name: "Page" }).inputValue();
    await page.keyboard.press("Enter");
    await page.keyboard.press("Enter");
    await expect.poll(() => count(page)).toBe("3 of 4");
    await expect.poll(shown).toBe("3");
    await page.keyboard.press("Enter");
    await expect.poll(shown).toBe("4");
    await page.keyboard.press("Enter");
    await expect.poll(() => count(page)).toBe("1 of 4");
    await page.keyboard.press("Shift+Enter");
    await expect.poll(() => count(page)).toBe("4 of 4");
    await clean(watched);
  });

  it("counts again with Match case and with Whole words", async () => {
    const watched = await finding("Chapter");
    const { page } = watched;
    await expect.poll(() => count(page), { timeout: 15_000 }).toBe("1 of 4");
    await page.getByRole("button", { name: "Match case" }).click();
    await expect.poll(() => count(page), { timeout: 15_000 }).toMatch(/ of 2$/);
    await page.getByRole("button", { name: "Match case" }).click();
    const field = page.getByRole("textbox", { name: "Find in document" });
    await field.fill("chap");
    // The search starts from the page in view, which Match case moved to: only the total is fixed.
    await expect.poll(() => count(page), { timeout: 15_000 }).toMatch(/ of 4$/);
    await page.getByRole("button", { name: "Whole words" }).click();
    await expect.poll(() => count(page), { timeout: 15_000 }).toBe("No matches");
    await clean(watched);
  });

  it("closes on Escape, clears the highlights and gives the focus back", async () => {
    const watched = await finding("chapter");
    const { page } = watched;
    await expect.poll(() => count(page), { timeout: 15_000 }).toBe("1 of 4");
    await page.keyboard.press("Escape");
    await expect.poll(() => page.getByRole("search").count()).toBe(0);
    expect(await page.locator(".lamina-find-match").count()).toBe(0);
    expect(await page.evaluate(() => document.activeElement?.closest("[role=search], search") ?? null)).toBeNull();
    await clean(watched);
  });
});
