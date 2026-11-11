// The theme switch (src/components/ThemeSelect.astro, on @laminapdf/site-kit's select) against
// `wrangler dev` (e2e/server.ts), in one browser per run (LAMINA_PDF_E2E_BROWSER), as a reader uses
// it: it stands as tall as the search box beside it, a choice shows at once and survives a reload,
// Auto follows the system and clears the stored choice, the header's and the phone menu's switches
// stay in step, and no CSP violation is reported.

import { env } from "node:process";
import { firefoxEnv } from "@laminapdf/test-utils/firefox";
import { type Browser, type BrowserContext, type BrowserType, chromium, firefox, type Page, webkit } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";

const DOCS = new URL("../", import.meta.url);
const BROWSERS: Record<string, BrowserType> = { chromium, firefox, webkit };
const BROWSER = env.LAMINA_PDF_E2E_BROWSER ?? "chromium";
const origin = inject("origin");

let browser: Browser;

beforeAll(async () => {
  const type = BROWSERS[BROWSER];
  if (type === undefined) throw new Error(`LAMINA_PDF_E2E_BROWSER: unknown browser ${BROWSER}`);
  browser = await type.launch(BROWSER === "firefox" ? { env: firefoxEnv(DOCS) } : {});
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
const switches = (page: Page) =>
  page.evaluate(() =>
    [...document.querySelectorAll("starlight-theme-select select")].map((s) => (s as HTMLSelectElement).value),
  );

describe(`the theme switch in ${BROWSER}`, () => {
  it("starts on Auto, showing the system's theme", async () => {
    const { context, page, errors } = await open("dark");
    const select = page.getByRole("combobox", { name: "Select theme" }).first();
    expect(await select.inputValue()).toBe("auto");
    expect(await shownTheme(page)).toBe("dark");
    expect(errors).toEqual([]);
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
    const theme = await box("starlight-theme-select select");
    expect(theme.height).toBe(search.height);
    expect(theme.middle).toBeCloseTo(search.middle, 0);
    await context.close();
  });

  it("shows a choice at once and keeps it after a reload", async () => {
    const { context, page, errors } = await open("dark");
    const select = page.getByRole("combobox", { name: "Select theme" }).first();
    await select.selectOption("light");
    expect(await shownTheme(page)).toBe("light");
    expect(await stored(page)).toBe("light");
    await page.reload();
    expect(await select.inputValue()).toBe("light");
    expect(await shownTheme(page)).toBe("light");
    expect(errors).toEqual([]);
    await context.close();
  });

  it("goes back to the system's theme on Auto, storing no choice", async () => {
    const { context, page, errors } = await open("light");
    const select = page.getByRole("combobox", { name: "Select theme" }).first();
    await select.selectOption("dark");
    expect(await shownTheme(page)).toBe("dark");
    await select.selectOption("auto");
    expect(await shownTheme(page)).toBe("light");
    expect(await stored(page)).toBe("");
    expect(errors).toEqual([]);
    await context.close();
  });

  it("keeps the phone menu's switch in step with the header's", async () => {
    const { context, page, errors } = await open("dark", { width: 375, height: 740 });
    await page.getByRole("button", { name: /menu/i }).click();
    const menuSwitch = page.locator("starlight-theme-select select").last();
    await menuSwitch.scrollIntoViewIfNeeded();
    expect(await menuSwitch.isVisible()).toBe(true);
    await menuSwitch.selectOption("light");
    expect(await shownTheme(page)).toBe("light");
    expect(await switches(page)).toEqual(["light", "light"]);
    expect(errors).toEqual([]);
    await context.close();
  });
});
