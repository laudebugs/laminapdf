// "Edit and run" against `wrangler dev` (e2e/server.ts), in one browser per run
// (LAMINA_PDF_E2E_BROWSER): the runner's own headers and the pages' unchanged ones; each of the six
// examples run unchanged gives its golden's bytes; an edit changes the PDF; a mistake shows its code
// and hint. Nothing loads before the button is pressed, nothing from another origin, and no CSP
// violation is reported.

import { readFileSync } from "node:fs";
import { env } from "node:process";
import { firefoxEnv } from "@laminapdf/test-utils/firefox";
import { getLocal } from "@laminapdf/test-utils/wrangler-dev";
import { type Browser, type BrowserType, chromium, firefox, type Page, webkit } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";

const DOCS = new URL("../", import.meta.url);
const ROOT = new URL("../../", DOCS);
const BROWSERS: Record<string, BrowserType> = { chromium, firefox, webkit };
const BROWSER = env.LAMINA_PDF_E2E_BROWSER ?? "chromium";
const origin = inject("origin");

/** The six runnable examples: their recipe page and their golden. */
const EXAMPLES = [
  ["hello", "hello"],
  ["invoice", "invoice"],
  ["images-and-vectors", "images_and_vectors"],
  ["links-and-outlines", "links_and_outlines"],
  ["element-flow", "element_flow"],
  ["pdf-ua-report", "pdf_ua_report"],
] as const;

/** A page with what it reported recorded: console errors (CSP violations among them) and other origins. */
interface Watched {
  readonly page: Page;
  readonly errors: string[];
  readonly foreign: string[];
  /** Every URL requested since the page loaded. */
  readonly requests: string[];
}

let browser: Browser;

beforeAll(async () => {
  const type = BROWSERS[BROWSER];
  if (type === undefined) throw new Error(`LAMINA_PDF_E2E_BROWSER: unknown browser ${BROWSER}`);
  browser = await type.launch(BROWSER === "firefox" ? { env: firefoxEnv(DOCS) } : {});
});

afterAll(async () => {
  await browser?.close();
});

/** Opens a recipe page on its Result tab, before "Edit and run" is pressed. */
async function recipe(slug: string): Promise<Watched> {
  // The server's certificate is self-signed (@laminapdf/test-utils/wrangler-dev).
  const context = await browser.newContext({ acceptDownloads: true, ignoreHTTPSErrors: true });
  const page = await context.newPage();
  const watched: Watched = { page, errors: [], foreign: [], requests: [] };
  page.on("console", (message) => {
    if (message.type() === "error") watched.errors.push(message.text());
  });
  page.on("request", (request) => {
    watched.requests.push(request.url());
    if (new URL(request.url()).origin !== origin && !request.url().startsWith("data:"))
      watched.foreign.push(request.url());
  });
  await page.goto(new URL(`/recipes/${slug}/`, origin).href);
  await page.getByRole("tab", { name: "Result" }).click();
  return watched;
}

/** Presses Run and waits for the pages or the error. */
async function run(page: Page): Promise<void> {
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await page.waitForSelector(".run-download:not([hidden]), .run-error:not([hidden])", { timeout: 60_000 });
}

/** The PDF the Download link gives. */
async function downloaded(page: Page): Promise<Buffer> {
  const [download] = await Promise.all([page.waitForEvent("download"), page.locator(".run-download").click()]);
  return readFileSync(await download.path());
}

/** Replaces the editor's text with `text`. */
async function edit(page: Page, text: string): Promise<void> {
  await page.locator(".cm-content").click();
  await page.keyboard.press("ControlOrMeta+a");
  await page.keyboard.insertText(text);
}

describe("headers", () => {
  it("the runner alone may eval", async () => {
    const worker = await getLocal(new URL("/run/worker.js", origin));
    expect(worker.status).toBe(200);
    expect(worker.headers.get("content-security-policy")).toBe(
      "default-src 'none'; script-src 'self' 'wasm-unsafe-eval' 'unsafe-eval'; connect-src 'self'; base-uri 'none'; form-action 'none'; object-src 'none'",
    );
    const page = await getLocal(new URL("/recipes/hello/", origin));
    const policy = page.headers.get("content-security-policy") ?? "";
    expect(policy).toContain("default-src 'none'; script-src 'self' 'wasm-unsafe-eval' 'sha256-");
    expect(policy).not.toContain("'unsafe-eval'");
  });
});

describe(`edit and run in ${BROWSER}`, () => {
  it.each(EXAMPLES)("%s run unchanged gives its golden's bytes", async (slug, golden) => {
    const { page, errors, foreign, requests } = await recipe(slug);
    expect(requests.filter((url) => new URL(url).pathname.startsWith("/run/"))).toEqual([]);
    await page.getByRole("button", { name: "Edit and run" }).click();
    await run(page);
    expect(await page.locator(".run-error:not([hidden])").count()).toBe(0);
    expect(await page.locator("canvas.run-page").count()).toBeGreaterThan(0);
    const bytes = await downloaded(page);
    expect(bytes.equals(readFileSync(new URL(`fixtures/golden/examples/${golden}.pdf`, ROOT)))).toBe(true);
    expect(errors).toEqual([]);
    expect(foreign).toEqual([]);
    await page.context().close();
  });

  it("an edit makes a new PDF, and a mistake shows its code and hint", async () => {
    const { page, errors } = await recipe("hello");
    await page.getByRole("button", { name: "Edit and run" }).click();
    const source = (await page.locator("lamina-run").getAttribute("data-source")) ?? "";
    expect(source).toContain('"Hello, world!"');
    await edit(page, source.replace('"Hello, world!"', '"Edited in the docs"'));
    await run(page);
    const golden = readFileSync(new URL("fixtures/golden/examples/hello.pdf", ROOT));
    expect((await downloaded(page)).equals(golden)).toBe(false);

    await edit(page, source.replace('b.page("A4")', "b.page([0, 842])"));
    await run(page);
    const problem = page.locator(".run-error");
    await expect.poll(() => problem.isVisible()).toBe(true);
    expect(await problem.locator("strong").innerText()).toBe("invalid-geometry");
    expect(await problem.locator(".run-hint").count()).toBe(1);

    await page.getByRole("button", { name: "Reset" }).click();
    await run(page);
    expect((await downloaded(page)).equals(golden)).toBe(true);
    expect(errors).toEqual([]);
    await page.context().close();
  });
});
