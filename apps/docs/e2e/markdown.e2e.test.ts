// Each page's Markdown twin (todo Q4l) against `wrangler dev` (e2e/server.ts), in one browser per run
// (LAMINA_PDF_E2E_BROWSER): the head links the twin, and "Copy as Markdown" fetches it only when
// pressed and copies its text, with no CSP violation. The clipboard is replaced before the page loads,
// so the test reads what the button wrote in every browser, whatever its clipboard permissions.

import { env } from "node:process";
import { firefoxEnv } from "@laminapdf/test-utils/firefox";
import { type Browser, type BrowserType, chromium, firefox, webkit } from "playwright";
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

/** What the page wrote to the clipboard, and the CSP violations it met. */
interface Recorded {
  copied: string[];
  violations: string[];
}

describe(`each page's Markdown twin in ${BROWSER}`, () => {
  it("links the twin from the head, and Copy as Markdown copies it on the first press", async () => {
    // The server's certificate is self-signed (@laminapdf/test-utils/wrangler-dev).
    const context = await browser.newContext({ ignoreHTTPSErrors: true });
    await context.addInitScript(() => {
      const recorded: Recorded = { copied: [], violations: [] };
      (window as unknown as { recorded: Recorded }).recorded = recorded;
      Object.defineProperty(navigator, "clipboard", {
        value: { writeText: async (text: string) => void recorded.copied.push(text) },
      });
      document.addEventListener("securitypolicyviolation", (e) => recorded.violations.push(e.violatedDirective));
    });
    const page = await context.newPage();
    const twins: string[] = [];
    page.on("request", (request) => {
      if (request.url().endsWith(".md")) twins.push(request.url());
    });
    await page.goto(new URL("/guides/save-a-pdf/", origin).href);

    const href = await page.locator('link[rel="alternate"][type="text/markdown"]').getAttribute("href");
    expect(href).toBe("/guides/save-a-pdf.md");
    expect(twins).toEqual([]);

    await page.getByRole("button", { name: "Copy as Markdown" }).click();
    await expect.poll(() => page.getByRole("status").filter({ hasText: "Copied" }).count()).toBe(1);
    const twin = await (await context.request.get(new URL(href ?? "", origin).href)).text();
    const recorded = await page.evaluate(() => (window as unknown as { recorded: Recorded }).recorded);
    expect(recorded.copied).toEqual([twin]);
    expect(twin).toMatch(/^---\ntitle: /);
    expect(twins).toEqual([new URL("/guides/save-a-pdf.md", origin).href]);
    expect(recorded.violations).toEqual([]);
    await context.close();
  });
});
