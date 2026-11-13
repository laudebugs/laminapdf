// The edit session in the built reader (todo E1d, Paper O01), in this run's browser: turning a
// page from the More menu, then undoing and redoing it with the keys, each step checked on the
// page itself (it lies on its side, then stands again) and in the dirty indicators (the top bar's
// save state and the tab's "•"); the History tab going back to the file as opened; closing with
// unsaved changes asks first, in the reader and in the browser, and closing a clean document
// doesn't; and the 500-step limit, which drops the oldest step and says so in the History tab.
// No CSP violation or other origin throughout.

import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, type Watched, WITHOUT_PICKER } from "./reader.ts";

const origin = inject("origin");
/** Four portrait pages, the first labelled "i". */
const FILE = new URL("../../../fixtures/golden/links-and-outlines.pdf", import.meta.url);
const NAME = "links-and-outlines.pdf";

/** The top bar's save state ("" before any edit). */
const saveState = (page: Page) => page.locator(".paper-top-bar-save-state").textContent();

/** Whether page 1, as drawn, is wider than it is tall: turned a quarter from portrait. */
async function firstPageTurned(page: Page): Promise<boolean> {
  const box = await page.locator('.lamina-page[data-page-index="0"]').boundingBox();
  if (box === null) throw new Error("page 1 isn't drawn");
  return box.width > box.height;
}

/** Whether page 1 is drawn sharp at its present size (its canvas fills its shell). */
async function firstPageDrawn(page: Page): Promise<void> {
  await expect
    .poll(async () => (await page.locator('.lamina-page[data-page-index="0"] canvas').count()) > 0, { timeout: 20_000 })
    .toBe(true);
}

/**
 * Whether closing the tab would make the browser ask: the reader cancels `beforeunload`. (The
 * prompt itself is the browser's; headless Chromium closes without showing it, so the test reads
 * what the page answers instead.)
 */
const asksBeforeUnload = (page: Page) =>
  page.evaluate(() => {
    const event = new Event("beforeunload", { cancelable: true });
    window.dispatchEvent(event);
    return event.defaultPrevented;
  });

/** Chooses the More menu's item `name`. */
async function more(page: Page, name: string): Promise<void> {
  await page.getByRole("button", { name: "More" }).click();
  await page.getByRole("menuitem", { name }).click();
}

describe(`the edit session in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  /** The reader with the four-page file open, in a wide window. */
  async function opened(): Promise<Watched> {
    const watched = await load(browser, origin, "/", WITHOUT_PICKER, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1280, height: 800 },
    });
    const { page } = watched;
    const chooser = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Choose a file" }).click();
    await (await chooser).setFiles(FILE.pathname);
    await page.locator(".web-viewer canvas").first().waitFor();
    await expect.poll(() => page.title()).toBe(`${NAME} · Lamina PDF`);
    return watched;
  }

  it("turns a page, undoes and redoes it, showing the page and the unsaved state at each step", async () => {
    const watched = await opened();
    const { page } = watched;
    expect([await firstPageTurned(page), await saveState(page)]).toEqual([false, ""]);
    await expect(page.getByRole("button", { name: "Undo", exact: true }).isDisabled()).resolves.toBe(true);

    await more(page, "Rotate page right");
    await expect.poll(() => firstPageTurned(page)).toBe(true);
    await firstPageDrawn(page);
    await expect.poll(() => saveState(page)).toBe("1 unsaved change");
    await expect.poll(() => page.title()).toBe(`• ${NAME} · Lamina PDF`);
    // Undo names the step it takes back.
    await page.getByRole("button", { name: "Undo rotate page i" }).waitFor();

    await page.locator(".lamina-viewer").focus();
    await page.keyboard.press("ControlOrMeta+Z");
    await expect.poll(() => firstPageTurned(page)).toBe(false);
    await expect.poll(() => saveState(page)).toBe("No unsaved changes");
    await expect.poll(() => page.title()).toBe(`${NAME} · Lamina PDF`);
    await page.getByText("Undid “Rotate page i”").waitFor();

    // Redo: ⇧⌘Z on macOS, Ctrl+Y elsewhere; Ctrl+Shift+Z works there too.
    await page.keyboard.press(process.platform === "darwin" ? "Meta+Shift+Z" : "Control+Y");
    await expect.poll(() => firstPageTurned(page)).toBe(true);
    await expect.poll(() => saveState(page)).toBe("1 unsaved change");
    await expect.poll(() => page.title()).toBe(`• ${NAME} · Lamina PDF`);

    // The History tab lists the opened file and the step; choosing the first row goes back to it.
    await more(page, "Show history");
    const history = page.getByRole("list", { name: "Edit history" });
    await expect.poll(() => history.getByRole("button").count()).toBe(2);
    await expect.poll(() => history.locator('[aria-current="step"]').textContent()).toContain("Rotate page i");
    await history.getByRole("button", { name: /Opened file/ }).click();
    await expect.poll(() => firstPageTurned(page)).toBe(false);
    await expect.poll(() => saveState(page)).toBe("No unsaved changes");
    await expect.poll(() => history.locator('[aria-current="step"]').textContent()).toContain("Opened file");
    // By the keys: down to the step, Enter goes to it.
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Enter");
    await expect.poll(() => firstPageTurned(page)).toBe(true);
    await clean(watched);
  });

  it("asks before closing a document with unsaved changes, and not once it is clean", async () => {
    const watched = await opened();
    const { page } = watched;
    await more(page, "Rotate page right");
    await expect.poll(() => saveState(page)).toBe("1 unsaved change");

    // In the reader: More › Close document would leave the document.
    await more(page, "Close document");
    const ask = page.getByRole("alertdialog", { name: `Save changes to ${NAME}?` });
    await ask.waitFor();
    await ask.getByRole("button", { name: "Cancel" }).click();
    await expect.poll(() => page.title()).toBe(`• ${NAME} · Lamina PDF`);

    // The browser's own prompt when the tab closes.
    expect(await asksBeforeUnload(page)).toBe(true);

    // Clean again, the document closes without asking.
    await page.locator(".lamina-viewer").focus();
    await page.keyboard.press("ControlOrMeta+Z");
    await expect.poll(() => saveState(page)).toBe("No unsaved changes");
    await more(page, "Close document");
    await page.getByRole("heading", { name: "Open a PDF" }).waitFor();
    expect(await page.getByRole("alertdialog").count()).toBe(0);
    expect(await asksBeforeUnload(page)).toBe(false);
    await clean(watched);
  });

  it("keeps 500 steps: the 501st edit drops the oldest, and the History tab says so", async () => {
    const watched = await opened();
    const { page } = watched;
    await more(page, "Rotate page right");
    // The command palette turns the page again each time.
    for (let i = 1; i < 501; i++) {
      await page.keyboard.press("ControlOrMeta+K");
      await page.keyboard.type("rotate page right");
      await page.keyboard.press("Enter");
      await expect.poll(() => saveState(page)).toBe(`${i + 1} unsaved changes`);
    }
    await more(page, "Show history");
    const tab = page.locator(".paper-history");
    await expect.poll(() => tab.locator(".paper-history-count").textContent()).toBe("500 steps");
    const first = tab.getByRole("button").first();
    await expect.poll(() => first.textContent()).toContain("Earlier steps no longer kept");
    // Back to the oldest step kept: the file as opened can't be reached, so a change stays unsaved.
    await first.click();
    await expect.poll(() => saveState(page), { timeout: 30_000 }).toBe("1 unsaved change");
    await clean(watched);
  }, 600_000);
});
