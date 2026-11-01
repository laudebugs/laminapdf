// pdf.js's operator list for every page of fixtures/corpus/, fixtures/golden/ and fixtures/content/, the
// independent count that crates/laminapdf-content/tests/corpus.rs compares lamina's
// interpreter with (todo B9a, ADR 0023 §6). Never run by tests; rerun when a fixture changes.
//
//     cd "$(mktemp -d)" && npm init -y && npm i pdfjs-dist@6.4.299   # Apache-2.0; tools only
//     node <repo>/tools/corpus-facts/pdfjs-operators.mjs "$PWD/node_modules/pdfjs-dist"
//
// Writes fixtures/corpus/pdfjs-operators.json: per file, per page, pdf.js's operator names
// and counts from `page.getOperatorList()` with annotations disabled. The `OPLIST` intent
// that getOperatorList uses turns the queue optimiser off, so nothing is merged; the
// `dependency` operators (one per font or image loaded, not content operators) are left out.
import { readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { argv } from "node:process";
import { fileURLToPath, pathToFileURL } from "node:url";

const PINNED = "6.4.299";
const [pdfjsDir] = argv.slice(2);
if (!pdfjsDir) {
  throw new Error("usage: pdfjs-operators.mjs <path to node_modules/pdfjs-dist>");
}
const version = JSON.parse(readFileSync(join(pdfjsDir, "package.json"), "utf8")).version;
if (version !== PINNED) {
  throw new Error(`pdfjs-dist ${version}, expected ${PINNED}`);
}
const pdfjs = await import(pathToFileURL(join(pdfjsDir, "legacy/build/pdf.mjs")).href);
const names = Object.fromEntries(Object.entries(pdfjs.OPS).map(([name, code]) => [code, name]));
const root = join(dirname(fileURLToPath(import.meta.url)), "../..");

const out = {};
for (const dir of ["fixtures/corpus", "fixtures/golden", "fixtures/content"]) {
  for (const file of readdirSync(join(root, dir))
    .filter((f) => f.endsWith(".pdf"))
    .sort()) {
    const data = new Uint8Array(readFileSync(join(root, dir, file)));
    const pages = [];
    try {
      const doc = await pdfjs.getDocument({ data, verbosity: 0 }).promise;
      for (let i = 1; i <= doc.numPages; i++) {
        const page = await doc.getPage(i);
        const list = await page.getOperatorList({ annotationMode: pdfjs.AnnotationMode.DISABLE });
        const counts = {};
        for (const code of list.fnArray) {
          const name = names[code];
          if (name !== "dependency") counts[name] = (counts[name] ?? 0) + 1;
        }
        pages.push(Object.fromEntries(Object.entries(counts).sort()));
      }
    } catch (error) {
      out[`${dir}/${file}`] = { error: String(error) };
      continue;
    }
    out[`${dir}/${file}`] = pages;
  }
}
writeFileSync(join(root, "fixtures/corpus/pdfjs-operators.json"), `${JSON.stringify(out, null, 2)}\n`);
