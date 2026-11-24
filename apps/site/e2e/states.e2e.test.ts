// The website's interaction states against `wrangler dev` (e2e/server.ts), in one browser per run
// (LAMINA_PDF_E2E_BROWSER), as a visitor meets them (motion plan §3, §6): every clickable element
// on every page shows the hand, at a desktop's and a phone's width and with the phone menu open;
// the buttons and code tabs show a pressed state; and a focused element keeps its own corners.

import { cursorCheck } from "@laminapdf/test-utils/cursors";
import { BROWSER, launchBrowser } from "@laminapdf/test-utils/e2e-browser";
import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";

const SITE = new URL("../", import.meta.url);
const origin = inject("origin");

/** Every page the website builds, and an address it doesn't (its 404 page). */
const PAGES = ["/", "/download/", "/how-we-test/", "/privacy/", "/support/", "/no-such-page/"];
const DESKTOP = { width: 1280, height: 800 };
const PHONE = { width: 390, height: 844 };

let browser: Browser;

beforeAll(async () => {
  browser = await launchBrowser(SITE);
});

afterAll(async () => {
  await browser?.close();
});

/** `path` in a fresh visitor's browser at `viewport`, once it has loaded. */
async function open(path: string, viewport: { width: number; height: number }): Promise<Page> {
  // The server's certificate is self-signed (@laminapdf/test-utils/wrangler-dev).
  const context = await browser.newContext({ ignoreHTTPSErrors: true, viewport, reducedMotion: "reduce" });
  const page = await context.newPage();
  await page.goto(new URL(path, origin).href);
  await page.waitForLoadState("load");
  return page;
}

/** The page's clickable elements whose cursor breaks the rule (@laminapdf/test-utils/cursors). */
/** The cursor problems on the page; fails when it checked no control, since covered ones are left out. */
async function problems(page: Page) {
  const { checked, problems } = await page.evaluate(cursorCheck, "");
  expect(checked, "controls the cursor check reached").toBeGreaterThan(0);
  return problems;
}

/** `selector`'s background (colour and layer) as drawn now. */
const fill = (page: Page, selector: string) =>
  page
    .locator(selector)
    .first()
    .evaluate((e) => {
      const style = getComputedStyle(e);
      return `${style.backgroundColor} ${style.backgroundImage}`;
    });

describe(`the website's cursors in ${BROWSER}`, () => {
  for (const [name, viewport] of [
    ["a desktop", DESKTOP],
    ["a phone", PHONE],
  ] as const) {
    it(`shows the hand on every clickable element, at ${name}'s width`, async () => {
      for (const path of PAGES) {
        const page = await open(path, viewport);
        expect(await problems(page), path).toEqual([]);
        await page.context().close();
      }
    });
  }

  it("shows the hand in the phone menu", async () => {
    const page = await open("/", PHONE);
    await page.locator(".site-menu > summary").click();
    await expect.poll(() => page.locator(".site-menu-panel").isVisible()).toBe(true);
    expect(await problems(page)).toEqual([]);
    await page.context().close();
  });
});

describe(`the website's states in ${BROWSER}`, () => {
  for (const selector of [".button-primary", ".button-secondary", ".code-tab"]) {
    it(`draws ${selector} pressed while the pointer holds it`, async () => {
      const page = await open("/", DESKTOP);
      const rest = await fill(page, selector);
      const box = await page.locator(selector).first().boundingBox();
      if (box === null) throw new Error(`no ${selector}`);
      // Pressed off the element's text, so a link isn't dragged; released elsewhere.
      await page.mouse.move(box.x + 4, box.y + box.height / 2);
      await page.mouse.down();
      // Press is at once; a hover fade before it may still be under way, so poll.
      await expect.poll(() => fill(page, selector)).not.toBe(rest);
      const pressed = await fill(page, selector);
      await page.mouse.move(0, 0);
      await page.mouse.up();
      expect(pressed).not.toBe(rest);
      await page.context().close();
    });
  }

  it("keeps a focused element's own corners: the ring takes no radius", async () => {
    const page = await open("/", DESKTOP);
    const tab = page.locator(".code-tab[aria-selected='true']");
    await tab.focus();
    const style = await tab.evaluate((e) => {
      const s = getComputedStyle(e);
      return { radius: s.borderTopLeftRadius, outline: s.outlineStyle, offset: s.outlineOffset };
    });
    expect(style).toEqual({ radius: "0px", outline: "solid", offset: "2px" });
    await page.context().close();
  });
});
