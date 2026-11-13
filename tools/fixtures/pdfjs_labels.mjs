// pdf.js's view of the page labels SetPageLabels wrote (todo E2q): per file, the list
// `getPageLabels()` returns (null when the file has none). Called by
// tools/fixtures/label_facts.py; prints JSON. Tools only, never run by tests.
//
//     cd "$(mktemp -d)" && npm init -y && npm i pdfjs-dist@6.4.299   # Apache-2.0; tools only
//     node tools/fixtures/pdfjs_labels.mjs "$PWD/node_modules/pdfjs-dist" <file.pdf>…
import { readFileSync } from "node:fs";
import { basename } from "node:path";
import { argv, stdout } from "node:process";
import { loadPdfjs } from "./pdfjs_common.mjs";

const PINNED = "6.4.299";
const [pdfjsDir, ...files] = argv.slice(2);
const { pdfjs, version } = await loadPdfjs(
  pdfjsDir,
  PINNED,
  "usage: pdfjs_labels.mjs <path to node_modules/pdfjs-dist> <file.pdf>…",
);

const out = { pdfjs_version: version, files: {} };
for (const file of files) {
  const data = new Uint8Array(readFileSync(file));
  const doc = await pdfjs.getDocument({ data, verbosity: 0 }).promise;
  out.files[basename(file)] = await doc.getPageLabels();
}
stdout.write(`${JSON.stringify(out)}\n`);
