// The reader with no mouse at all (todo C7a), in this run's browser: open a file from the home
// screen, skip to the document, page through it, read the shortcut help, find, jump to a page and
// back, open the document's properties and the print dialog, each from the keyboard; the live
// region says the page.

import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { load, WITHOUT_PICKER } from "./reader.ts";

const origin = inject("origin");
const FILE = new URL("../../../fixtures/corpus/outline.pdf", import.meta.url).pathname;

const shownPage = (page: Page) => page.getByRole("textbox", { name: "Page" }).inputValue();
/** The focused element's accessible name, or its text. */
const focused = (page: Page) =>
  page.evaluate(() => {
    const el = document.activeElement as HTMLElement | null;
    return el?.getAttribute("aria-label") ?? el?.textContent?.trim() ?? "";
  });
/** What the polite live region says now. */
const said = (page: Page) =>
  page.evaluate(() => document.querySelector('[aria-live="polite"][aria-atomic="true"]')?.textContent ?? "");

/** Presses Tab (or Shift+Tab, `back`) until the focus is on `name` (at most 40 times). */
async function tabTo(page: Page, name: string, back = false): Promise<void> {
  for (let i = 0; i < 40; i++) {
    await page.keyboard.press(back ? "Shift+Tab" : "Tab");
    if ((await focused(page)) === name) return;
  }
  throw new Error(`Tab never reached ${name}`);
}

/** Runs the palette command `title`, typed, from the keyboard; `then` types its argument. */
async function command(page: Page, title: string, argument?: string): Promise<void> {
  await page.keyboard.press("ControlOrMeta+K");
  await page.keyboard.type(title);
  await page.keyboard.press("Enter");
  if (argument !== undefined) {
    await page.keyboard.type(argument);
    await page.keyboard.press("Enter");
  }
}

describe(`the reader from the keyboard in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  it("opens, pages, finds, jumps and comes back, and opens properties and print", async () => {
    const { page } = await load(browser, origin, "/", WITHOUT_PICKER, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1280, height: 900 },
    });
    // Open: Tab to Choose a file, then Enter.
    await tabTo(page, "Choose a file");
    const chooser = page.waitForEvent("filechooser");
    await page.keyboard.press("Enter");
    await (await chooser).setFiles(FILE);
    await page.locator(".web-viewer canvas").first().waitFor({ timeout: 30_000 });
    await expect.poll(() => said(page)).toMatch(/^Opened outline\.pdf, \d+ pages/);
    // Skip to the document: back up to the skip link, the page's first stop (Firefox's Tab leaves
    // the page for its own toolbar after the last stop, so Tab never comes round).
    await tabTo(page, "Skip to document", true);
    await page.keyboard.press("Enter");
    await expect.poll(() => page.evaluate(() => document.activeElement?.className)).toBe("lamina-viewer");
    // Page through it; the page is said once scrolling settles.
    await page.keyboard.press("PageDown");
    // The file labels its pages i, A-1, A-2…
    await expect.poll(() => shownPage(page)).toBe("A-1");
    await expect.poll(() => said(page)).toMatch(/^Page A-1, 2 of \d+/);
    // The shortcut help, and back.
    await page.keyboard.press("?");
    const help = page.getByRole("dialog", { name: "Keyboard shortcuts" });
    await help.waitFor();
    await page.keyboard.press("Escape");
    await expect.poll(() => help.count()).toBe(0);
    // Find.
    await page.keyboard.press("ControlOrMeta+F");
    await page.keyboard.type("e");
    await page.keyboard.press("Enter");
    await expect.poll(() => page.getByRole("search").textContent()).toMatch(/\d+ of \d+/);
    await page.keyboard.press("Escape");
    // A jump lands focus on its page; Back returns.
    await command(page, "Go to page", "3");
    await expect.poll(() => shownPage(page)).toBe("A-2");
    await expect.poll(() => focused(page)).toMatch(/^Page A-2, 3 of \d+/);
    await command(page, "Back");
    await expect.poll(() => shownPage(page)).toBe("A-1");
    // Properties and print.
    await command(page, "Document properties");
    const properties = page.getByRole("dialog", { name: "Document properties" });
    await properties.waitFor();
    await page.keyboard.press("Escape");
    await expect.poll(() => properties.count()).toBe(0);
    await page.keyboard.press("ControlOrMeta+P");
    const print = page.getByRole("dialog", { name: "Print" });
    await print.waitFor();
    await page.keyboard.press("Escape");
    await expect.poll(() => print.count()).toBe(0);
    await page.close();
  });
});
