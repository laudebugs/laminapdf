// pdf.js's view of fixtures/annotations/ (todo E4b): per file and page, the annotations
// `page.getAnnotations()` returns, as subtype and the object they come from, in order. Writes the
// facts that crates/laminapdf-model/tests/annotations.rs compares with Lamina PDF's reading.
// Tools only, never run by tests.
//
//     cd "$(mktemp -d)" && npm init -y && npm i pdfjs-dist@6.4.299   # Apache-2.0; tools only
//     node tools/fixtures/pdfjs_annotations.mjs "$PWD/node_modules/pdfjs-dist" fixtures/annotations/*.pdf \
//       > fixtures/annotations/pdfjs.json
import { readFileSync } from "node:fs";
import { basename } from "node:path";
import { argv, stdout } from "node:process";
import { loadPdfjs } from "./pdfjs_common.mjs";

const PINNED = "6.4.299";
const [pdfjsDir, ...files] = argv.slice(2);
const { pdfjs, version } = await loadPdfjs(
  pdfjsDir,
  PINNED,
  "usage: pdfjs_annotations.mjs <path to node_modules/pdfjs-dist> <file.pdf>…",
);

const out = { pdfjs_version: version, files: {} };
for (const file of files.sort()) {
  const doc = await pdfjs.getDocument({ data: new Uint8Array(readFileSync(file)), verbosity: 0 }).promise;
  const pages = [];
  for (let i = 1; i <= doc.numPages; i++) {
    const page = await doc.getPage(i);
    const annots = await page.getAnnotations();
    pages.push(annots.map((a) => ({ subtype: a.subtype, id: a.id })));
  }
  out.files[basename(file)] = pages;
}
stdout.write(`${JSON.stringify(out, null, 2)}\n`);
