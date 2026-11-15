// The theme switch (src/components/ThemeSelect.astro, on @laminapdf/site-kit's MenuSelect) against
// `wrangler dev` (e2e/server.ts), in one browser per run (LAMINA_PDF_E2E_BROWSER), as a reader uses
// it: it stands as tall as the search box beside it, a choice made with the pointer or the keyboard
// shows at once and survives a reload, Auto follows the system and clears the stored choice, the
// header's and the phone menu's switches stay in step, and no CSP violation is reported.

import { BROWSER, launchBrowser } from "@laminapdf/test-utils/e2e-browser";
import { withoutSelectErrors } from "@laminapdf/test-utils/webkit-select";
import type { Browser, BrowserContext, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";

const DOCS = new URL("../", import.meta.url);
const origin = inject("origin");

let browser: Browser;

beforeAll(async () => {
  browser = await launchBrowser(DOCS);
});

afterAll(async () => {
  await browser?.close();
});

/** A fresh reader, its system in `scheme`, on the install page; console errors recorded. */
async function open(
  scheme: "dark" | "light",
  viewport = { width: 1280, height: 720 },
): Promise<{ context: BrowserContext; page: Page; errors: string[] }> {
  // The server's certificate is self-signed (@laminapdf/test-utils/wrangler-dev).
  const context = await browser.newContext({ ignoreHTTPSErrors: true, colorScheme: scheme, viewport });
  const page = await context.newPage();
  const errors: string[] = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  await page.goto(new URL("/install/", origin).href);
  return { context, page, errors };
}

const shownTheme = (page: Page) => page.evaluate(() => document.documentElement.dataset.theme);
const stored = (page: Page) => page.evaluate(() => localStorage.getItem("starlight-theme"));
/** Each switch's value, and the label its button shows. */
const switches = (page: Page) =>
  page.evaluate(() =>
    [...document.querySelectorAll("starlight-theme-select site-menu-select")].map((menu) => {
      const shown = [...menu.querySelectorAll<HTMLElement>("[data-for]")].filter((e) => !e.hidden);
      return `${menu.getAttribute("value")}:${shown.map((e) => e.textContent?.trim()).join("|")}`;
    }),
  );
/** The CSP errors of one load: none expected, and no <select> left for WebKit to report. */
const cspErrors = async (page: Page, errors: string[]) =>
  withoutSelectErrors(BROWSER, errors, await page.locator("select").count());

describe(`the theme switch in ${BROWSER}`, () => {
  it("starts on Auto with its icon, showing the system's theme", async () => {
    const { context, page, errors } = await open("dark");
    const button = page.getByRole("combobox", { name: "Select theme" }).first();
    // The other themes' labels are in the button too, hidden.
    expect((await button.locator("[data-for]:not([hidden])").textContent())?.trim()).toBe("Auto");
    expect(await button.locator("svg.site-menu-select-icon:visible").count()).toBe(1);
    expect(await switches(page)).toEqual(["auto:Auto", "auto:Auto"]);
    expect(await shownTheme(page)).toBe("dark");
    expect(await cspErrors(page, errors)).toEqual([]);
    await context.close();
  });

  it("stands as tall as the search box beside it, on one centre line", async () => {
    const { context, page } = await open("light");
    const box = (selector: string) =>
      page
        .locator(selector)
        .first()
        .evaluate((e) => {
          const r = e.getBoundingClientRect();
          return { height: r.height, middle: r.top + r.height / 2 };
        });
    const search = await box("button[data-open-modal]");
    const theme = await box("starlight-theme-select .site-menu-select-button");
    expect(theme.height).toBe(search.height);
    expect(theme.middle).toBeCloseTo(search.middle, 0);
    await context.close();
  });

  it("opens a list of the three themes, each with an icon, and keeps a choice after a reload", async () => {
    const { context, page, errors } = await open("dark");
    const button = page.getByRole("combobox", { name: "Select theme" }).first();
    await button.click();
    expect(await button.getAttribute("aria-expanded")).toBe("true");
    const options = page.getByRole("option");
    expect(await options.allTextContents()).toEqual(["Light", "Dark", "Auto"].map((t) => expect.stringContaining(t)));
    expect(await page.getByRole("listbox").locator("svg.site-menu-select-icon").count()).toBe(3);
    await page.getByRole("option", { name: "Light" }).click();
    expect(await button.getAttribute("aria-expanded")).toBe("false");
    expect(await shownTheme(page)).toBe("light");
    expect(await stored(page)).toBe("light");
    expect(await switches(page)).toEqual(["light:Light", "light:Light"]);
    expect(await cspErrors(page, errors)).toEqual([]);
    errors.length = 0;

    await page.reload();
    expect(await switches(page)).toEqual(["light:Light", "light:Light"]);
    expect(await shownTheme(page)).toBe("light");
    expect(await cspErrors(page, errors)).toEqual([]);
    await context.close();
  });

  it("is used from the keyboard, the focus staying on the button", async () => {
    const { context, page } = await open("light");
    const button = page.getByRole("combobox", { name: "Select theme" }).first();
    await button.focus();
    await page.keyboard.press("Enter");
    const active = () =>
      page.evaluate(() => {
        const id = document.activeElement?.getAttribute("aria-activedescendant");
        return id ? document.getElementById(id)?.textContent?.trim() : null;
      });
    expect(await active()).toBe("Auto");
    await page.keyboard.press("ArrowUp");
    expect(await active()).toBe("Dark");
    await page.keyboard.press("Escape");
    expect(await button.getAttribute("aria-expanded")).toBe("false");
    expect(await shownTheme(page)).toBe("light");

    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Home");
    expect(await active()).toBe("Light");
    await page.keyboard.press("d");
    expect(await active()).toBe("Dark");
    await page.keyboard.press("Enter");
    expect(await shownTheme(page)).toBe("dark");
    expect(await page.evaluate(() => document.activeElement?.getAttribute("role"))).toBe("combobox");
    await context.close();
  });

  it("closes on a click outside, choosing nothing", async () => {
    const { context, page } = await open("light");
    const button = page.getByRole("combobox", { name: "Select theme" }).first();
    await button.click();
    await page.locator("h1").first().click();
    expect(await button.getAttribute("aria-expanded")).toBe("false");
    expect(await page.getByRole("listbox").first().isVisible()).toBe(false);
    // Starlight's script stores Auto as "" when the page loads; nothing else was stored.
    expect(await stored(page)).toBe("");
    await context.close();
  });

  it("goes back to the system's theme on Auto, storing no choice", async () => {
    const { context, page, errors } = await open("light");
    const button = page.getByRole("combobox", { name: "Select theme" }).first();
    await button.click();
    await page.getByRole("option", { name: "Dark" }).click();
    expect(await shownTheme(page)).toBe("dark");
    await button.click();
    await page.getByRole("option", { name: "Auto" }).click();
    expect(await shownTheme(page)).toBe("light");
    expect(await stored(page)).toBe("");
    expect(await cspErrors(page, errors)).toEqual([]);
    await context.close();
  });

  it("keeps the phone menu's switch in step with the header's", async () => {
    const { context, page, errors } = await open("dark", { width: 375, height: 740 });
    await page.getByRole("button", { name: /menu/i }).click();
    const menuSwitch = page.locator("starlight-theme-select .site-menu-select-button").last();
    await menuSwitch.scrollIntoViewIfNeeded();
    expect(await menuSwitch.isVisible()).toBe(true);
    await menuSwitch.click();
    const light = page.getByRole("option", { name: "Light" });
    expect(await light.isVisible()).toBe(true);
    await light.click();
    expect(await shownTheme(page)).toBe("light");
    expect(await switches(page)).toEqual(["light:Light", "light:Light"]);
    expect(await cspErrors(page, errors)).toEqual([]);
    await context.close();
  });
});
