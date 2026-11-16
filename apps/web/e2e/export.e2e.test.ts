// Export (Paper O16) and From images (O07) in the built reader, in this run's browser, as a person
// uses them: More › Export… on a three-page file with text, a picture, a link and bookmarks;
// pages as PNG in a zip, text as JSON with positions, the file's picture, and its links and
// bookmarks, each checked against what it should hold (the JSON against its committed schema);
// Cancel stops an export; and Organise's Insert › From images makes a new PDF of two pictures.
// Saves download in every engine (no save picker), and nothing breaks the CSP or reaches another
// origin.

import { readFileSync } from "node:fs";
import { crc32, deflateSync } from "node:zlib";
import { Builder, Document, Font, Image, init } from "@laminapdf/sdk";
import { Ajv2020 } from "ajv/dist/2020.js";
import type { Browser, Download, Page } from "playwright";
import { afterAll, beforeAll, describe, expect, inject, it } from "vitest";
import { BROWSER, launch } from "./browser.ts";
import { clean, load, serious, type Watched } from "./reader.ts";

const origin = inject("origin");
const SCHEMAS = new URL("../../../crates/", import.meta.url);

/** No open or save pickers: files come through the file input and saves download, in every engine. */
const NO_PICKERS = {
  script: () => {
    Reflect.deleteProperty(window, "showOpenFilePicker");
    Reflect.deleteProperty(window, "showSaveFilePicker");
    Reflect.deleteProperty(window, "showDirectoryPicker");
  },
  arg: [],
};

/** As {@link NO_PICKERS}, but a folder picker stands in for Chromium's, keeping what's written on `window.written`. */
const FOLDER = {
  script: () => {
    Reflect.deleteProperty(window, "showOpenFilePicker");
    Reflect.deleteProperty(window, "showSaveFilePicker");
    const written: Record<string, number> = {};
    const folder = {
      kind: "directory",
      name: "Q3 exports",
      async getFileHandle(name: string, options?: { create?: boolean }) {
        if (!(name in written) && options?.create !== true) throw new DOMException("missing", "NotFoundError");
        return {
          createWritable: async () => ({
            write: async (chunk: Uint8Array) => {
              written[name] = chunk.length;
            },
            close: async () => undefined,
            abort: async () => undefined,
          }),
        };
      },
    };
    Object.assign(window, { written, showDirectoryPicker: async () => folder });
  },
  arg: [],
};

/** A 40 × 40 picture in one colour, as RGBA. */
const square = (r: number, g: number, b: number) =>
  Image.rgba(
    40,
    40,
    new Uint8Array(40 * 40 * 4).map((_, i) => [r, g, b, 255][i % 4] ?? 255),
  );

/** Three pages of text; the first with a picture and a link; two bookmarks. */
function report(): Buffer {
  using b = new Builder();
  const font = Font.standard("Helvetica");
  for (let i = 1; i <= 3; i++) {
    const page = b.page([612, 792]);
    page.surface().text(`Quarterly report page ${i}`, { font, size: 18, at: [72, 100] });
    if (i === 1) {
      using picture = square(30, 120, 200);
      page.surface().image(picture, [72, 200, 120, 120]);
      page.surface().link([72, 400, 200, 20], { type: "uri", uri: "https://example.com/" });
    }
    page.finish();
  }
  b.outline([
    { title: "Summary", target: { type: "page", index: 0 } },
    { title: "Figures", target: { type: "page", index: 2 } },
  ]);
  using doc = b.finish();
  return Buffer.from(doc.save());
}

/** `count` pages of text: an export long enough to cancel part way. */
function long(count: number): Buffer {
  using b = new Builder();
  const font = Font.standard("Helvetica");
  for (let i = 1; i <= count; i++) {
    const page = b.page([612, 792]);
    page.surface().text(`Appendix page ${i}`, { font, size: 18, at: [72, 100] });
    page.finish();
  }
  using doc = b.finish();
  return Buffer.from(doc.save());
}

/** A 2 × 2 PNG in one colour, for From images. */
function png(r: number, g: number, b: number): Buffer {
  const chunk = (type: string, data: Buffer) => {
    const length = Buffer.alloc(4);
    length.writeUInt32BE(data.length);
    const body = Buffer.concat([Buffer.from(type, "latin1"), data]);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(crc32(body));
    return Buffer.concat([length, body, crc]);
  };
  const header = Buffer.alloc(13);
  header.writeUInt32BE(2, 0);
  header.writeUInt32BE(2, 4);
  header.set([8, 2, 0, 0, 0], 8);
  const row = [0, r, g, b, r, g, b];
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", header),
    chunk("IDAT", deflateSync(Buffer.from([...row, ...row]))),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/** A stored zip's files, read from their local headers. */
function unzip(zip: Buffer): Map<string, Buffer> {
  const files = new Map<string, Buffer>();
  let at = 0;
  while (zip.readUInt32LE(at) === 0x04034b50) {
    const size = zip.readUInt32LE(at + 18);
    const nameLength = zip.readUInt16LE(at + 26);
    const extraLength = zip.readUInt16LE(at + 28);
    const name = zip.toString("utf8", at + 30, at + 30 + nameLength);
    const start = at + 30 + nameLength + extraLength;
    files.set(name, zip.subarray(start, start + size));
    at = start + size;
  }
  return files;
}

/** Whether `json` holds to the schema in `path` under crates/. */
function valid(json: unknown, path: string): { ok: boolean; errors: string } {
  // The schemas are JSON Schema 2020-12; formats such as `uint` are notes for readers, not checks.
  const ajv = new Ajv2020({ strict: false, allErrors: true, validateFormats: false });
  const validate = ajv.compile(JSON.parse(readFileSync(new URL(path, SCHEMAS), "utf8")));
  return { ok: validate(json), errors: JSON.stringify(validate.errors) };
}

const PNG_SIGNATURE = Buffer.from([0x89, 0x50, 0x4e, 0x47]);

/** Opens `bytes` as "Q3 report.pdf" and waits for its first page. */
async function open(page: Page, bytes: Buffer): Promise<void> {
  const chooser = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Choose a file" }).click();
  await (await chooser).setFiles({ name: "Q3 report.pdf", mimeType: "application/pdf", buffer: bytes });
  await page.locator(".web-viewer canvas").first().waitFor({ timeout: 30_000 });
}

/** More › Export…, then `kind`'s card, once the file's contents are counted. */
async function exportDialog(page: Page, kind: string): Promise<void> {
  await page.getByRole("button", { name: "More", exact: true }).first().click();
  await page.getByRole("menuitem", { name: "Export…" }).click();
  const dialog = page.getByRole("dialog", { name: "Export" });
  await dialog.waitFor();
  await expect.poll(() => dialog.getByRole("radio", { name: kind }).isEnabled()).toBe(true);
  await pick(page, kind);
}

/** Chooses the export dialog's option `label` by its words, as a person clicks it. */
async function pick(page: Page, label: string | RegExp): Promise<void> {
  const dialog = page.getByRole("dialog", { name: "Export" });
  await dialog
    .getByText(label, { exact: typeof label === "string" })
    .first()
    .click();
  await expect.poll(() => dialog.getByRole("radio", { name: label, exact: true }).isChecked()).toBe(true);
}

/** Presses the dialog's primary button `name` and waits for the download it starts. */
async function exported(page: Page, name: string): Promise<{ download: Download; bytes: Buffer }> {
  const download = page.waitForEvent("download", { timeout: 60_000 });
  await page.getByRole("dialog", { name: "Export" }).getByRole("button", { name }).click();
  const done = await download;
  return { download: done, bytes: readFileSync(await done.path()) };
}

describe(`export in ${BROWSER}`, () => {
  let browser: Browser;
  let watched: Watched;

  beforeAll(async () => {
    await init({ text: true });
    browser = await launch();
  });

  afterAll(async () => {
    await browser?.close();
  });

  async function opened(): Promise<Page> {
    watched = await load(browser, origin, "/", NO_PICKERS, { ignoreHTTPSErrors: true });
    await open(watched.page, report());
    return watched.page;
  }

  it("[cap H.06 web read] [cap H.11 web read] exports pages as PNG into one zip, a file a page", async () => {
    const page = await opened();
    await exportDialog(page, "Pages as images");
    await pick(page, "PNG");
    const { download, bytes } = await exported(page, "Export 3 pages");
    expect(download.suggestedFilename()).toBe("Q3 report-pages.zip");
    const files = unzip(bytes);
    expect([...files.keys()]).toEqual(["Q3 report-page-1.png", "Q3 report-page-2.png", "Q3 report-page-3.png"]);
    for (const file of files.values()) expect(file.subarray(0, 4)).toEqual(PNG_SIGNATURE);
    await clean(watched);
  });

  it("[cap A.10 web read] [cap H.07 web read] exports the text as JSON with positions, as its schema says", async () => {
    const page = await opened();
    await exportDialog(page, "Text");
    await pick(page, "JSON with positions");
    const { download, bytes } = await exported(page, "Export text");
    expect(download.suggestedFilename()).toBe("Q3 report.json");
    const json = JSON.parse(bytes.toString("utf8"));
    const { ok, errors } = valid(json, "laminapdf-extract/schemas/extract.schema.json");
    expect(ok, errors).toBe(true);
    expect(JSON.stringify(json)).toContain("Quarterly report page 3");
    // Positions: every block has its box on the page.
    const blocks = (json as { blocks: { bbox?: number[] }[] }).blocks;
    expect(blocks.length).toBeGreaterThan(0);
    expect(blocks.every((b) => Array.isArray(b.bbox) && b.bbox.length === 4)).toBe(true);
    await clean(watched);
  });

  it("[cap B.03 web read] [cap H.07 web read] exports the file's picture into a zip of pictures", async () => {
    const page = await opened();
    await exportDialog(page, "Pictures in the file");
    const { download, bytes } = await exported(page, "Export pictures");
    expect(download.suggestedFilename()).toBe("Q3 report-pictures.zip");
    const files = [...unzip(bytes)];
    expect(files.map(([name]) => name)).toEqual([expect.stringMatching(/^Q3 report-.*\.png$/)]);
    expect(files[0]?.[1].subarray(0, 4)).toEqual(PNG_SIGNATURE);
    await clean(watched);
  });

  it("[cap H.08 web read] exports the links and the bookmarks as JSON, each as its schema says", async () => {
    const page = await opened();
    await exportDialog(page, "Links and bookmarks");
    await pick(page, "JSON");
    const { download, bytes } = await exported(page, "Export links and bookmarks");
    expect(download.suggestedFilename()).toBe("Q3 report-links-and-bookmarks.zip");
    const files = unzip(bytes);
    const links = JSON.parse(files.get("Q3 report-links.json")?.toString("utf8") ?? "null");
    const bookmarks = JSON.parse(files.get("Q3 report-bookmarks.json")?.toString("utf8") ?? "null");
    expect(valid(links, "laminapdf/schemas/links.schema.json").ok).toBe(true);
    expect(valid(bookmarks, "laminapdf/schemas/outline.schema.json").ok).toBe(true);
    expect(JSON.stringify(links)).toContain("https://example.com/");
    expect(JSON.stringify(bookmarks)).toContain("Figures");
    await clean(watched);
  });

  it("exports the page in view as one JPEG, not zipped", async () => {
    const page = await opened();
    await exportDialog(page, "Pages as images");
    await pick(page, "This page (1)");
    await pick(page, "JPEG");
    const { download, bytes } = await exported(page, "Export 1 page");
    expect(download.suggestedFilename()).toBe("Q3 report.jpg");
    expect([...bytes.subarray(0, 3)]).toEqual([0xff, 0xd8, 0xff]);
    await clean(watched);
  });

  it("exports every page as one multi-page TIFF", async () => {
    const page = await opened();
    await exportDialog(page, "Pages as images");
    await pick(page, "TIFF");
    const { download, bytes } = await exported(page, "Export 3 pages");
    expect(download.suggestedFilename()).toBe("Q3 report.tif");
    expect(bytes.toString("latin1", 0, 2)).toMatch(/^(II|MM)$/);
    await clean(watched);
  });

  it("exports the text as Markdown", async () => {
    const page = await opened();
    await exportDialog(page, "Text");
    await pick(page, "Markdown");
    const { download, bytes } = await exported(page, "Export text");
    expect(download.suggestedFilename()).toBe("Q3 report.md");
    expect(bytes.toString("utf8")).toContain("Quarterly report page 2");
    await clean(watched);
  });

  it("exports the links and the bookmarks as CSV, each with its header row", async () => {
    const page = await opened();
    await exportDialog(page, "Links and bookmarks");
    await pick(page, "CSV");
    const { bytes } = await exported(page, "Export links and bookmarks");
    const files = unzip(bytes);
    expect(files.get("Q3 report-links.csv")?.toString("utf8")).toContain("https://example.com/");
    expect(files.get("Q3 report-bookmarks.csv")?.toString("utf8")).toContain("Figures");
    await clean(watched);
  });

  // A folder only where the browser can write one: Chromium, its picker stubbed (it needs a person).
  it.runIf(BROWSER === "chromium")("writes pages into a folder the person picks, a file a page", async () => {
    watched = await load(browser, origin, "/", FOLDER, { ignoreHTTPSErrors: true });
    const { page } = watched;
    await open(page, report());
    await exportDialog(page, "Pages as images");
    await page.getByRole("dialog", { name: "Export" }).getByRole("button", { name: "Export 3 pages" }).click();
    await expect
      .poll(() => page.evaluate(() => Object.keys((window as unknown as { written: object }).written).sort()))
      .toEqual(["Q3 report-page-1.png", "Q3 report-page-2.png", "Q3 report-page-3.png"]);
    await page.getByText("Exported to Q3 exports").waitFor();
    await clean(watched);
  });

  it("stops an export on Cancel export, and downloads nothing", async () => {
    watched = await load(browser, origin, "/", NO_PICKERS, { ignoreHTTPSErrors: true });
    const { page } = watched;
    await open(page, long(40));
    await exportDialog(page, "Pages as images");
    await page.getByRole("dialog", { name: "Export" }).getByLabel("Resolution").selectOption("300");
    let downloaded = false;
    page.on("download", () => {
      downloaded = true;
    });
    await page.getByRole("dialog", { name: "Export" }).getByRole("button", { name: "Export 40 pages" }).click();
    await page.getByRole("button", { name: "Cancel export" }).click();
    await expect.poll(() => page.getByRole("button", { name: "Cancel export" }).count()).toBe(0);
    await page.waitForTimeout(1_000);
    expect(downloaded).toBe(false);
    await clean(watched);
  });

  it("shows the export dialog with no serious or critical axe violations", async () => {
    const page = await opened();
    await exportDialog(page, "Text");
    expect(await serious(page)).toEqual([]);
    await clean(watched);
  });

  it("[cap H.02 web write] makes a new PDF of two pictures from Organise's Insert menu, a page each", async () => {
    const page = await opened();
    await page.getByRole("button", { name: "Organise", exact: true }).click();
    await page.locator(".paper-page-tile").nth(2).waitFor();
    await page.getByRole("button", { name: "Insert", exact: true }).click();
    const chooser = page.waitForEvent("filechooser");
    await page.getByRole("menuitem", { name: "New PDF from images…" }).click();
    await (await chooser).setFiles([
      { name: "front.png", mimeType: "image/png", buffer: png(200, 30, 30) },
      { name: "back.png", mimeType: "image/png", buffer: png(30, 30, 200) },
    ]);
    const dialog = page.getByRole("dialog", { name: "New PDF from images" });
    const download = page.waitForEvent("download", { timeout: 60_000 });
    await dialog.getByRole("button", { name: "Make a PDF of 2 images" }).click();
    const done = await download;
    expect(done.suggestedFilename()).toBe("front.pdf");
    using made = Document.open(new Uint8Array(readFileSync(await done.path())));
    expect(made.pages.length).toBe(2);
    await clean(watched);
  });
});
