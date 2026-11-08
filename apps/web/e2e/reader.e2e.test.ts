// The reader against `wrangler dev` (e2e/server.ts), in one browser per run (LAMINA_PDF_E2E_BROWSER):
// the headers ADR 0011 §4 sets, the built page's lack of inline code, and each way of opening a
// local file (the input, a drop, a paste, showOpenFilePicker in Chromium), a repaired file and a
// refused one. Every page load listens for CSP violations and for requests to another origin: a
// local file must cause neither.

import { readdirSync, readFileSync } from "node:fs";
import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { getLocal } from "./local-tls.ts";
import {
  ASSET_CACHE,
  bytes,
  chooseFile,
  clean,
  expectHeaders,
  HTML_CACHE,
  type InitScript,
  load as loadPage,
  opened,
  type Watched,
  WITHOUT_PICKER,
} from "./reader.ts";

const APP = new URL("../", import.meta.url);
const origin = inject("origin");

/** A built asset's path whose file name matches `pattern`. */
function asset(pattern: RegExp): string {
  const name = readdirSync(new URL("dist/assets/", APP)).find((file) => pattern.test(file));
  if (name === undefined) throw new Error(`dist/assets/ has no file matching ${pattern}`);
  return `/assets/${name}`;
}

/** `GET path` from the local server, which has a self-signed certificate (local-tls.ts), without following redirects. */
function get(server: string, path: string): Promise<Response> {
  return getLocal(new URL(path, server));
}

describe("what the server sends", () => {
  it.each([
    ["/", "text/html"],
    ["/some/deep/link", "text/html"],
  ])("%s is the app, revalidated on every load", async (path, type) => {
    const response = await get(origin, path);
    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toContain(type);
    expect(response.headers.get("cache-control")).toBe(HTML_CACHE);
    expectHeaders(response);
    expect(await response.text()).toContain('<div id="app">');
  });

  it("/index.html redirects to /, the page's one address, with the same headers", async () => {
    const response = await get(origin, "/index.html");
    expect(response.status).toBe(307);
    expect(response.headers.get("location")).toBe("/");
    expectHeaders(response);
  });

  it.each([
    [/^index-.*\.js$/, "text/javascript"],
    [/^pdfium-.*\.wasm$/, "application/wasm"],
  ])("hashed assets (%s) are cached for good", async (pattern, type) => {
    const response = await get(origin, asset(pattern));
    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toContain(type);
    expect(response.headers.get("cache-control")).toBe(ASSET_CACHE);
    expectHeaders(response);
  });

  it("the built page has no inline script or style, and the Worker no code", () => {
    const html = readFileSync(new URL("dist/index.html", APP), "utf8");
    for (const [, attributes = ""] of html.matchAll(/<script([^>]*)>/g)) expect(attributes).toContain("src=");
    expect(html).not.toMatch(/<style|\sstyle=/);
    // A `main` script would answer requests itself, and _headers doesn't apply to its responses.
    expect(readFileSync(new URL("wrangler.jsonc", APP), "utf8")).not.toMatch(/"main"\s*:/);
  });
});

describe(`opening local files in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  /** Opens the app at `path` in this run's browser, after `init`. The server's certificate is self-signed (local-tls.ts). */
  const load = (path = "/", init?: InitScript): Promise<Watched> =>
    loadPage(browser, origin, path, init, { ignoreHTTPSErrors: true });

  /** Dispatches `type` on the window carrying a transfer that holds `name` with `data`. */
  async function transfer(page: Page, type: "drop" | "paste", name: string, data: number[], mime: string) {
    await page.evaluate(
      ({ type, name, data, mime }) => {
        const files = new DataTransfer();
        files.items.add(new File([new Uint8Array(data)], name, { type: mime }));
        // Every engine dispatches this shape; not every one takes the transfer in an event's init.
        const send = (kind: string) => {
          const event = new Event(kind, { bubbles: true, cancelable: true });
          Object.defineProperty(event, type === "paste" ? "clipboardData" : "dataTransfer", { value: files });
          window.dispatchEvent(event);
        };
        if (type === "drop") ["dragenter", "dragover", "drop"].forEach(send);
        else send("paste");
      },
      { type, name, data, mime },
    );
  }

  it("opens a file chosen with the file input", async () => {
    const watched = await load("/", WITHOUT_PICKER);
    await chooseFile(watched, "hello.pdf");
    await opened(watched, "hello.pdf");
    await clean(watched);
  });

  it("opens a file dropped on the window", async () => {
    const watched = await load();
    await transfer(watched.page, "drop", "hello.pdf", bytes("hello.pdf"), "application/pdf");
    await opened(watched, "hello.pdf");
    await clean(watched);
  });

  it("opens a pasted file", async () => {
    const watched = await load();
    await transfer(watched.page, "paste", "hello.pdf", bytes("hello.pdf"), "application/pdf");
    await opened(watched, "hello.pdf");
    await clean(watched);
  });

  it.runIf(BROWSER === "chromium")("opens a file from showOpenFilePicker", async () => {
    const watched = await load("/", {
      script: (data) => {
        const file = new File([new Uint8Array(data)], "hello.pdf", { type: "application/pdf" });
        const handle = { kind: "file", name: file.name, getFile: async () => file };
        Object.assign(window, { showOpenFilePicker: async () => [handle] });
      },
      arg: bytes("hello.pdf"),
    });
    await watched.page.getByRole("button", { name: "Choose a file" }).click();
    await opened(watched, "hello.pdf");
    await clean(watched);
  });

  it("opens a damaged file with the repaired notice and its details", async () => {
    const watched = await load();
    await transfer(watched.page, "drop", "hello-broken.pdf", bytes("hello-broken.pdf"), "application/pdf");
    await opened(watched, "hello-broken.pdf");
    await watched.page.getByRole("button", { name: /Repaired on open/ }).click();
    await watched.page.getByRole("heading", { name: /Repaired on open/ }).waitFor();
    await clean(watched);
  });

  it("refuses a file that isn't a PDF and stays on the home screen", async () => {
    const watched = await load();
    await transfer(watched.page, "drop", "notes.txt", [...new TextEncoder().encode("plain text")], "text/plain");
    await expect.poll(() => watched.page.getByRole("alert").textContent()).toContain("isn't a PDF");
    await watched.page.getByRole("heading", { name: "Open a PDF" }).waitFor();
    await clean(watched);
  });

  it("serves the app at a deep link", async () => {
    const watched = await load("/some/deep/link");
    await clean(watched);
  });
});
