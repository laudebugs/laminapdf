// The reader against `wrangler dev` (e2e/server.ts), in one browser per run (LAMINA_PDF_E2E_BROWSER):
// the headers ADR 0011 §4 sets, the built page's lack of inline code, and each way of opening a
// local file (the input, a drop, a paste, showOpenFilePicker in Chromium), a repaired file and a
// refused one. Every page load listens for CSP violations and for requests to another origin: a
// local file must cause neither.

import { readdirSync, readFileSync } from "node:fs";
import { env } from "node:process";
import { firefoxEnv } from "@laminapdf/test-utils/firefox";
import { type Browser, type BrowserType, chromium, firefox, type Page, webkit } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { getLocal } from "./local-tls.ts";

const APP = new URL("../", import.meta.url);
const CORPUS = new URL("../../../fixtures/corpus/", import.meta.url);
const BROWSERS: Record<string, BrowserType> = { chromium, firefox, webkit };
const BROWSER = env.LAMINA_PDF_E2E_BROWSER ?? "chromium";
const origin = inject("origin");

/** ADR 0011 §4's headers, exactly, on every response. */
const HEADERS: Record<string, string> = {
  "content-security-policy":
    "default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self'; img-src 'self' blob: data:; font-src 'self'; connect-src 'self' https:; worker-src 'self'; manifest-src 'self'; frame-src 'self'; frame-ancestors 'self'; base-uri 'none'; form-action 'none'; object-src 'none'; upgrade-insecure-requests",
  "referrer-policy": "no-referrer",
  "permissions-policy":
    "camera=(), microphone=(), geolocation=(), payment=(), usb=(), serial=(), hid=(), bluetooth=(), browsing-topics=(), clipboard-read=(), clipboard-write=(self)",
  "strict-transport-security": "max-age=63072000; includeSubDomains",
  "x-content-type-options": "nosniff",
  "x-frame-options": "SAMEORIGIN",
  "cross-origin-resource-policy": "same-origin",
};
const HTML_CACHE = "public, max-age=0, must-revalidate";
const ASSET_CACHE = "public, max-age=31536000, immutable";

/** A built asset's path whose file name matches `pattern`. */
function asset(pattern: RegExp): string {
  const name = readdirSync(new URL("dist/assets/", APP)).find((file) => pattern.test(file));
  if (name === undefined) throw new Error(`dist/assets/ has no file matching ${pattern}`);
  return `/assets/${name}`;
}

/** `GET path` without following redirects. */
function get(path: string): Promise<Response> {
  return getLocal(new URL(path, origin));
}

describe("what the server sends", () => {
  it.each([
    ["/", "text/html"],
    ["/some/deep/link", "text/html"],
  ])("%s is the app, revalidated on every load", async (path, type) => {
    const response = await get(path);
    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toContain(type);
    expect(response.headers.get("cache-control")).toBe(HTML_CACHE);
    for (const [name, value] of Object.entries(HEADERS)) expect(response.headers.get(name), name).toBe(value);
    expect(await response.text()).toContain('<div id="app">');
  });

  it("/index.html redirects to /, the page's one address, with the same headers", async () => {
    const response = await get("/index.html");
    expect(response.status).toBe(307);
    expect(response.headers.get("location")).toBe("/");
    for (const [name, value] of Object.entries(HEADERS)) expect(response.headers.get(name), name).toBe(value);
  });

  it.each([
    [/^index-.*\.js$/, "text/javascript"],
    [/^pdfium-.*\.wasm$/, "application/wasm"],
  ])("hashed assets (%s) are cached for good", async (pattern, type) => {
    const response = await get(asset(pattern));
    expect(response.status).toBe(200);
    expect(response.headers.get("content-type")).toContain(type);
    expect(response.headers.get("cache-control")).toBe(ASSET_CACHE);
    for (const [name, value] of Object.entries(HEADERS)) expect(response.headers.get(name), name).toBe(value);
  });

  it("the built page has no inline script or style, and the Worker no code", () => {
    const html = readFileSync(new URL("dist/index.html", APP), "utf8");
    for (const [, attributes = ""] of html.matchAll(/<script([^>]*)>/g)) expect(attributes).toContain("src=");
    expect(html).not.toMatch(/<style|\sstyle=/);
    // A `main` script would answer requests itself, and _headers doesn't apply to its responses.
    expect(readFileSync(new URL("wrangler.jsonc", APP), "utf8")).not.toMatch(/"main"\s*:/);
  });
});

/** A loaded page with its CSP violations and its requests to other origins recorded. */
interface Watched {
  readonly page: Page;
  /** CSP violations so far. */
  violations(): Promise<string[]>;
  /** Requests to an origin other than the app's. */
  readonly foreign: string[];
}

describe(`opening local files in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    const type = BROWSERS[BROWSER];
    if (type === undefined) throw new Error(`LAMINA_PDF_E2E_BROWSER: unknown browser ${BROWSER}`);
    browser = await type.launch(BROWSER === "firefox" ? { env: firefoxEnv(APP) } : {});
  });

  afterAll(async () => {
    await browser?.close();
  });

  /** Opens the app at `path`, after `init` (an init script: runs before the page's own). */
  async function load(path = "/", init?: { script: (arg: number[]) => void; arg: number[] }): Promise<Watched> {
    // The server's certificate is self-signed (local-tls.ts).
    const page = await browser.newPage({ ignoreHTTPSErrors: true });
    const foreign: string[] = [];
    page.on("request", (request) => {
      const url = new URL(request.url());
      if (url.protocol.startsWith("http") && url.origin !== origin) foreign.push(url.href);
    });
    await page.addInitScript(() => {
      const seen: string[] = [];
      Object.assign(window, { cspViolations: seen });
      document.addEventListener("securitypolicyviolation", (e) => seen.push(`${e.violatedDirective} ${e.blockedURI}`));
    });
    if (init !== undefined) await page.addInitScript(init.script, init.arg);
    await page.goto(new URL(path, origin).href);
    await page.getByRole("heading", { name: "Open a PDF" }).waitFor();
    return {
      page,
      foreign,
      violations: () => page.evaluate(() => (window as unknown as { cspViolations: string[] }).cspViolations),
    };
  }

  /** Waits for the viewer's first canvas and the window title that names `fileName`. */
  async function opened({ page }: Watched, fileName: string): Promise<void> {
    await page.locator(".web-viewer canvas").first().waitFor();
    await expect.poll(() => page.title()).toBe(`${fileName} · Lamina PDF`);
  }

  /** The page made no CSP violation and asked no other origin for anything. */
  async function clean(watched: Watched): Promise<void> {
    expect(await watched.violations()).toEqual([]);
    expect(watched.foreign).toEqual([]);
    await watched.page.close();
  }

  const bytes = (name: string) => [...readFileSync(new URL(name, CORPUS))];

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
    // Without showOpenFilePicker the platform falls back to the input, in every engine.
    const watched = await load("/", { script: () => Reflect.deleteProperty(window, "showOpenFilePicker"), arg: [] });
    const chooser = watched.page.waitForEvent("filechooser");
    await watched.page.getByRole("button", { name: "Choose a file" }).click();
    await (await chooser).setFiles(new URL("hello.pdf", CORPUS).pathname);
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
