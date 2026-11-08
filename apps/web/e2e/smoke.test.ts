// The deployed reader at LAMINA_PDF_SMOKE_URL (a preview, staging or production), in Chromium: what
// .github/workflows/web-deploy.yml checks after each deploy (todo C4d). The live headers are ADR
// 0011 §4's, unknown paths get the app, hashed assets are cached for good, and hello.pdf opened
// with the file input renders page 1 with no CSP violation and no request to another origin. That
// last check also catches anything Cloudflare's zone settings would inject into the page.

import { env } from "node:process";
import { type Browser, chromium } from "playwright";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import {
  ASSET_CACHE,
  chooseFile,
  clean,
  expectHeaders,
  get,
  HTML_CACHE,
  load,
  opened,
  WITHOUT_PICKER,
} from "./reader.ts";

// e2e/live.ts, the setup, fails unless it is set, and waits until it serves the app.
const origin = env.LAMINA_PDF_SMOKE_URL ?? "";

describe(`the reader deployed at ${origin}`, () => {
  it.each(["/", "/some/deep/link"])("%s is the app, with ADR 0011's headers", async (path) => {
    const response = await get(origin, path);
    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toContain("text/html");
    expect(response.headers.get("cache-control")).toBe(HTML_CACHE);
    expectHeaders(response);
    expect(await response.text()).toContain('<div id="app">');
  });

  it("caches the page's hashed script for good, with the same headers", async () => {
    const html = await (await get(origin, "/")).text();
    const script = html.match(/<script[^>]*\ssrc="(\/assets\/[^"]+\.js)"/)?.[1];
    expect(script, "the page's module script").toBeDefined();
    const response = await get(origin, script ?? "");
    expect(response.status).toBe(200);
    expect(response.headers.get("cache-control")).toBe(ASSET_CACHE);
    expectHeaders(response);
  });

  describe("in Chromium", () => {
    let browser: Browser;

    beforeAll(async () => {
      browser = await chromium.launch();
    });

    afterAll(async () => {
      await browser?.close();
    });

    it("opens hello.pdf from the file input and renders page 1", async () => {
      const watched = await load(browser, origin, "/", WITHOUT_PICKER);
      await chooseFile(watched, "hello.pdf");
      await opened(watched, "hello.pdf");
      await clean(watched);
    });
  });
});
