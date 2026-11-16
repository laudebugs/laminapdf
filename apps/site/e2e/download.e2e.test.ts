// The download page (/download/) against `wrangler dev` (e2e/server.ts), built from
// e2e/downloads.fixture.json (`cargo xtask pdf-runtimes` sets LAMINA_PDF_DOWNLOADS), in one
// browser per run: the installer it offers first on each system, the full table against the list,
// and no script errors under the website's CSP. The user agent is set per visitor, and Chromium's
// client hints are removed or stubbed, since a context's user agent doesn't change them.

import { readFileSync } from "node:fs";
import { BROWSER, launchBrowser } from "@laminapdf/test-utils/e2e-browser";
import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";

const SITE = new URL("../", import.meta.url);
const origin = inject("origin");
const FIXTURE = JSON.parse(readFileSync(new URL("./downloads.fixture.json", import.meta.url), "utf8")) as {
  beta: { files: { name: string; sha256: string; url: string }[] };
};

const MAC =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_6) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15";
const WINDOWS =
  "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0 Safari/537.36";
const LINUX = "Mozilla/5.0 (X11; Linux x86_64; rv:144.0) Gecko/20100101 Firefox/144.0";
const IPHONE =
  "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148";

let browser: Browser;
let errors: string[] = [];

beforeAll(async () => {
  browser = await launchBrowser(SITE);
});

afterAll(async () => {
  await browser?.close();
});

/**
 * /download/ as a visitor with `userAgent`, with `hints` as the page's client hints (`null`: the
 * browser has none, as Safari and Firefox).
 */
async function open(userAgent: string, hints: { platform: string; architecture: string } | null): Promise<Page> {
  const context = await browser.newContext({ ignoreHTTPSErrors: true, userAgent, reducedMotion: "reduce" });
  await context.addInitScript((given) => {
    const data =
      given === null
        ? undefined
        : { getHighEntropyValues: async () => ({ ...given, mobile: false }), mobile: false, platform: given.platform };
    Object.defineProperty(Navigator.prototype, "userAgentData", { get: () => data, configurable: true });
  }, hints);
  const page = await context.newPage();
  errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  await page.goto(new URL("/download/", origin).href);
  await page.waitForLoadState("load");
  return page;
}

/** The beta pane's cards the visitor sees, by what each offers. */
async function shownCards(page: Page): Promise<string[]> {
  const cards = page.locator("#pane-beta [data-card]:visible");
  await expect.poll(() => cards.count()).toBeGreaterThan(0);
  return cards.evaluateAll((all) => all.map((card) => (card as HTMLElement).dataset.card ?? ""));
}

describe(`the download page in ${BROWSER}`, () => {
  it("offers a Mac both builds when the browser doesn't say which processor it has", async () => {
    expect(await shownCards(await open(MAC, null))).toEqual(["darwin-aarch64", "darwin-x86_64"]);
  });

  it("offers a Mac with Apple silicon its own build when the client hints say so", async () => {
    expect(await shownCards(await open(MAC, { platform: "macOS", architecture: "arm" }))).toEqual(["darwin-aarch64"]);
  });

  it("offers a 64-bit Windows PC the x64 installer", async () => {
    expect(await shownCards(await open(WINDOWS, null))).toEqual(["windows-x86_64"]);
  });

  it("offers a 64-bit Linux computer the x64 AppImage", async () => {
    const page = await open(LINUX, null);
    expect([
      await shownCards(page),
      await page.locator('#pane-beta [data-card="linux-x86_64"] a').getAttribute("href"),
    ]).toEqual([["linux-x86_64"], FIXTURE.beta.files.find((file) => file.name.endsWith("amd64.AppImage"))?.url]);
  });

  it("sends a phone to the web reader", async () => {
    expect(await shownCards(await open(IPHONE, null))).toEqual(["phone"]);
  });

  it("lists every installer with the list's SHA-256", async () => {
    const page = await open(LINUX, null);
    const rows = await page.locator("#pane-beta tbody tr").evaluateAll((all) =>
      all.map((row) => ({
        name: row.querySelector("td a")?.textContent ?? "",
        sha256: row.querySelector(".download-hash")?.textContent ?? "",
      })),
    );
    const expected = FIXTURE.beta.files.map(({ name, sha256 }) => ({ name, sha256 }));
    expect(rows.sort((a, b) => a.name.localeCompare(b.name))).toEqual(
      expected.sort((a, b) => a.name.localeCompare(b.name)),
    );
  });

  it("runs without a script error", async () => {
    await shownCards(await open(WINDOWS, null));
    expect(errors).toEqual([]);
  });
});
