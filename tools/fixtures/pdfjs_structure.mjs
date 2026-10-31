// pdf.js's view of lamina-build's tagged goldens (todo B8a): per file and page, the structure
// tree `page.getStructTree()` returns, reduced to each element's role, alternate text and
// language, its children, and the marked content (MCIDs) and annotations it holds. Called by
// tools/fixtures/structure_facts.py; prints JSON. Tools only, never run by tests.
//
//     cd "$(mktemp -d)" && npm init -y && npm i pdfjs-dist@6.4.299   # Apache-2.0; tools only
//     node tools/fixtures/pdfjs_structure.mjs "$PWD/node_modules/pdfjs-dist" <file.pdf>…
import { readFileSync } from "node:fs";
import { basename } from "node:path";
import { argv, stdout } from "node:process";
import { loadPdfjs } from "./pdfjs_common.mjs";

const PINNED = "6.4.299";
const [pdfjsDir, ...files] = argv.slice(2);
const { pdfjs, version } = await loadPdfjs(
  pdfjsDir,
  PINNED,
  "usage: pdfjs_structure.mjs <path to node_modules/pdfjs-dist> <file.pdf>…",
);

// pdf.js names marked content `<prefix>mc<MCID>` and annotations by their object.
function node(n) {
  if (n.type === "content") {
    return { content: Number(n.id.replace(/^.*mc/, "")) };
  }
  if (n.type === "annotation") {
    return { annotation: true };
  }
  if (n.type === "object") {
    return { object: true };
  }
  const out = { role: n.role };
  if (n.alt !== undefined) out.alt = n.alt;
  if (n.lang !== undefined) out.lang = n.lang;
  out.children = (n.children ?? []).map(node);
  return out;
}

const out = { pdfjs_version: version, files: {} };
for (const file of files) {
  const data = new Uint8Array(readFileSync(file));
  const doc = await pdfjs.getDocument({ data, verbosity: 0 }).promise;
  const pages = [];
  for (let i = 1; i <= doc.numPages; i++) {
    const page = await doc.getPage(i);
    const tree = await page.getStructTree();
    pages.push(tree ? node(tree) : null);
  }
  out.files[basename(file)] = { pages };
}
stdout.write(`${JSON.stringify(out)}\n`);
