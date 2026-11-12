// The docs under their own CSP, whose style-src is 'self' alone (public/_headers), against
// `wrangler dev` (e2e/server.ts) in one browser per run (LAMINA_PDF_E2E_BROWSER), as a reader sees
// them: code is coloured in both themes from classes (no style attribute or <style> element is
// left, @laminapdf/site-kit's inlineStylesToClasses), search opens and shows its results, the
// search button shows its shortcut once search has loaded, and no page reports a CSP violation.

import { env } from "node:process";
import { firefoxEnv } from "@laminapdf/test-utils/firefox";
import { type CspReport, withoutSelectErrors, withoutSelectReports } from "@laminapdf/test-utils/webkit-select";
import { type Browser, type BrowserType, chromium, firefox, type Page, webkit } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";

const DOCS = new URL("../", import.meta.url);
const BROWSERS: Record<string, BrowserType> = { chromium, firefox, webkit };
const BROWSER = env.LAMINA_PDF_E2E_BROWSER ?? "chromium";
const origin = inject("origin");

/** Pages with code in the forms the docs have (tabs, a Result tab, generated reference), and two without. */
const PAGES = [
  "/",
  "/start/first-pdf/",
  "/guides/edit-a-pdf/",
  "/reference/sdk/classes/document/",
  "/reference/codes/",
];
const WITHOUT_CODE = new Set(["/", "/reference/codes/"]);

let browser: Browser;

beforeAll(async () => {
  const type = BROWSERS[BROWSER];
  if (type === undefined) throw new Error(`LAMINA_PDF_E2E_BROWSER: unknown browser ${BROWSER}`);
  browser = await type.launch(BROWSER === "firefox" ? { env: firefoxEnv(DOCS) } : {});
});

afterAll(async () => {
  await browser?.close();
});

/**
 * `path` in a fresh reader whose system is `scheme`. `problems()` gives its console errors and CSP
 * violations, less WebKit's report of each <select> (@laminapdf/test-utils/webkit-select).
 */
async function open(
  path: string,
  scheme: "dark" | "light",
): Promise<{ page: Page; problems: () => Promise<string[]>; close: () => Promise<void> }> {
  // The server's certificate is self-signed (@laminapdf/test-utils/wrangler-dev).
  const context = await browser.newContext({
    ignoreHTTPSErrors: true,
    colorScheme: scheme,
    viewport: { width: 1280, height: 800 },
  });
  const page = await context.newPage();
  const errors: string[] = [];
  const reports: (CspReport & { blocked: string })[] = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  await page.exposeFunction("__violation", (directive: string, blocked: string, target: string) => {
    reports.push({ directive, blocked, target });
  });
  await page.addInitScript(() => {
    document.addEventListener("securitypolicyviolation", (e) => {
      (window as unknown as { __violation: (d: string, b: string, t: string) => void }).__violation(
        e.violatedDirective,
        e.blockedURI,
        (e.target as Node | null)?.nodeName ?? "",
      );
    });
  });
  await page.goto(new URL(path, origin).href);
  await page.waitForLoadState("load");
  const problems = async () => {
    const selects = await page.locator("select").count();
    const left = withoutSelectReports(BROWSER, reports, selects);
    return [...withoutSelectErrors(BROWSER, errors, selects), ...left.map((r) => `CSP: ${r.directive} ${r.blocked}`)];
  };
  return { page, problems, close: () => context.close() };
}

/** How many of the page's code tokens have a colour other than their line's: the coloured ones. */
const colouredTokens = (page: Page) =>
  page.evaluate(() => {
    let coloured = 0;
    for (const token of document.querySelectorAll(".expressive-code .ec-line span")) {
      const line = token.closest(".ec-line");
      if (line !== null && getComputedStyle(token).color !== getComputedStyle(line).color) coloured++;
    }
    return coloured;
  });

describe(`the docs under a CSP without inline styles in ${BROWSER}`, () => {
  for (const scheme of ["light", "dark"] as const) {
    it(`colours code from classes in the ${scheme} theme, with no style attribute and no CSP violation`, async () => {
      for (const path of PAGES) {
        const { page, problems, close } = await open(path, scheme);
        // Script may set styles once the page has loaded, which the CSP allows: only the markup
        // counts, so this reads the HTML the server sent.
        const sent = await (await page.request.get(new URL(path, origin).href)).text();
        expect(/\sstyle\s*=|<style[\s>]/i.test(sent), `${path} sends an inline style`).toBe(false);
        if (!WITHOUT_CODE.has(path)) expect(await colouredTokens(page), `${path}'s code`).toBeGreaterThan(0);
        expect(await problems(), path).toEqual([]);
        await close();
      }
    });
  }

  it("opens search, finds a page, and shows the shortcut once search has loaded", async () => {
    const { page, problems, close } = await open("/install/", "light");
    const button = page.locator("button[data-open-modal]").first();
    await expect.poll(() => button.isEnabled()).toBe(true);
    await expect.poll(() => page.locator("button[data-open-modal] > kbd").first().isVisible()).toBe(true);
    await button.click();
    await page.locator(".pagefind-ui__search-input").first().fill("encrypted");
    await expect.poll(() => page.locator(".pagefind-ui__result").count(), { timeout: 15_000 }).toBeGreaterThan(0);
    expect(await problems()).toEqual([]);
    await close();
  });

  it("types over the editor's lines with no CSP violation, and a bracket wraps them", async () => {
    // A one-page result has no Result tab: "Edit and run" is under the picture beside the code.
    const { page, problems, close } = await open("/recipes/hello/", "light");
    await page
      .getByRole("button", { name: /Edit and run/ })
      .first()
      .click();
    const content = page.locator(".cm-content");
    const before = await content.innerText();
    await content.click();
    await page.keyboard.press("ControlOrMeta+a");
    await page.keyboard.type("(");
    const after = await content.innerText();
    // The whole code, wrapped (innerText counts the last line break its own way: whitespace aside).
    expect(after.replace(/\s/g, "")).toBe(`(${before.replace(/\s/g, "")})`);
    await page.keyboard.press("ControlOrMeta+a");
    await page.keyboard.type("x");
    expect(await content.innerText()).toBe("x");
    expect(await problems()).toEqual([]);
    await close();
  });
});
