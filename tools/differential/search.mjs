// Find counts from pdf.js (todo C5b; ADR 0018 tier 4): for each file and query below, how many
// matches pdf.js's own find controller (PDFFindController, the one its viewer uses) counts on
// each page, with each query's options. The reader's find must count the same, which
// packages/viewer/test/find-parity.test.ts checks against the committed facts, with the
// deliberate differences in fixtures/text/search-exceptions.json.
//
//   node tools/differential/search.mjs fixtures/text/search.json
//
// Run by hand when the files or queries change; never by tests. Inputs are only read: scripting
// off, no fonts loaded. pdf_viewer.mjs reads a few DOM globals when it loads, which a stub gives;
// find itself needs none.
import { readFileSync, writeFileSync } from "node:fs";
import { argv, exit } from "node:process";
import { fileURLToPath } from "node:url";

const style = { setProperty() {}, getPropertyValue: () => "" };
globalThis.document = {
  documentElement: { style },
  createElement: () => ({ style, getContext: () => null }),
  addEventListener() {},
};
globalThis.window = globalThis;
globalThis.requestAnimationFrame = (f) => setTimeout(f, 0);
globalThis.navigator ??= { userAgent: "node" };

const { getDocument, version } = await import("pdfjs-dist/legacy/build/pdf.mjs");
const { EventBus, PDFFindController } = await import("pdfjs-dist/legacy/web/pdf_viewer.mjs");

const ROOT = fileURLToPath(new URL("../../", import.meta.url));

/** The files: Latin with ligatures and accents, the standard 14, five scripts, several pages. */
export const FILES = [
  "fixtures/golden/text-latin.pdf",
  "fixtures/golden/text-shaped.pdf",
  "fixtures/golden/text-std14.pdf",
  "fixtures/golden/text-cff.pdf",
  "fixtures/golden/build-text-latin.pdf",
  "fixtures/golden/build-text-multiscript.pdf",
  "fixtures/golden/links-and-outlines.pdf",
  "fixtures/golden/examples/invoice.pdf",
];

/** The queries, with the reader's option names. */
export const QUERIES = [
  { query: "the" },
  { query: "The", options: { matchCase: true } },
  { query: "the", options: { wholeWords: true } },
  { query: "office" },
  { query: "fine" },
  { query: "waffle flask" },
  { query: "cafe" },
  { query: "café", options: { matchDiacritics: true } },
  { query: "chapter" },
  { query: "שלום" },
  { query: "مرحبا" },
  { query: "हिन्दी" },
  { query: "中文" },
];

/** Per-page match counts of `query` in the open `doc`. */
async function counts(doc, { query, options = {} }) {
  const eventBus = new EventBus();
  const linkService = { pdfDocument: doc, pagesCount: doc.numPages, page: 1, rotation: 0, scrollPageIntoView() {} };
  const find = new PDFFindController({ linkService, eventBus, updateMatchesCountOnProgress: false });
  find.setDocument(doc);
  eventBus.dispatch("find", {
    source: null,
    type: "",
    query,
    caseSensitive: options.matchCase === true,
    entireWord: options.wholeWords === true,
    matchDiacritics: options.matchDiacritics === true,
    highlightAll: true,
    findPrevious: false,
  });
  // Every page gets an array once its text is searched.
  for (let i = 0; i < 2000; i++) {
    const m = find.pageMatches;
    if (m.length === doc.numPages && m.every((x) => Array.isArray(x))) return m.map((x) => x.length);
    await new Promise((r) => setTimeout(r, 10));
  }
  throw new Error(`pdf.js never finished searching for ${query}`);
}

const out = argv[2];
if (out === undefined) {
  console.error("usage: node tools/differential/search.mjs <out.json>");
  exit(2);
}
const cases = [];
for (const file of FILES) {
  const task = getDocument({
    data: new Uint8Array(readFileSync(`${ROOT}${file}`)),
    isEvalSupported: false,
    disableFontFace: true,
    useSystemFonts: false,
    verbosity: 0,
  });
  const doc = await task.promise;
  for (const q of QUERIES) cases.push({ file, query: q.query, options: q.options ?? {}, pages: await counts(doc, q) });
  await task.destroy();
}
writeFileSync(out, `${JSON.stringify({ tool: "pdf.js", version, cases }, null, 2)}\n`);
console.log(`wrote ${cases.length} cases to ${out}`);
exit(0);
