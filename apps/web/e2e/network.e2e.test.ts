// The network indicator (R13) against the browser's own record of every request: Playwright's
// browser-context request log, which includes the document worker's requests. On every flow the
// panel's counts must equal the log's, split by the same classifier the panel uses. Then forced
// requests: a raw fetch from the worker to another origin is counted, an HTTPS request from the
// page or the worker to an unexpected site is counted and flagged, and an image the page's policy
// blocks is listed as blocked, not counted.

import { readFileSync } from "node:fs";
import axe from "axe-core";
import type { Browser, BrowserContext, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { classifyRequest } from "../src/network/classify.ts";
import { BROWSER, launch } from "./browser.ts";
import { type RemoteHost, startRemoteHost } from "./remote-host.ts";

const origin = inject("origin");
const CORPUS = new URL("../../../fixtures/corpus/", import.meta.url);

/** How each engine words a request its Content Security Policy refused before sending it. */
const BLOCKED_BY_POLICY = /^csp$|BLOCKED_BY_CSP|Content Security Policy|CONTENT_BLOCKED|NS_ERROR_CSP/i;

/**
 * A context whose every http(s) request is logged, but the favicon the browser asks for by itself
 * (made by no page or worker code, so no feed can see it) and those its policy refused (some
 * engines report them as requests that failed; nothing was sent).
 */
async function logged(browser: Browser): Promise<{ context: BrowserContext; log: string[] }> {
  const context = await browser.newContext({ ignoreHTTPSErrors: true });
  const log: string[] = [];
  context.on("request", (request) => {
    const url = request.url();
    if (/^https?:/.test(url) && new URL(url).pathname !== "/favicon.ico") log.push(url);
  });
  context.on("requestfailed", (request) => {
    if (!BLOCKED_BY_POLICY.test(request.failure()?.errorText ?? "")) return;
    const at = log.lastIndexOf(request.url());
    if (at >= 0) log.splice(at, 1);
  });
  return { context, log };
}

/** The panel's counts: document requests and app-file requests. */
async function panelCounts(page: Page): Promise<{ document: number; app: number }> {
  const panel = page.getByRole("complementary", { name: "Network activity" });
  if (!(await panel.isVisible()))
    await page.getByRole("button", { name: /network requests? for this document/ }).click();
  await panel.waitFor();
  const document = Number(await panel.locator(".paper-network-number").textContent());
  const appText = (await panel.getByRole("button", { name: /^App files/ }).textContent()) ?? "";
  return { document, app: Number(/(\d+)/.exec(appText)?.[1]) };
}

/** Waits until the page has been quiet for a moment, so every request is in both records. */
async function settle(page: Page): Promise<void> {
  await page.waitForLoadState("networkidle");
  await page.waitForTimeout(500);
}

/** The log split as the panel splits it: app files, and everything else. */
function split(log: readonly string[], documentUrl?: string): { document: number; app: number } {
  const app = log.filter((url) => classifyRequest(url, { appOrigin: origin, documentUrl }) === "app").length;
  return { document: log.length - app, app };
}

describe(`the network indicator in ${BROWSER}`, () => {
  let browser: Browser;
  let host: RemoteHost;

  beforeAll(async () => {
    browser = await launch();
    host = await startRemoteHost();
    host.add("hello.pdf", readFileSync(new URL("hello.pdf", CORPUS)));
  });

  afterAll(async () => {
    await browser?.close();
    await host?.close();
  });

  it("shows 0 for a local file, and its app files equal the browser's request log", async () => {
    const { context, log } = await logged(browser);
    const page = await context.newPage();
    await page.goto(new URL("/", origin).href);
    await page.evaluate(() => Reflect.deleteProperty(window, "showOpenFilePicker"));
    const chooser = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Choose a file" }).click();
    await (await chooser).setFiles(new URL("hello.pdf", CORPUS).pathname);
    await page.locator(".web-viewer canvas").first().waitFor();
    await settle(page);
    expect(await page.getByRole("button", { name: /network requests? for this document/ }).textContent()).toContain(
      "0 network requests for this document",
    );
    expect(await panelCounts(page)).toEqual(split(log));
    expect(split(log).document).toBe(0);
    await context.close();
  });

  it("counts a linked file's range requests, from its host only", async () => {
    const { context, log } = await logged(browser);
    const page = await context.newPage();
    const url = `${host.origin}/cors/ranges/hello.pdf`;
    await page.goto(new URL(`/#url=${encodeURIComponent(url)}`, origin).href);
    await page.locator(".web-viewer canvas").first().waitFor();
    await settle(page);
    const counts = await panelCounts(page);
    // The log's preflights (OPTIONS) are requests too: the worker's fetches cause them, the browser makes them.
    const expected = split(log, url);
    expect(counts).toEqual(expected);
    expect(counts.document).toBeGreaterThan(0);
    const panel = page.getByRole("complementary", { name: "Network activity" });
    expect(await panel.getByRole("rowheader").allTextContents()).toEqual([
      expect.stringContaining(new URL(host.origin).host),
    ]);
    await context.close();
  });

  /** Opens hello.pdf from disk in a logged context, and waits for its first page. */
  async function openLocal(): Promise<{ context: BrowserContext; log: string[]; page: Page }> {
    const { context, log } = await logged(browser);
    const page = await context.newPage();
    await page.goto(new URL("/", origin).href);
    await page.evaluate(() => Reflect.deleteProperty(window, "showOpenFilePicker"));
    const chooser = page.waitForEvent("filechooser");
    await page.getByRole("button", { name: "Choose a file" }).click();
    await (await chooser).setFiles(new URL("hello.pdf", CORPUS).pathname);
    await page.locator(".web-viewer canvas").first().waitFor();
    await settle(page);
    return { context, log, page };
  }

  it("counts a raw fetch from the document worker and flags its unexpected site", async () => {
    const { context, log, page } = await openLocal();
    const [worker] = page.workers();
    expect(worker).toBeDefined();
    // No Lamina PDF helper: the worker's own global fetch, as any code in it would call it.
    await worker?.evaluate(
      (url) =>
        fetch(url).then(
          () => undefined,
          () => undefined,
        ),
      `${host.origin}/cors/ranges/hello.pdf`,
    );
    await settle(page);
    const indicator = page.getByRole("button", { name: /network requests? for this document/ });
    expect(await indicator.textContent()).toContain(new URL(host.origin).host);
    expect(await panelCounts(page)).toEqual(split(log));
    const panel = page.getByRole("complementary", { name: "Network activity" });
    await panel.getByText("A request went to an unexpected site").waitFor();
    await context.close();
  });

  it("counts an HTTPS request from the page to another site", async () => {
    const { context, log, page } = await openLocal();
    await page.evaluate(
      (url) =>
        fetch(url).then(
          () => undefined,
          () => undefined,
        ),
      `${host.origin}/cors/ranges/hello.pdf`,
    );
    await settle(page);
    expect(await panelCounts(page)).toEqual(split(log));
    expect(split(log).document).toBe(1);
    await context.close();
  });

  it("lists an image the page's policy blocks, and counts nothing for it", async () => {
    const { context, log, page } = await openLocal();
    // img-src allows the app's own origin, blob: and data: only.
    await page.evaluate((url) => {
      const image = new Image();
      image.src = url;
      document.body.append(image);
    }, `${host.origin}/pixel.png`);
    await settle(page);
    expect(log.some((url) => url.includes("pixel.png"))).toBe(false);
    expect(await panelCounts(page)).toEqual(split(log));
    const panel = page.getByRole("complementary", { name: "Network activity" });
    await panel.getByText("Blocked by the page's policy").waitFor();
    expect(await panel.getByText(`${host.origin}/pixel.png`).count()).toBe(1);
    await context.close();
  });

  it("never sends a plain-HTTP request, and counts what the browser sent instead", async () => {
    const { context, log, page } = await openLocal();
    const plain = `${host.origin.replace("https:", "http:")}/cors/ranges/hello.pdf`;
    await page.evaluate(
      (url) =>
        fetch(url).then(
          () => undefined,
          () => undefined,
        ),
      plain,
    );
    await settle(page);
    expect(log.some((url) => url.startsWith("http:"))).toBe(false);
    expect(await panelCounts(page)).toEqual(split(log));
    // Chromium leaves loopback addresses un-upgraded, so connect-src blocks the request and reports
    // it; WebKit upgrades or drops it without a report. Either way nothing went out over HTTP.
    if (BROWSER === "chromium") {
      const panel = page.getByRole("complementary", { name: "Network activity" });
      await panel.getByText("Blocked by the page's policy").waitFor();
    }
    await context.close();
  });

  it("lists a request the worker's policy blocks", async () => {
    const { context, log, page } = await openLocal();
    const [worker] = page.workers();
    // connect-src allows the app's own origin and https: only, so a data: URL is refused.
    await worker?.evaluate(() =>
      fetch("data:text/plain,x").then(
        () => undefined,
        () => undefined,
      ),
    );
    await settle(page);
    expect(await panelCounts(page)).toEqual(split(log));
    const panel = page.getByRole("complementary", { name: "Network activity" });
    await panel.getByText("Blocked by the page's policy").waitFor();
    await context.close();
  });

  it("opens from the keyboard, says whether it is open, and passes axe", async () => {
    const { context, page } = await openLocal();
    const indicator = page.getByRole("button", { name: /network requests? for this document/ });
    expect(await indicator.getAttribute("aria-expanded")).toBe("false");
    await indicator.focus();
    await page.keyboard.press("Enter");
    await page.getByRole("complementary", { name: "Network activity" }).waitFor();
    expect(await indicator.getAttribute("aria-expanded")).toBe("true");
    // Evaluated rather than added as a script tag, which the page's policy would refuse.
    await page.evaluate(axe.source);
    const serious = await page.evaluate(async () => {
      const run = (
        window as unknown as { axe: { run: (n: Node) => Promise<{ violations: { id: string; impact: string }[] }> } }
      ).axe;
      const { violations } = await run.run(document.body);
      return violations.filter((v) => v.impact === "serious" || v.impact === "critical").map((v) => v.id);
    });
    expect(serious).toEqual([]);
    await context.close();
  });
});
