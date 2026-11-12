// The docs' CSP allows no inline styles (`style-src 'self'`), so every style the build wrote as an
// attribute is a class (@laminapdf/site-kit's inline-styles step). Against `wrangler dev`
// (e2e/server.ts), which sends the production headers, in one browser per run
// (LAMINA_PDF_E2E_BROWSER): pages with code, the sidebar, search and the phone menu report no CSP
// violation, and code tokens keep their colours in both themes.

import { env } from "node:process";
import { firefoxEnv } from "@laminapdf/test-utils/firefox";
import { type Browser, type BrowserType, chromium, firefox, type Page, webkit } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";

const DOCS = new URL("../", import.meta.url);
const BROWSERS: Record<string, BrowserType> = { chromium, firefox, webkit };
const BROWSER = env.LAMINA_PDF_E2E_BROWSER ?? "chromium";
const origin = inject("origin");

/** Pages with code blocks, tabs, the generated reference and an error code's page. */
const PAGES = ["/", "/start/first-pdf/", "/recipes/generate-then-edit/", "/reference/sdk/classes/document/"];

let browser: Browser;

beforeAll(async () => {
  const type = BROWSERS[BROWSER];
  if (type === undefined) throw new Error(`LAMINA_PDF_E2E_BROWSER: unknown browser ${BROWSER}`);
  browser = await type.launch(BROWSER === "firefox" ? { env: firefoxEnv(DOCS) } : {});
});

afterAll(async () => {
  await browser?.close();
});

/** A page in `scheme` at `width`, recording CSP violations and console errors. */
async function open(scheme: "dark" | "light", width = 1280): Promise<{ page: Page; problems: string[] }> {
  // The server's certificate is self-signed (@laminapdf/test-utils/wrangler-dev).
  const context = await browser.newContext({
    ignoreHTTPSErrors: true,
    colorScheme: scheme,
    viewport: { width, height: 800 },
  });
  await context.addInitScript(() => {
    document.addEventListener("securitypolicyviolation", (event) => {
      console.error(`CSP violation: ${event.violatedDirective} ${event.blockedURI}`);
    });
  });
  const page = await context.newPage();
  const problems: string[] = [];
  page.on("console", (message) => {
    if (message.type() === "error") problems.push(message.text());
  });
  return { page, problems };
}

/**
 * The first code token whose class sets both themes' colours: its computed colour and the two it
 * sets (`--0` dark, `--1` light, as Expressive Code names them), as `rgb(…)`.
 */
const token = (page: Page) =>
  page.evaluate(() => {
    const rgb = (hex: string) => {
      const n = Number.parseInt(hex.trim().slice(1, 7), 16);
      return `rgb(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255})`;
    };
    for (const span of document.querySelectorAll(".expressive-code .ec-line span.s-vars")) {
      const style = getComputedStyle(span);
      const [dark, light] = [style.getPropertyValue("--0"), style.getPropertyValue("--1")];
      if (dark.trim().length === 7 && light.trim().length === 7)
        return { colour: style.color, dark: rgb(dark), light: rgb(light) };
    }
    return null;
  });

describe(`styles without 'unsafe-inline' in ${BROWSER}`, () => {
  for (const scheme of ["dark", "light"] as const) {
    it(`shows every page with no CSP violation in ${scheme}`, async () => {
      const { page, problems } = await open(scheme);
      for (const path of PAGES) await page.goto(new URL(path, origin).href, { waitUntil: "networkidle" });
      await page.getByRole("button", { name: "Search" }).first().click();
      await page.getByRole("dialog", { name: "Search" }).waitFor();
      expect(problems).toEqual([]);
      await page.context().close();
    });

    it(`colours code tokens in ${scheme}`, async () => {
      const { page } = await open(scheme);
      await page.goto(new URL("/start/first-pdf/", origin).href, { waitUntil: "networkidle" });
      const found = await token(page);
      expect(found).not.toBeNull();
      expect(found?.colour).toBe(scheme === "dark" ? found?.dark : found?.light);
      await page.context().close();
    });
  }

  it("opens the phone menu with no CSP violation", async () => {
    const { page, problems } = await open("light", 390);
    await page.goto(new URL("/start/first-pdf/", origin).href, { waitUntil: "networkidle" });
    await page.getByRole("button", { name: "Menu" }).click();
    await page.locator("#starlight__sidebar").waitFor({ state: "visible" });
    expect(problems).toEqual([]);
    await page.context().close();
  });
});
