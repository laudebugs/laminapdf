// pdf.js's text for page 1 of each PDF given (todo B4b): `getTextContent()` items joined, a
// line break wherever pdf.js marks one (`hasEOL`). Called by tools/fixtures/text_facts.py
// when PDFJS_DIST is set; prints {"<file>": ["line", …]} as JSON. Tools only, never run by
// tests.
//
//     cd "$(mktemp -d)" && npm init -y && npm i pdfjs-dist@6.4.299   # Apache-2.0; tools only
//     node tools/fixtures/pdfjs_text.mjs "$PWD/node_modules/pdfjs-dist" <file.pdf>…
import { readFileSync } from "node:fs";
import { basename } from "node:path";
import { argv, stdout } from "node:process";
import { loadPdfjs, pageLines } from "./pdfjs_common.mjs";

const PINNED = "6.4.299";
const [pdfjsDir, ...files] = argv.slice(2);
const { pdfjs, version } = await loadPdfjs(
  pdfjsDir,
  PINNED,
  "usage: pdfjs_text.mjs <path to node_modules/pdfjs-dist> <file.pdf>…",
);
const out = { pdfjs_version: version, files: {} };
for (const file of files) {
  const data = new Uint8Array(readFileSync(file));
  const doc = await pdfjs.getDocument({ data, verbosity: 0 }).promise;
  out.files[basename(file)] = await pageLines(await doc.getPage(1));
}
stdout.write(`${JSON.stringify(out)}\n`);
