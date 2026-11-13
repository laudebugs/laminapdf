// "Edit and run" against `wrangler dev` (e2e/server.ts), in one browser per run
// (LAMINA_PDF_E2E_BROWSER): the runner's own headers and the pages' unchanged ones; each runnable
// example (the six recipes and "Your first PDF"'s Node app) run unchanged gives its golden's bytes; an edit changes the PDF; a mistake shows its code
// and hint. Nothing loads before the button is pressed, nothing from another origin, and no CSP
// violation is reported.

import { readFileSync } from "node:fs";
import { env } from "node:process";
import { firefoxEnv } from "@laminapdf/test-utils/firefox";
import { withoutSelectErrors } from "@laminapdf/test-utils/webkit-select";
import { getLocal } from "@laminapdf/test-utils/wrangler-dev";
import { type Browser, type BrowserType, chromium, firefox, type Page, webkit } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";

const DOCS = new URL("../", import.meta.url);
const ROOT = new URL("../../", DOCS);
const BROWSERS: Record<string, BrowserType> = { chromium, firefox, webkit };
const BROWSER = env.LAMINA_PDF_E2E_BROWSER ?? "chromium";
const origin = inject("origin");
const ca = inject("ca");

/** The runnable examples: a page that shows each, and its golden. */
const EXAMPLES = [
  ["/recipes/hello/", "hello"],
  ["/recipes/invoice/", "invoice"],
  ["/recipes/images-and-vectors/", "images_and_vectors"],
  ["/recipes/links-and-outlines/", "links_and_outlines"],
  ["/recipes/element-flow/", "element_flow"],
  ["/recipes/pdf-ua-report/", "pdf_ua_report"],
  ["/start/first-pdf/", "hello"],
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

/** Opens a page with an example, before "Edit and run" (under the example's first page) is pressed. */
async function recipe(path: string): Promise<Watched> {
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
  await page.goto(new URL(path, origin).href);
  return watched;
}

/** Presses Run and waits until the run ends (Run is enabled again), with its pages or its error. */
async function run(page: Page, timeout = 60_000): Promise<void> {
  const button = page.getByRole("button", { name: "Run", exact: true });
  await button.click();
  await page.waitForFunction(() => document.querySelector<HTMLButtonElement>(".run-run")?.disabled === false, null, {
    timeout,
  });
}

/** The PDF the Download button saves. */
async function downloaded(page: Page): Promise<Buffer> {
  const [download] = await Promise.all([page.waitForEvent("download"), page.locator(".run-download").click()]);
  return readFileSync(await download.path());
}

/**
 * The editor's whole text. CodeMirror draws only the lines in view, so the text comes from its
 * state, reached as `EditorView.findFromDOM` reaches it (the content element's `cmTile.view`), which
 * the page doesn't export.
 */
async function editorText(page: Page): Promise<string> {
  return page.evaluate(() => {
    const content = document.querySelector(".run-editor")?.shadowRoot?.querySelector(".cm-content");
    const view = (content as { cmTile?: { view?: { state: { doc: { toString(): string } } } } } | null)?.cmTile?.view;
    if (view === undefined) throw new Error("the editor's view isn't on its content element");
    return view.state.doc.toString();
  });
}

/** Replaces the editor's text with `text`. */
async function edit(page: Page, text: string): Promise<void> {
  await page.locator(".cm-content").click();
  await page.keyboard.press("ControlOrMeta+a");
  await page.keyboard.insertText(text);
}

describe("headers", () => {
  it("the runner alone may eval", async () => {
    const worker = await getLocal(new URL("/run/worker.js", origin), ca);
    expect(worker.status).toBe(200);
    expect(worker.headers.get("content-security-policy")).toBe(
      "default-src 'none'; script-src 'self' 'wasm-unsafe-eval' 'unsafe-eval'; connect-src 'self'; base-uri 'none'; form-action 'none'; object-src 'none'",
    );
    const page = await getLocal(new URL("/recipes/hello/", origin), ca);
    const policy = page.headers.get("content-security-policy") ?? "";
    expect(policy).toContain("default-src 'none'; script-src 'self' 'wasm-unsafe-eval' 'sha256-");
    expect(policy).not.toContain("'unsafe-eval'");
  });
});

describe(`edit and run in ${BROWSER}`, () => {
  it.each(EXAMPLES)("%s run unchanged gives its golden's bytes", async (path, golden) => {
    const { page, errors, foreign, requests } = await recipe(path);
    expect(requests.filter((url) => new URL(url).pathname.startsWith("/run/"))).toEqual([]);
    await page.getByRole("button", { name: "Edit and run" }).click();
    await run(page);
    expect(await page.locator(".run-error:not([hidden])").count()).toBe(0);
    expect(await page.locator("canvas.run-page").count()).toBeGreaterThan(0);
    // The run's page takes the place of the first page's picture.
    expect(await page.locator(".example-first").isHidden()).toBe(true);
    const [download] = await Promise.all([page.waitForEvent("download"), page.locator(".run-download").click()]);
    expect(download.suggestedFilename()).toBe(`${golden}.pdf`);
    const bytes = readFileSync(await download.path());
    expect(bytes.equals(readFileSync(new URL(`fixtures/golden/examples/${golden}.pdf`, ROOT)))).toBe(true);
    expect(withoutSelectErrors(BROWSER, errors, await page.locator("select").count())).toEqual([]);
    expect(foreign).toEqual([]);
    await page.context().close();
  });

  it("an edit makes a new PDF, and a mistake shows its code and hint", async () => {
    const { page, errors } = await recipe("/recipes/hello/");
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
    expect(await problem.isVisible()).toBe(true);
    expect(await problem.locator("strong").innerText()).toBe("invalid-geometry");
    expect(await problem.locator(".run-hint").count()).toBe(1);

    // An edit that never finishes is stopped, and the next run starts a new runner.
    await edit(
      page,
      source.replace("using b = new Builder(options);", "using b = new Builder(options);\n  for (;;) {}"),
    );
    await page.getByRole("button", { name: "Run", exact: true }).click();
    await expect.poll(() => problem.locator("strong").innerText(), { timeout: 45_000 }).toBe("timeout");

    await page.getByRole("button", { name: "Reset" }).click();
    await run(page);
    expect((await downloaded(page)).equals(golden)).toBe(true);
    expect(withoutSelectErrors(BROWSER, errors, await page.locator("select").count())).toEqual([]);
    await page.context().close();
  });

  it("Format (todo Q4r) puts back the example's formatting, loading Prettier only then, and leaves a mistake alone", async () => {
    const { page, errors, foreign, requests } = await recipe("/recipes/images-and-vectors/");
    await page.getByRole("button", { name: "Edit and run" }).click();
    const source = (await page.locator("lamina-run").getAttribute("data-source")) ?? "";
    const prettier = () => requests.filter((url) => /\/(standalone|typescript|estree)\.[^/]*\.js$/.test(url));
    expect(prettier()).toEqual([]);
    // Every line's indentation taken out: Prettier at the examples' settings gives the file back.
    await edit(page, source.replace(/\n +/g, "\n"));
    await page.getByRole("button", { name: "Format" }).click();
    await expect.poll(() => page.locator(".run-status").textContent()).toBe("Formatted.");
    expect(prettier().length).toBeGreaterThan(0);
    expect(await editorText(page)).toBe(source);
    await run(page);
    expect(
      (await downloaded(page)).equals(readFileSync(new URL("fixtures/golden/examples/images_and_vectors.pdf", ROOT))),
    ).toBe(true);

    await edit(page, "const x = ;");
    await page.getByRole("button", { name: "Format" }).click();
    const problem = page.locator(".run-error");
    await expect.poll(() => problem.isVisible()).toBe(true);
    expect(await problem.locator("strong").textContent()).toBe("can't format");
    expect(await editorText(page)).toBe("const x = ;");
    expect(withoutSelectErrors(BROWSER, errors, await page.locator("select").count())).toEqual([]);
    expect(foreign).toEqual([]);
    await page.context().close();
  });
});
