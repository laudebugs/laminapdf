// An example's first page beside or below its code (todo Q4d), against `wrangler dev` (e2e/server.ts),
// in one browser per run (LAMINA_PDF_E2E_BROWSER): beside the code at 1440 px, below it at 390 px,
// shown with no click and with JavaScript off, the Result tab only where there is more than that
// one page, no horizontal scroll, and no serious or critical axe violation in the page's content at
// either width in either theme.

import { env } from "node:process";
import { firefoxEnv } from "@laminapdf/test-utils/firefox";
import axe from "axe-core";
import { type Browser, type BrowserType, chromium, firefox, type Page, webkit } from "playwright";
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

interface Opened {
  readonly page: Page;
  readonly errors: string[];
}

/** `path` at `width` px, JavaScript on or off, axe let in past the CSP when asked; console errors recorded. */
async function open(
  path: string,
  width: number,
  options: { javaScript?: boolean; scheme?: "light" | "dark"; axe?: boolean } = {},
): Promise<Opened> {
  // The server's certificate is self-signed (@laminapdf/test-utils/wrangler-dev).
  const context = await browser.newContext({
    ignoreHTTPSErrors: true,
    viewport: { width, height: 900 },
    javaScriptEnabled: options.javaScript ?? true,
    colorScheme: options.scheme ?? "light",
    bypassCSP: options.axe ?? false,
  });
  const page = await context.newPage();
  const errors: string[] = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  await page.goto(new URL(path, origin).href);
  return { page, errors };
}

/** Where the first example's code and first page sit, and whether the page is wider than the window. */
async function layout(page: Page) {
  // The picture is shown as the page loads; it has arrived once it has its pixels.
  await page.locator(".example-first img").first().waitFor();
  await page.waitForFunction(() => {
    const img = document.querySelector<HTMLImageElement>(".example-first img");
    return img?.complete === true && img.naturalWidth > 0;
  });
  return page.evaluate(() => {
    const box = (selector: string) => {
      const r = document.querySelector(selector)?.getBoundingClientRect();
      if (r === undefined) throw new Error(`${selector} is not on the page`);
      return { left: r.left, right: r.right, top: r.top, bottom: r.bottom, width: r.width };
    };
    return {
      code: box(".example-code"),
      picture: box(".example-first img"),
      scrolls: document.documentElement.scrollWidth > window.innerWidth,
    };
  });
}

/**
 * axe's serious and critical violations in the page's main content (the examples and the prose
 * around them; the header is the theme test's) now, as `id: targets`.
 */
async function serious(page: Page): Promise<string[]> {
  // Expressive Code makes a code block that scrolls focusable once the browser is idle.
  await page.waitForFunction(() =>
    [...document.querySelectorAll("main pre")].every(
      (pre) => pre.scrollWidth <= pre.clientWidth || pre.hasAttribute("tabindex"),
    ),
  );
  await page.evaluate(axe.source);
  return page.evaluate(async () => {
    const run = (window as unknown as { axe: typeof axe }).axe;
    const result = await run.run("main", {
      runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"] },
    });
    return result.violations
      .filter((v) => v.impact === "serious" || v.impact === "critical")
      .map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`);
  });
}

describe(`an example's first page in ${BROWSER}`, () => {
  it.each([true, false])("is beside the code at 1440 px (JavaScript on: %s)", async (javaScript) => {
    const { page, errors } = await open("/recipes/invoice/", 1440, { javaScript });
    const { code, picture, scrolls } = await layout(page);
    expect(picture.left).toBeGreaterThan(code.right);
    expect(Math.abs(picture.top - code.top)).toBeLessThan(2);
    // Code readable first: at least 36rem of it beside the page.
    expect(code.width).toBeGreaterThanOrEqual(576);
    expect(picture.width).toBeGreaterThan(200);
    expect(scrolls).toBe(false);
    expect(errors).toEqual([]);
    await page.context().close();
  });

  it.each([true, false])("is below the code at 390 px (JavaScript on: %s)", async (javaScript) => {
    const { page, errors } = await open("/start/first-pdf/", 390, { javaScript });
    const { code, picture, scrolls } = await layout(page);
    expect(picture.top).toBeGreaterThan(code.bottom);
    expect(picture.left).toBeCloseTo(code.left, 0);
    expect(scrolls).toBe(false);
    expect(errors).toEqual([]);
    await page.context().close();
  });

  it("keeps the Result tab only for a result of more than one page", async () => {
    const { page } = await open("/recipes/invoice/", 1440);
    expect(await page.getByRole("tab", { name: "Result" }).count()).toBe(0);
    await page.goto(new URL("/recipes/element-flow/", origin).href);
    await page.getByRole("tab", { name: "Result" }).click();
    expect(await page.getByRole("tabpanel").getByRole("img").count()).toBe(2);
    await page.context().close();
  });

  it.each(["light", "dark"] as const)("has no serious axe violation on its page, %s theme", async (scheme) => {
    for (const width of [1440, 390]) {
      const { page } = await open("/start/first-pdf/", width, { scheme, axe: true });
      await layout(page);
      expect(await serious(page)).toEqual([]);
      await page.context().close();
    }
  });
});
