// pdf.js's text for page 1 of each PDF given (todo B4b): `getTextContent()` items joined, a
// line break wherever pdf.js marks one (`hasEOL`). Called by tools/fixtures/text_facts.py
// when PDFJS_DIST is set; prints {"<file>": ["line", …]} as JSON. Tools only, never run by
// tests.
//
//     cd "$(mktemp -d)" && npm init -y && npm i pdfjs-dist@6.4.299   # Apache-2.0; tools only
//     node tools/fixtures/pdfjs_text.mjs "$PWD/node_modules/pdfjs-dist" <file.pdf>…
import { readFileSync } from "node:fs";
import { basename, join } from "node:path";
import { argv, stdout } from "node:process";
import { pathToFileURL } from "node:url";

const PINNED = "6.4.299";
const [pdfjsDir, ...files] = argv.slice(2);
if (!pdfjsDir) {
  throw new Error("usage: pdfjs_text.mjs <path to node_modules/pdfjs-dist> <file.pdf>…");
}
const version = JSON.parse(readFileSync(join(pdfjsDir, "package.json"), "utf8")).version;
if (version !== PINNED) {
  throw new Error(`pdfjs-dist ${version}, expected ${PINNED}`);
}
const pdfjs = await import(pathToFileURL(join(pdfjsDir, "legacy/build/pdf.mjs")).href);
const out = { pdfjs_version: version, files: {} };
for (const file of files) {
  const data = new Uint8Array(readFileSync(file));
  const doc = await pdfjs.getDocument({ data, verbosity: 0 }).promise;
  const page = await doc.getPage(1);
  const content = await page.getTextContent();
  let text = "";
  for (const item of content.items) {
    text += item.str ?? "";
    if (item.hasEOL) {
      text += "\n";
    }
  }
  out.files[basename(file)] = text.split("\n").filter((line) => line.length > 0);
}
stdout.write(`${JSON.stringify(out)}\n`);
