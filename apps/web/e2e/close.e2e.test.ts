// Closing a document (todos C8a, C8b; Reader A R19), in this run's browser. The × after the file
// name, More › Close document and the palette's Close document each go back to the home screen,
// with focus on its heading, "Closed {name}" said, the window title back to the app's name and the
// file first in Recent. A document with unsaved changes asks first on every route, the address
// included: taking `#url=` out of it shows the unsaved-changes dialog, Cancel keeps the document
// and puts the link back in the address, and Don't save closes.

import { readFileSync } from "node:fs";
import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { chooseFile, clean, load, opened, stuck, type Watched, WITHOUT_PICKER } from "./reader.ts";
import { type RemoteHost, startRemoteHost } from "./remote-host.ts";

const origin = inject("origin");
const CORPUS = new URL("../../../fixtures/corpus/", import.meta.url);

/** What the polite live region says now. */
const said = (page: Page) => page.locator('.paper-visually-hidden[aria-live="polite"]').first().textContent();

/** Chooses the More menu's item `name`. */
async function more(page: Page, name: string): Promise<void> {
  await page.getByRole("button", { name: "More" }).click();
  await page.getByRole("menuitem", { name }).click();
}

/**
 * Waits until the rotation just made is recorded. Before that nothing is unsaved, so closing,
 * by the × or by the address, asks nothing.
 */
async function recorded(page: Page): Promise<void> {
  await expect.poll(() => page.locator(".paper-top-bar-save-state").textContent()).toBe("1 unsaved change");
}

/** Whether the home screen shows with focus on its heading, the document closed and said to be. */
async function home(page: Page, name: string): Promise<void> {
  const heading = page.getByRole("heading", { name: "Open a PDF" });
  await heading.waitFor();
  await expect.poll(() => heading.evaluate((h) => h === document.activeElement)).toBe(true);
  await expect.poll(() => said(page)).toBe(`Closed ${name}`);
  expect(await page.title()).toBe("Lamina PDF");
}

describe(`closing a document in ${BROWSER}`, () => {
  let browser: Browser;
  let host: RemoteHost;

  beforeAll(async () => {
    browser = await launch();
    host = await startRemoteHost();
    host.add(
      "links-and-outlines.pdf",
      readFileSync(new URL("../../../fixtures/golden/links-and-outlines.pdf", import.meta.url)),
    );
  });

  afterAll(async () => {
    await browser?.close();
    await host?.close();
  });

  /** The reader with corpus file outline.pdf open, in a wide window. */
  async function local(): Promise<Watched> {
    const watched = await load(browser, origin, "/", WITHOUT_PICKER, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1280, height: 800 },
    });
    await chooseFile(watched, "outline.pdf");
    await opened(watched, "Outline fixture");
    return watched;
  }

  it("closes with the × after the file name, back to the home screen with the file first in Recent", async () => {
    const watched = await local();
    const { page } = watched;
    await page.getByRole("button", { name: "Close outline.pdf" }).click();
    await home(page, "outline.pdf");
    await expect.poll(() => page.getByRole("button", { name: /outline\.pdf/ }).count()).toBe(1);
    await clean(watched);
  });

  it("closes from More and from the palette, and opens the picker from More's Open another file…", async () => {
    const watched = await local();
    const { page } = watched;
    const chooser = page.waitForEvent("filechooser");
    await more(page, "Open another file…");
    await (await chooser).setFiles(new URL("outline.pdf", CORPUS).pathname);
    // Opened again before closing: a close while it opens says nothing, as only an open document is announced.
    await opened(watched, "Outline fixture");
    await more(page, "Close document");
    await home(page, "outline.pdf");
    await chooseFile(watched, "outline.pdf");
    await opened(watched, "Outline fixture");
    await page.keyboard.press("ControlOrMeta+K");
    await page.keyboard.type("Close document");
    await page.keyboard.press("Enter");
    await home(page, "outline.pdf");
    await clean(watched);
  });

  it("asks before the × closes unsaved changes, and Cancel returns focus to the ×", async () => {
    const watched = await local();
    const { page } = watched;
    await more(page, "Rotate page right");
    await recorded(page);
    const close = page.getByRole("button", { name: "Close outline.pdf" });
    await close.click();
    const ask = page.getByRole("alertdialog", { name: "Save changes to outline.pdf?" });
    await ask.getByRole("button", { name: "Cancel" }).click();
    await expect.poll(() => close.evaluate((b) => b === document.activeElement)).toBe(true);
    await close.click();
    await ask.getByRole("button", { name: "Don't save" }).click();
    await home(page, "outline.pdf");
    await clean(watched);
  });

  it("asks before the address drops #url= on unsaved changes; Cancel puts the link back (C8a)", async () => {
    const url = `${host.origin}/cors/ranges/links-and-outlines.pdf`;
    const watched = await load(browser, origin, `/#url=${encodeURIComponent(url)}`, undefined, {
      ignoreHTTPSErrors: true,
      home: false,
      viewport: { width: 1280, height: 800 },
    });
    const { page } = watched;
    await stuck(watched, "opening the #url= link", () => page.locator(".web-viewer canvas").first().waitFor());
    await more(page, "Rotate page right");
    await recorded(page);
    await page.evaluate(() => {
      location.hash = "";
    });
    const ask = page.getByRole("alertdialog", { name: "Save changes to links-and-outlines.pdf?" });
    await stuck(watched, "asking after #url= went", () => ask.getByRole("button", { name: "Cancel" }).click());
    await expect.poll(() => page.evaluate(() => location.hash)).toBe(`#url=${encodeURIComponent(url)}`);
    expect(await page.locator(".web-viewer canvas").count()).toBeGreaterThan(0);
    await page.evaluate(() => {
      location.hash = "";
    });
    await ask.getByRole("button", { name: "Don't save" }).click();
    await home(page, "links-and-outlines.pdf");
    expect(await page.evaluate(() => location.hash)).toBe("");
    await page.close();
  });
});
