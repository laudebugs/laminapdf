// The export dialog's reads in the worker: the open document's text in each format, its pictures,
// what it has to export, its links and bookmarks as files, images → PDF, and pages drawn with or
// without their annotations. Each reads the document as the edit session leaves it.
import { Builder, Document, Font, Image, init, isLaminaPdfError, type Op } from "@laminapdf/sdk";
import type { ViewerClient } from "@laminapdf/viewer";
import { afterEach, beforeAll, describe, expect, it } from "vitest";
import { read, viewer } from "./helpers.js";

const LINKS_AND_OUTLINES = "fixtures/golden/links-and-outlines.pdf";
const TAGGED = "fixtures/golden/tagged-report.pdf";
const ANNOTATED = "fixtures/appearance/annotated.pdf";

beforeAll(() => init({ text: true }));

let client: ViewerClient | undefined;
afterEach(async () => {
  await client?.close();
  client = undefined;
});

async function opened(bytes: Uint8Array): Promise<ViewerClient> {
  client = await viewer("hayro");
  await client.open(bytes);
  return client;
}

/** Two Letter pages saying `Alpha` and `Bravo`. */
function twoWords(): Uint8Array {
  const b = new Builder({ id: new TextEncoder().encode("lamina-e3j-word1"), producer: "lamina tests" });
  const font = Font.standard("Helvetica");
  for (const word of ["Alpha", "Bravo"]) {
    const page = b.page([612, 792]);
    page
      .surface()
      .fillColor([0, 0, 0])
      .text(word, { font, size: 24, at: [72, 700] });
    page.finish();
  }
  const doc = b.finish();
  const bytes = doc.save();
  doc.free();
  return bytes;
}

/** `width` × `height` pixels of one colour, as a JPEG the browser encodes. */
async function jpeg(width: number, height: number): Promise<Uint8Array<ArrayBuffer>> {
  const canvas = new OffscreenCanvas(width, height);
  const context = canvas.getContext("2d");
  if (context === null) throw new Error("no 2D context");
  context.fillStyle = "#3366cc";
  context.fillRect(0, 0, width, height);
  const blob = await canvas.convertToBlob({ type: "image/jpeg", quality: 0.9 });
  return new Uint8Array(await blob.arrayBuffer());
}

/** One page drawing a 64 × 48 JPEG and a 4 × 4 RGBA picture. */
async function withPictures(): Promise<Uint8Array> {
  const b = new Builder({ id: new TextEncoder().encode("lamina-e3j-pics1"), producer: "lamina tests" });
  const page = b.page([612, 792]);
  page
    .surface()
    .image(Image.jpeg(await jpeg(64, 48)), [72, 600, 128, 96])
    .image(Image.rgba(4, 4, new Uint8Array(4 * 4 * 4).fill(200)), [300, 600, 40, 40]);
  page.finish();
  const doc = b.finish();
  const bytes = doc.save();
  doc.free();
  return bytes;
}

describe("extractText", () => {
  it("gives the pages' text as plain text", async () => {
    const extracted = await (await opened(twoWords())).extractText({ format: "text", readingOrder: "auto" });
    expect(extracted.text).toContain("Alpha");
    expect(extracted.text).toContain("Bravo");
  });

  it("gives only the pages asked for", async () => {
    const extracted = await (await opened(twoWords())).extractText({
      format: "markdown",
      pages: [1],
      readingOrder: "auto",
    });
    expect(extracted.text).toContain("Bravo");
    expect(extracted.text).not.toContain("Alpha");
  });

  it("gives JSON that parses, with a box for every block", async () => {
    const extracted = await (await opened(twoWords())).extractText({ format: "json", readingOrder: "auto" });
    const json = JSON.parse(extracted.text) as { schema: string; blocks: { bbox: unknown }[] };
    expect(json.schema).toBe("laminapdf.extract/1");
    expect(json.blocks.length).toBeGreaterThan(0);
    for (const block of json.blocks) expect(block.bbox).toHaveLength(4);
  });

  it("says a tagged file is tagged", async () => {
    const extracted = await (await opened(await read(TAGGED))).extractText({
      format: "markdown",
      readingOrder: "tagged",
    });
    expect(extracted.tagged).toBe(true);
  });

  it("reads the document as the edit session leaves it", async () => {
    const c = await opened(twoWords());
    const first = c.document?.pages[0]?.id as string;
    await c.applyEdit([{ op: "deletePages", args: { pages: [first], references: "retarget" } } as Op]);
    const extracted = await c.extractText({ format: "text", readingOrder: "auto" });
    expect(extracted.text).not.toContain("Alpha");
    expect(extracted.text).toContain("Bravo");
  });
});

describe("extractPictures", () => {
  it("gives each picture as a file named by its page", async () => {
    const read = await (await opened(await withPictures())).extractPictures({ keepOriginals: true, minimumSize: 0 });
    expect(read.pictures.map((p) => [p.name, p.width, p.height])).toEqual([
      ["page-1-picture-1.jpg", 64, 48],
      ["page-1-picture-2.png", 4, 4],
    ]);
  });

  it("keeps a JPEG byte for byte with keepOriginals", async () => {
    const read = await (await opened(await withPictures())).extractPictures({ keepOriginals: true, minimumSize: 0 });
    const bytes = read.pictures[0]?.bytes as Uint8Array;
    expect([bytes[0], bytes[1]]).toEqual([0xff, 0xd8]);
  });

  it("writes a JPEG as PNG without keepOriginals", async () => {
    const read = await (await opened(await withPictures())).extractPictures({ keepOriginals: false, minimumSize: 0 });
    const first = read.pictures[0];
    expect(first?.name).toBe("page-1-picture-1.png");
    expect([...(first?.bytes.subarray(1, 4) ?? [])]).toEqual([0x50, 0x4e, 0x47]);
  });

  it("leaves out pictures under minimumSize, counting them as skipped", async () => {
    const read = await (await opened(await withPictures())).extractPictures({ keepOriginals: true, minimumSize: 16 });
    expect(read.pictures.map((p) => p.width)).toEqual([64]);
    expect(read.skipped).toBe(1);
  });
});

describe("exportSummary", () => {
  it("counts a file's links and bookmarks at every level", async () => {
    const c = await opened(await read(LINKS_AND_OUTLINES));
    const summary = await c.exportSummary();
    const { links, bookmarks } = await c.linksAndBookmarks("json");
    const items = (nodes: { children?: unknown[] }[]): number =>
      nodes.reduce((sum, node) => sum + 1 + items((node.children ?? []) as { children?: unknown[] }[]), 0);
    expect(summary.links).toBe((JSON.parse(links) as { links: unknown[] }).links.length);
    expect(summary.links).toBeGreaterThan(0);
    expect(summary.bookmarks).toBe(items((JSON.parse(bookmarks) as { items: { children?: unknown[] }[] }).items));
    expect(summary.bookmarks).toBeGreaterThan(1);
  });

  it("counts again after an edit", async () => {
    const c = await opened(await read(LINKS_AND_OUTLINES));
    const before = (await c.exportSummary()).links;
    // The page with the first link goes: a blank page is added first, so one is left.
    const { links } = await c.linksAndBookmarks("json");
    const first = (JSON.parse(links) as { links: { page: number }[] }).links[0]?.page ?? 0;
    const page = c.document?.pages[first]?.id as string;
    await c.applyEdit([
      {
        op: "insertBlankPages",
        args: { at: { after: page }, count: 1, size: { width: 612, height: 792 }, rotate: 0 },
      } as Op,
    ]);
    await c.applyEdit([{ op: "deletePages", args: { pages: [page], references: "retarget" } } as Op]);
    expect((await c.exportSummary()).links).toBeLessThan(before);
  });
});

describe("linksAndBookmarks", () => {
  it("gives the links and the outline as JSON that parses", async () => {
    const { links, bookmarks } = await (await opened(await read(LINKS_AND_OUTLINES))).linksAndBookmarks("json");
    expect((JSON.parse(links) as { schema: string }).schema).toBe("laminapdf.links/1");
    expect((JSON.parse(bookmarks) as { schema: string }).schema).toBe("laminapdf.outline/1");
  });

  it("gives CSV with a header row", async () => {
    const { links, bookmarks } = await (await opened(await read(LINKS_AND_OUTLINES))).linksAndBookmarks("csv");
    expect(links.split(/\r?\n/)[0]).toContain("page");
    expect(bookmarks.split(/\r?\n/)[0]).toContain("level");
  });
});

describe("imagesToPdf", () => {
  it("makes a new document of a page per picture, each turned to fit its picture", async () => {
    client = await viewer("hayro");
    const picture = await jpeg(40, 30);
    const bytes = await client.imagesToPdf(
      [
        { bytes: picture, type: "image/jpeg" },
        { bytes: picture.slice(), type: "image/jpeg" },
      ],
      { pageSize: "a4", margin: 36 },
    );
    const doc = Document.open(bytes);
    expect(doc.pages.map((p) => [Math.round(p.width), Math.round(p.height)])).toEqual([
      [842, 595],
      [842, 595],
    ]);
    doc.free();
  });

  it("refuses a picture type it can't embed", async () => {
    client = await viewer("hayro");
    const refused = await client
      .imagesToPdf([{ bytes: new Uint8Array([1, 2, 3]), type: "image/gif" }], { pageSize: "image", margin: 0 })
      .catch((e: unknown) => e);
    expect(isLaminaPdfError(refused, "unsupported-image")).toBe(true);
  });
});

describe("exportPage annotations", () => {
  it("draws a page without its annotations when asked", async () => {
    const c = await opened(await read(ANNOTATED));
    const drawn = await c.exportPage(0, { format: "png", dpi: 36 });
    const bare = await c.exportPage(0, { format: "png", dpi: 36, annotations: false });
    expect(bare.image).not.toEqual(drawn.image);
  });

  it("draws a page with its annotations by default, as annotations: true does", async () => {
    const c = await opened(await read(ANNOTATED));
    const byDefault = await c.exportPage(0, { format: "png", dpi: 36 });
    const asked = await c.exportPage(0, { format: "png", dpi: 36, annotations: true });
    expect(asked.image).toEqual(byDefault.image);
  });
});
