// Find in document in the built reader (todo C5b), in this run's browser, over the
// links-and-outlines reference PDF, whose four pages say "chapter" four times (twice as "Chapter"):
// Ctrl+F or ⌘F opens the find bar with the focus in its field; typing shows the first highlight
// and the count; Enter and Shift+Enter move between matches across pages; Match case and Whole
// words change the count; the rail's Results tab lists the matches and goes to one; Escape closes the bar, clears the highlights and gives the focus back.
// No CSP violation or other origin throughout.

import { readFileSync } from "node:fs";
import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load as loadPage, type Watched, WITHOUT_PICKER } from "./reader.ts";
import { type RemoteHost, startRemoteHost } from "./remote-host.ts";

const origin = inject("origin");
const FILE = new URL("../../../fixtures/golden/links-and-outlines.pdf", import.meta.url);
const ENCRYPTED = new URL("../../../fixtures/text/encrypted/text-shaped-aes-256.pdf", import.meta.url);

/** The count inside the find field ("2 of 4", "No matches"). */
const count = (page: Page) => page.locator(".paper-find-count").textContent();

describe(`find in ${BROWSER}`, () => {
  let browser: Browser;
  let host: RemoteHost;

  beforeAll(async () => {
    browser = await launch();
    host = await startRemoteHost();
    host.add("links-and-outlines.pdf", readFileSync(FILE));
  });

  afterAll(async () => {
    await browser?.close();
    await host?.close();
  });

  /** Opens find with `query` typed, in a reader that already shows a document. */
  async function typeQuery(page: Page, query: string): Promise<void> {
    await page.locator(".web-viewer canvas").first().waitFor({ timeout: 30_000 });
    await page.keyboard.press("ControlOrMeta+F");
    await page.getByRole("textbox", { name: "Find in document" }).fill(query);
  }

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
    await expect.poll(() => page.getByText("Searched all 4 pages").isVisible()).toBe(true);
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
    // The page box shows page labels: pages 3 and 4 of this file are "1" and "2".
    await expect.poll(shown).toBe("1");
    await page.keyboard.press("Enter");
    await expect.poll(shown).toBe("2");
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

  it("lists every match in the rail's Results tab, goes to one on a click, and drops the tab on close", async () => {
    const watched = await finding("chapter");
    const { page } = watched;
    await expect.poll(() => count(page), { timeout: 15_000 }).toBe("1 of 4");
    const results = page.getByRole("tab", { name: "Results" });
    // The rail starts open on a wide window; open it if it isn't.
    if ((await results.count()) === 0) await page.getByRole("button", { name: "Sidebar" }).click();
    await results.click();
    const links = page.locator(".paper-find-result");
    await expect.poll(() => links.count()).toBe(4);
    const headings = await page.locator(".paper-find-results-heading").allTextContents();
    expect(headings.length).toBeGreaterThan(1);
    expect(headings.every((h) => h.startsWith("Page "))).toBe(true);
    expect(await page.locator('.paper-find-result[aria-current="true"]').count()).toBe(1);
    // The last match is on the fourth page, labelled "2".
    await links.nth(3).click();
    await expect.poll(() => count(page)).toBe("4 of 4");
    await expect.poll(() => page.getByRole("textbox", { name: "Page" }).inputValue()).toBe("2");
    expect(await links.nth(3).getAttribute("aria-current")).toBe("true");
    await page.getByRole("button", { name: "Close find" }).click();
    await expect.poll(() => results.count()).toBe(0);
    expect(await page.getByRole("tab", { name: "Pages", selected: true }).count()).toBe(1);
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

  it("finds text in an encrypted file once its password is typed", async () => {
    const watched = await loadPage(browser, origin, "/", WITHOUT_PICKER, { ignoreHTTPSErrors: true });
    const { page } = watched;
    const chooser = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Choose a file" }).click();
    await (await chooser).setFiles(ENCRYPTED.pathname);
    const dialog = page.getByRole("dialog", { name: "Enter the password" });
    await dialog.getByLabel("Password", { exact: true }).fill("user");
    await dialog.getByRole("button", { name: "Open" }).click();
    // pdf.js counts "office" twice on this page (fixtures/text/search.json).
    await typeQuery(page, "office");
    await expect.poll(() => count(page), { timeout: 15_000 }).toBe("1 of 2");
    await clean(watched);
  });

  it("finds text in a remote file, read by range requests", async () => {
    const url = `${host.origin}/cors/ranges/links-and-outlines.pdf`;
    const watched = await loadPage(browser, origin, `/#url=${encodeURIComponent(url)}`, undefined, {
      ignoreHTTPSErrors: true,
      home: false,
    });
    const { page } = watched;
    await typeQuery(page, "chapter");
    await expect.poll(() => count(page), { timeout: 15_000 }).toBe("1 of 4");
    // The page asked nothing of any origin but its own and the file's host.
    expect(await watched.violations()).toEqual([]);
    expect(watched.foreign.filter((u) => new URL(u).origin !== host.origin)).toEqual([]);
    await page.close();
  });
});
