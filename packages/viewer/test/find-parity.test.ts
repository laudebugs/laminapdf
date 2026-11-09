// Find's counts against pdf.js's (todo C5b; ADR 0018 tier 4): for every file and query in
// fixtures/text/search.json (tools/differential/search.mjs, pdf.js's own find controller), the
// viewer's find, run over Lamina PDF's page text as the document Web Worker reads it, counts the
// same matches on each page, except the cases fixtures/text/search-exceptions.json lists with
// their reasons.
import { readFileSync } from "node:fs";
import { Document, init, loadExtract } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { findOnPage, type PageGlyphs, searchText } from "../src/find/page.js";

const ROOT = new URL("../../../", import.meta.url);
const read = (path: string) => readFileSync(new URL(path, ROOT));

interface Case {
  readonly file: string;
  readonly query: string;
  readonly options: { matchCase?: boolean; wholeWords?: boolean; matchDiacritics?: boolean };
  readonly pages: readonly number[];
}

const facts = JSON.parse(read("fixtures/text/search.json").toString("utf8")) as { version: string; cases: Case[] };
const exceptions = JSON.parse(read("fixtures/text/search-exceptions.json").toString("utf8")) as {
  exceptions: { file: string; query: string; reason: string }[];
};
const excused = (c: Case) => exceptions.exceptions.some((e) => e.file === c.file && e.query === c.query);

beforeAll(async () => {
  await init();
  await loadExtract();
});

/** Every page's glyphs, as the worker keeps them. */
function pagesOf(file: string): PageGlyphs[] {
  using doc = Document.open(new Uint8Array(read(file)));
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
