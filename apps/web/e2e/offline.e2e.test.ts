// The reader offline (todo C7b; R15), in this run's browser. After one visit the service worker
// controls the page; with the network cut, a reload still shows the reader, its shell served from
// the worker's cache with exactly the headers the server sends (ADR 0011 §4), an inline script
// still blocked; a local file opens, renders and can be searched; the offline notice and the
// status bar say so, and the notice goes once the network is back.

import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { expectHeaders, load, WITHOUT_PICKER } from "./reader.ts";

const origin = inject("origin");
const FILE = new URL("../../../fixtures/golden/text-latin.pdf", import.meta.url);

/** Waits until the service worker controls `page`, the first visit's install done. */
async function controlled(page: Page): Promise<void> {
  await page.evaluate(() => navigator.serviceWorker.ready);
  await expect.poll(() => page.evaluate(() => navigator.serviceWorker.controller !== null)).toBe(true);
}

describe(`offline in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  it("works with no network after one visit: the shell, its headers, a local file, find", async () => {
    const watched = await load(browser, origin, "/", WITHOUT_PICKER, {
      ignoreHTTPSErrors: true,
      viewport: { width: 1280, height: 800 },
    });
    const { page } = watched;
    await controlled(page);
    await page.context().setOffline(true);

    const response = await page.reload();
    expect(response?.fromServiceWorker()).toBe(true);
    expectHeaders(new Response(null, { headers: await (response?.allHeaders() ?? {}) }));
    await page.getByRole("heading", { name: "Open a PDF" }).waitFor();
    await page.getByText("You're offline.").waitFor();
    expect(await page.locator(".paper-reader-status").getByText("Offline").count()).toBe(1);

    // The cached shell keeps its CSP: an inline script added to the page doesn't run.
    const ran = await page.evaluate(async () => {
      const script = document.createElement("script");
      script.textContent = "window.__inline = true";
      document.head.append(script);
      await new Promise((resolve) => setTimeout(resolve, 50));
      return (window as unknown as { __inline?: boolean }).__inline === true;
    });
    expect(ran).toBe(false);

    const chooser = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Choose a file" }).click();
    await (await chooser).setFiles(FILE.pathname);
    await page.locator(".web-viewer canvas").first().waitFor({ timeout: 30_000 });
    await page.keyboard.press("ControlOrMeta+F");
    await page.getByRole("textbox", { name: "Find in document" }).fill("the");
    await expect.poll(() => page.locator(".paper-find-count").textContent()).toMatch(/^1 of \d+$/);

    await page.context().setOffline(false);
    await page.getByText("You're offline.").waitFor({ state: "detached" });
    // The page itself made no request to another site, offline or on.
    expect(watched.foreign).toEqual([]);
    await page.close();
  });
});
