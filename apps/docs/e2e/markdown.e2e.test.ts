// Each page's Markdown twin against `wrangler dev` (e2e/server.ts), in one browser per run
// (LAMINA_PDF_E2E_BROWSER): the head links the twin, and "Copy as Markdown" fetches it only when
// pressed and copies its text, with no CSP violation, on a page with a title and on the home page's
// hero. Headless Firefox and WebKit can't read the clipboard back, so a stand-in clipboard records
// what the button wrote and whether the write started inside the press, which Safari requires. In
// Chromium a second test reads the real clipboard, which the button may only write.

import { env } from "node:process";
import { firefoxEnv } from "@laminapdf/test-utils/firefox";
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

/** What the stand-in clipboard received, and the CSP violations the page met. */
interface Recorded {
  copied: string[];
  outsidePress: number;
  violations: string[];
}

/** The twin's text, as the server sends it. */
async function twinText(page: Page, href: string): Promise<string> {
  return (await page.context().request.get(new URL(href, origin).href)).text();
}

/**
 * Presses the page's "Copy as Markdown" from the keyboard and waits for its status; returns whether
 * the button kept the focus.
 */
async function pressCopy(page: Page): Promise<boolean> {
  const button = page.getByRole("button", { name: "Copy as Markdown" });
  await button.focus();
  await page.keyboard.press("Enter");
  await expect.poll(() => page.getByRole("status").filter({ hasText: "Copied" }).count()).toBe(1);
  return button.evaluate((b) => b === document.activeElement);
}

describe(`each page's Markdown twin in ${BROWSER}`, () => {
  for (const [path, twin] of [
    ["/guides/save-a-pdf/", "/guides/save-a-pdf.md"],
    ["/", "/index.md"],
  ] as const) {
    it(`${path} links ${twin}, and Copy as Markdown copies it within the press`, async () => {
      // The server's certificate is self-signed (@laminapdf/test-utils/wrangler-dev).
      const context = await browser.newContext({ ignoreHTTPSErrors: true });
      await context.addInitScript(() => {
        const recorded: Recorded = { copied: [], outsidePress: 0, violations: [] };
        (window as unknown as { recorded: Recorded }).recorded = recorded;
        // A write counts as inside the press only when it starts in the same task as the click.
        let pressing = false;
        document.addEventListener(
          "click",
          () => {
            pressing = true;
            setTimeout(() => {
              pressing = false;
            });
          },
          true,
        );
        const note = (text: Promise<string>) => {
          if (!pressing) recorded.outsidePress += 1;
          return text.then((t) => void recorded.copied.push(t));
        };
        Object.defineProperty(navigator, "clipboard", {
          value: {
            write: (items: ClipboardItem[]) =>
              note(Promise.resolve(items[0]?.getType("text/plain")).then((blob) => blob?.text() ?? "")),
            writeText: (text: string) => note(Promise.resolve(text)),
          },
        });
        document.addEventListener("securitypolicyviolation", (e) => recorded.violations.push(e.violatedDirective));
      });
      const page = await context.newPage();
      const twins: string[] = [];
      page.on("request", (request) => {
        if (request.url().endsWith(".md")) twins.push(request.url());
      });
      await page.goto(new URL(path, origin).href);

      expect(await page.locator('link[rel="alternate"][type="text/markdown"]').getAttribute("href")).toBe(twin);
      expect(twins).toEqual([]);

      expect(await pressCopy(page)).toBe(true);
      const recorded = await page.evaluate(() => (window as unknown as { recorded: Recorded }).recorded);
      const text = await twinText(page, twin);
      expect(text).toMatch(/^---\ntitle: /);
      expect(recorded).toEqual({ copied: [text], outsidePress: 0, violations: [] });
      expect(twins).toEqual([new URL(twin, origin).href]);
      await context.close();
    });
  }

  it.runIf(BROWSER === "chromium")("copies the twin to the real clipboard", async () => {
    const context = await browser.newContext({ ignoreHTTPSErrors: true });
    await context.grantPermissions(["clipboard-read", "clipboard-write"], { origin });
    const page = await context.newPage();
    await page.goto(new URL("/guides/save-a-pdf/", origin).href);
    await pressCopy(page);
    // The docs' Permissions-Policy blocks clipboard-read (ADR 0011), so the clipboard is read back
    // on a page Playwright serves itself, with no such header.
    const reader = await context.newPage();
    await reader.route(new URL("/__clipboard__", origin).href, (route) =>
      route.fulfill({ contentType: "text/html", body: "<!doctype html><title>clipboard</title>" }),
    );
    await reader.goto(new URL("/__clipboard__", origin).href);
    await reader.bringToFront();
    const copied = await reader.evaluate(() => navigator.clipboard.readText());
    expect(copied).toBe(await twinText(page, "/guides/save-a-pdf.md"));
    await context.close();
  });
});
