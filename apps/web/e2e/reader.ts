// What both browser suites check, wherever the reader is served: the e2e suite against `wrangler
// dev` (reader.e2e.test.ts) and the smoke test against a deployed preview, staging or production
// (smoke.test.ts). ADR 0011 §4's headers, a page load that records CSP violations and requests to
// other origins, and the checks that a file opened cleanly.

import { readFileSync } from "node:fs";
import axe from "axe-core";
import type { Browser, Page } from "playwright";
import { expect } from "vitest";
import { BROWSER } from "./browser.ts";

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
 * screen shows (unless `options.home` is false). `options` are otherwise the page's: the e2e
 * suite's local server needs `ignoreHTTPSErrors` for its self-signed certificate, while a
 * deployed reader's certificate is checked; a device profile (`devices["iPhone 15"]`) gives the
 * page a phone's viewport, touch and user agent.
 */
export async function load(
  browser: Browser,
  origin: string,
  path = "/",
  init?: InitScript,
  options: Parameters<Browser["newPage"]>[0] & { home?: boolean } = {},
): Promise<Watched> {
  const { home = true, ...pageOptions } = options;
  const page = await browser.newPage(pageOptions);
  const foreign: string[] = [];
  page.on("request", (request) => {
    const url = new URL(request.url());
    if (url.protocol.startsWith("http") && url.origin !== new URL(origin).origin) foreign.push(url.href);
  });
  await page.addInitScript(() => {
    const seen: { text: string; target: string; at: string }[] = [];
    Object.assign(window, { cspViolations: seen });
    document.addEventListener("securitypolicyviolation", (e) =>
      seen.push({
        text: `${e.violatedDirective} ${e.blockedURI}`,
        target: e.target instanceof Element ? e.target.tagName : "",
        at: `${e.sourceFile}:${e.lineNumber}:${e.columnNumber}`,
      }),
    );
  });
  if (init !== undefined) await page.addInitScript(init.script, init.arg);
  await page.goto(new URL(path, origin).href);
  // A `#url=` link opens a remote file at once, so its page may never show the home screen.
  if (home) await page.getByRole("heading", { name: "Open a PDF" }).waitFor();
  return {
    page,
    foreign,
    violations: async () => {
      const seen = await page.evaluate(
        () => (window as unknown as { cspViolations: { text: string; target: string; at: string }[] }).cspViolations,
      );
      if (BROWSER !== "webkit") return seen.map((v) => v.text);
      // Playwright's WebKit reports every native <select> as an inline style under `style-src
      // 'self'`: the engine's own styles for the control (a page with a bare <select> does the
      // same). It reports at the script's `createElement("select")`, aimed at the select or, once
      // that is detached, at the document; those are left out, and everything else must be clean.
      const sources = new Map<string, boolean>();
      for (const { at } of seen) if (!sources.has(at)) sources.set(at, await createsSelect(page, at));
      return seen.filter((v) => !sources.get(v.at)).map((v) => v.text);
    },
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

/** axe's serious and critical violations on the page now, as `id: targets`, without the rules `off`. */
export async function serious(page: Page, off: readonly string[] = []): Promise<string[]> {
  await page.evaluate(axe.source);
  return page.evaluate(async (off) => {
    const run = (window as unknown as { axe: typeof axe }).axe;
    const result = await run.run(document, {
      runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"] },
      rules: Object.fromEntries(off.map((id) => [id, { enabled: false }])),
    });
    return result.violations
      .filter((v) => v.impact === "serious" || v.impact === "critical")
      .map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`);
  }, off);
}

/** Whether script position `at` (`url:line:column`) is a `createElement("select")` call. */
async function createsSelect(page: Page, at: string): Promise<boolean> {
  const m = /^(.*):(\d+):(\d+)$/.exec(at);
  if (m === null || !(m[1] ?? "").startsWith("http")) return false;
  const [, url = "", line, column] = m;
  const source = await page.evaluate(async (u) => (await fetch(u)).text(), url);
  const text = source.split("\n")[Number(line) - 1] ?? "";
  const col = Number(column) - 1;
  return text.slice(0, col).endsWith("createElement") && /^\(\s*[`'"]select[`'"]/.test(text.slice(col));
}
