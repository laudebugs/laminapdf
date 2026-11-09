// Find's counts against pdf.js's (ADR 0018 tier 4): for every file and query in
// fixtures/text/search.json (tools/differential/search.mjs, pdf.js's own find controller), the
// viewer's find, run over Lamina PDF's page text as the document Web Worker reads it, counts the
// same matches on each page, except the cases fixtures/text/search-exceptions.json lists with
// their reasons.
import { Document, init, loadExtract } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { findOnPage, type PageGlyphs, searchText } from "../src/find/page.js";

// Read through Vite, so the test needs no Node API: the facts as text, the PDFs as data URLs.
const texts = import.meta.glob<string>(
  ["../../../fixtures/text/search.json", "../../../fixtures/text/search-exceptions.json"],
  {
    query: "?raw",
    import: "default",
    eager: true,
  },
);
const pdfs = import.meta.glob<string>("../../../fixtures/golden/**/*.pdf", {
  query: "?inline",
  import: "default",
  eager: true,
});

/** Repository file `path`'s text. */
function readText(path: string): string {
  const text = texts[`../../../${path}`];
  if (text === undefined) throw new Error(`${path} is not read by the test`);
  return text;
}

/** Repository PDF `path`'s bytes, from its data URL. */
function readPdf(path: string): Uint8Array {
  const url = pdfs[`../../../${path}`];
  if (url === undefined) throw new Error(`${path} is not read by the test`);
  const base64 = url.slice(url.indexOf(",") + 1);
  return Uint8Array.from(atob(base64), (c) => c.charCodeAt(0));
}

interface Case {
  readonly file: string;
  readonly query: string;
  readonly options: { matchCase?: boolean; wholeWords?: boolean; matchDiacritics?: boolean };
  readonly pages: readonly number[];
}

const facts = JSON.parse(readText("fixtures/text/search.json")) as { version: string; cases: Case[] };
const exceptions = JSON.parse(readText("fixtures/text/search-exceptions.json")) as {
  exceptions: { file: string; query: string; reason: string }[];
};
const excused = (c: Case) => exceptions.exceptions.some((e) => e.file === c.file && e.query === c.query);

beforeAll(async () => {
  await init();
  await loadExtract();
});

/** Every page's glyphs, as the worker keeps them. */
function pagesOf(file: string): PageGlyphs[] {
  using doc = Document.open(readPdf(file));
  return Array.from({ length: doc.pageCount }, (_, i) => {
    const text = doc.page(i).text();
    const { quads, glyphText, glyphOffsets } = text.buffers;
    const runs = text.runs.map((run) => ({
      text: run.text,
      size: run.size,
      vertical: run.vertical,
      actualText: run.actualText,
      invisible: run.invisible,
      glyphRange: run.glyphRange,
    }));
    return { runs, quads, glyphText, glyphOffsets };
  });
}

describe(`find counts against pdf.js ${facts.version}`, () => {
  const files = new Map<string, PageGlyphs[]>();
  const pages = (file: string) => {
    let got = files.get(file);
    if (got === undefined) {
      got = pagesOf(file);
      files.set(file, got);
    }
    return got;
  };

  it.each(
    facts.cases
      .filter((c) => !excused(c))
      .map((c) => [`${c.file} "${c.query}" ${JSON.stringify(c.options)}`, c] as const),
  )("%s", (_name, c) => {
    const counts = pages(c.file).map((p) => findOnPage(p, searchText(p), c.query, c.options).length);
    expect(counts).toEqual(c.pages);
  });

  it("lists only exceptions that name a case and say why", () => {
    for (const e of exceptions.exceptions) {
      expect(facts.cases.some((c) => c.file === e.file && c.query === e.query)).toBe(true);
      expect(e.reason.length).toBeGreaterThan(20);
    }
  });
});
