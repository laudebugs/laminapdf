// The object inspector in the built reader (todo E3g, Paper O15), in this run's browser:
// - More › Inspect objects puts the inspector over the pages, with the file's version and counts;
//   Go to object shows a page dictionary with its keys, what refers to it and its content stream,
//   decoded and in hex; a reference goes to its object; Show page goes back to the pages;
// - the Fonts, Colour and Conformance tabs list the fonts, the output intent's profile and the
//   claims, each "Claimed, not verified";
// - a stream that decodes past the engine's limit is refused and shown raw, never decoded.
// No CSP violation or other origin throughout.

import { readFileSync } from "node:fs";
import { deflateSync } from "node:zlib";
import { Document, init, loadExtract } from "@laminapdf/sdk";
import type { Browser, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, type Watched } from "./reader.ts";

const origin = inject("origin");

/** No pickers: Choose a file is the file input, and Save raw stream… a download, in every engine. */
const NO_PICKERS = {
  script: () => {
    Reflect.deleteProperty(window, "showOpenFilePicker");
    Reflect.deleteProperty(window, "showSaveFilePicker");
  },
  arg: [],
};

/** A PDF/A-2b file with an output intent, embedded fonts and an XMP claim. */
const ARCHIVE = readFileSync(new URL("../../../fixtures/golden/examples/pdf_a_archive-2b.pdf", import.meta.url));

/** How long the worker may take to read the document for the inspector: WebKit in CI is slow. */
const READ = 30_000;

/** A one-page file whose content stream (object 4) inflates past the engine's 64 MiB limit. */
function bomb(): Buffer {
  const data = deflateSync(Buffer.alloc(64 * 1024 * 1024 + 1));
  const head =
    "%PDF-1.7\n1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n" +
    "2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n" +
    "3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R >> endobj\n" +
    `4 0 obj << /Length ${data.length} /Filter /FlateDecode >>\nstream\n`;
  return Buffer.concat([
    Buffer.from(head),
    data,
    Buffer.from("\nendstream\nendobj\ntrailer << /Root 1 0 R >>\n%%EOF\n"),
  ]);
}

/** Opens `bytes` as `name` with Choose a file; the reader may not draw a page it can't decode. */
async function openBytes(page: Page, name: string, bytes: Buffer): Promise<void> {
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Choose a file" }).click();
  await (await chooser).setFiles({ name, mimeType: "application/pdf", buffer: bytes });
  await page.getByRole("button", { name: "More" }).waitFor();
  await page.locator(".web-viewer").waitFor({ state: "attached" });
}

/** More › Inspect objects, once the document's later parts have loaded; waits for its counts. */
async function inspect(page: Page): Promise<void> {
  await page.getByRole("button", { name: "More" }).click();
  await page.getByRole("menuitem", { name: "Inspect objects" }).click({ timeout: READ });
  await page.getByText(/^PDF \d\.\d · \d+ objects · \d+ revisions?$/).waitFor({ timeout: READ });
}

/** Goes to object `num` with the rail's Go to object field. */
async function goTo(page: Page, num: number): Promise<void> {
  const field = page.getByLabel("Go to object");
  await field.fill(String(num));
  await field.press("Enter");
}

describe(`object inspector in ${BROWSER}`, () => {
  let browser: Browser;

  beforeAll(async () => {
    await init();
    await loadExtract();
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  /** The reader in a wide window: the inspector isn't offered on a phone. */
  function reader(): Promise<Watched> {
    return load(browser, origin, "/", NO_PICKERS, { ignoreHTTPSErrors: true, viewport: { width: 1440, height: 900 } });
  }

  it("[cap J.09 web read] browses a page's object, what refers to it and its content stream", async () => {
    using doc = Document.open(ARCHIVE);
    const pageObject = doc.objects().find((o) => o.typeName === "Page");
    if (pageObject === undefined) throw new Error("the fixture has a page");
    const watched = await reader();
    const { page } = watched;
    await openBytes(page, "archive.pdf", ARCHIVE);
    await page.locator(".web-viewer canvas").first().waitFor({ timeout: READ });
    await inspect(page);
    // React Aria's tree is a treegrid: rows of one cell.
    await page
      .getByRole("treegrid", { name: "Object tree" })
      .getByRole("row", { name: /Trailer/ })
      .first()
      .waitFor();

    await goTo(page, pageObject.num);
    const title = page.getByRole("heading", { level: 2 });
    await title.filter({ hasText: `Object ${pageObject.num} · Page dictionary` }).waitFor({ timeout: READ });
    const keys = page.getByRole("table", { name: `Keys of object ${pageObject.num}` });
    await keys.getByRole("cell", { name: "/MediaBox" }).waitFor();
    await page.getByRole("heading", { name: "Referenced by" }).waitFor();
    // The content stream, decoded: content operators.
    const block = page.getByRole("region", { name: /^Stream \d+, Decoded/ });
    await block.waitFor({ timeout: READ });
    expect(await block.textContent()).toMatch(/\b(BT|re|cm)\b/);
    await page.getByRole("button", { name: "Hex", exact: true }).click();
    await page
      .getByRole("region", { name: /^Stream \d+, Hex/ })
      .getByText(/^00000000 /)
      .waitFor({ timeout: READ });

    // A reference goes to its object: /Parent is the page tree.
    await keys
      .getByRole("row", { name: /\/Parent/ })
      .getByRole("button")
      .click();
    await title.filter({ hasText: "· Page tree" }).waitFor({ timeout: READ });

    // Back to pages shows them again.
    await page.getByRole("button", { name: "Back to pages" }).click();
    await page.locator(".web-viewer canvas").first().waitFor({ state: "visible" });
    expect(await page.getByRole("treegrid", { name: "Object tree" }).count()).toBe(0);
    await clean(watched);
  });

  it("[cap J.07 web read] [cap J.08 web read] lists the fonts, the colour profiles and the claims", async () => {
    const watched = await reader();
    const { page } = watched;
    await openBytes(page, "archive.pdf", ARCHIVE);
    await page.locator(".web-viewer canvas").first().waitFor({ timeout: READ });
    await inspect(page);

    await page.getByRole("tab", { name: "Fonts" }).click();
    const fonts = page.getByRole("table", { name: "Fonts" });
    await fonts.getByRole("row").nth(1).waitFor();
    expect(await fonts.getByRole("cell", { name: /^Subset$|^Yes$/ }).count()).toBeGreaterThan(0);

    await page.getByRole("tab", { name: "Colour" }).click();
    await page
      .getByRole("table", { name: "Output intents" })
      .getByRole("cell", { name: "GTS_PDFA1", exact: true })
      .waitFor();
    await page.getByRole("table", { name: "ICC profiles" }).getByRole("cell", { name: "RGB", exact: true }).waitFor();

    await page.getByRole("tab", { name: "Conformance" }).click();
    await page.getByText("PDF/A-2b", { exact: true }).waitFor();
    await page.getByText("Claimed, not verified").first().waitFor();
    await clean(watched);
  });

  it("[cap J.07 web read] opens on the claims from Document properties' Inspect…", async () => {
    const watched = await reader();
    const { page } = watched;
    await openBytes(page, "archive.pdf", ARCHIVE);
    await page.locator(".web-viewer canvas").first().waitFor({ timeout: READ });
    await page.getByRole("button", { name: "More" }).click();
    await page.getByRole("menuitem", { name: "Document properties…" }).click({ timeout: READ });
    const dialog = page.getByRole("dialog", { name: "Document properties" });
    await dialog.getByRole("tab", { name: "File" }).click();
    await dialog.getByRole("button", { name: "Inspect…" }).click();
    await page.getByRole("tab", { name: "Conformance", selected: true }).waitFor({ timeout: READ });
    await page.getByText("PDF/A-2b", { exact: true }).waitFor({ timeout: READ });
    await clean(watched);
  });

  it("[cap J.09 web read] decodes a decompression bomb only as far as it shows", async () => {
    const watched = await reader();
    const { page } = watched;
    await openBytes(page, "bomb.pdf", bomb());
    await inspect(page);
    await goTo(page, 4);
    // The engine decodes the first megabyte and one byte more, never the 64 MiB the stream holds.
    await page.getByText("This stream is longer than 1 MB.", { exact: false }).waitFor({ timeout: READ });
    await page.getByText(/more than 1,048,576 bytes decoded/).waitFor();
    await page.getByRole("button", { name: "Save raw stream…" }).waitFor();
    await page.getByRole("region", { name: /^Stream 4, Decoded/ }).waitFor({ timeout: READ });
    await clean(watched);
  });
});
