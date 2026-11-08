// What both browser suites check, wherever the reader is served: the e2e suite against `wrangler
// dev` (reader.e2e.test.ts) and the smoke test against a deployed preview, staging or production
// (smoke.test.ts). ADR 0011 §4's headers, a page load that records CSP violations and requests to
// other origins, and the checks that a file opened cleanly.

import { readFileSync } from "node:fs";
import type { Browser, Page } from "playwright";
import { expect } from "vitest";

/** The test corpus, whose hello.pdf both suites open. */
const CORPUS = new URL("../../../fixtures/corpus/", import.meta.url);

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
/** HTML is revalidated on every load, so a release reaches the next load. */
export const HTML_CACHE = "public, max-age=0, must-revalidate";
/** Vite's hashed files under /assets/ never change. */
export const ASSET_CACHE = "public, max-age=31536000, immutable";

/** Fails unless `response` carries every one of ADR 0011 §4's headers with its exact value. */
export function expectHeaders(response: Response): void {
  for (const [name, value] of Object.entries(HEADERS)) expect(response.headers.get(name), name).toBe(value);
}

/** `GET path` from `origin` without following redirects. */
export function get(origin: string, path: string): Promise<Response> {
  return fetch(new URL(path, origin), { redirect: "manual" });
}

/** A loaded page with its CSP violations and its requests to other origins recorded. */
export interface Watched {
  readonly page: Page;
  /** CSP violations so far. */
  violations(): Promise<string[]>;
  /** Requests to an origin other than the app's. */
  readonly foreign: string[];
}

/** An init script (runs before the page's own) and its argument. */
export interface InitScript {
  readonly script: (arg: number[]) => void;
  readonly arg: number[];
}

/**
 * Opens the app at `path` on `origin` in a new page of `browser`, after `init`, until its home
 * screen shows. `options` are the page's: the e2e suite's local server needs `ignoreHTTPSErrors`
 * for its self-signed certificate, while a deployed reader's certificate is checked.
 */
export async function load(
  browser: Browser,
  origin: string,
  path = "/",
  init?: InitScript,
  options: { ignoreHTTPSErrors?: boolean } = {},
): Promise<Watched> {
  const page = await browser.newPage(options);
  const foreign: string[] = [];
  page.on("request", (request) => {
    const url = new URL(request.url());
    if (url.protocol.startsWith("http") && url.origin !== new URL(origin).origin) foreign.push(url.href);
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

/** Without showOpenFilePicker the platform falls back to the file input, in every engine. */
export const WITHOUT_PICKER: InitScript = {
  script: () => Reflect.deleteProperty(window, "showOpenFilePicker"),
  arg: [],
};

/** Chooses corpus file `name` with Choose a file, on a page loaded with {@link WITHOUT_PICKER}. */
export async function chooseFile({ page }: Watched, name: string): Promise<void> {
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Choose a file" }).click();
  await (await chooser).setFiles(new URL(name, CORPUS).pathname);
}

/** Waits for the viewer's first canvas and the window title that names `fileName`. */
export async function opened({ page }: Watched, fileName: string): Promise<void> {
  await page.locator(".web-viewer canvas").first().waitFor();
  await expect.poll(() => page.title()).toBe(`${fileName} · Lamina PDF`);
}

/** The page made no CSP violation and asked no other origin for anything; then it closes. */
export async function clean(watched: Watched): Promise<void> {
  expect(await watched.violations()).toEqual([]);
  expect(watched.foreign).toEqual([]);
  await watched.page.close();
}

/** Corpus file `name`'s bytes, as an array a page can take. */
export const bytes = (name: string): number[] => [...readFileSync(new URL(name, CORPUS))];
