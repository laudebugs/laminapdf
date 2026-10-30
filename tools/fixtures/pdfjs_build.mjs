// pdf.js's view of lamina-build's B5b goldens (todo B5b): per file, each page's text lines
// (`getTextContent()` items joined, a break wherever pdf.js marks one), link annotations, the
// outline with each item's page, the named destinations, page labels, the Info dictionary and
// the XMP values. Called by tools/fixtures/build_facts.py; prints JSON. Tools only, never run
// by tests.
//
//     cd "$(mktemp -d)" && npm init -y && npm i pdfjs-dist@6.4.299   # Apache-2.0; tools only
//     node tools/fixtures/pdfjs_build.mjs "$PWD/node_modules/pdfjs-dist" <file.pdf>…
import { readFileSync } from "node:fs";
import { basename } from "node:path";
import { argv, stdout } from "node:process";
import { loadPdfjs, pageLines } from "./pdfjs_common.mjs";

const PINNED = "6.4.299";
const [pdfjsDir, ...files] = argv.slice(2);
const { pdfjs, version } = await loadPdfjs(
  pdfjsDir,
  PINNED,
  "usage: pdfjs_build.mjs <path to node_modules/pdfjs-dist> <file.pdf>…",
);

// The 0-based page index of an explicit destination array or a destination name.
async function pageOf(doc, dest) {
  const explicit = typeof dest === "string" ? await doc.getDestination(dest) : dest;
  if (!Array.isArray(explicit)) {
    return null;
  }
  return doc.getPageIndex(explicit[0]);
}

async function outline(doc, items, depth, out) {
  for (const item of items ?? []) {
    out.push({
      title: item.title,
      depth,
      page: item.dest ? await pageOf(doc, item.dest) : null,
      url: item.url ?? null,
      bold: item.bold,
      italic: item.italic,
    });
    await outline(doc, item.items, depth + 1, out);
  }
  return out;
}

const out = { pdfjs_version: version, files: {} };
for (const file of files) {
  const data = new Uint8Array(readFileSync(file));
  const doc = await pdfjs.getDocument({ data, verbosity: 0 }).promise;
  const pages = [];
  const links = [];
  for (let i = 1; i <= doc.numPages; i++) {
    const page = await doc.getPage(i);
    pages.push(await pageLines(page));
    const annotations = await page.getAnnotations();
    const pageLinks = [];
    for (const a of annotations.filter((a) => a.subtype === "Link")) {
      pageLinks.push(a.url ? { url: a.url } : { page: await pageOf(doc, a.dest) });
    }
    links.push(pageLinks);
  }
  const destinations = {};
  // A Map in pdf.js 6 (a plain object in older releases).
  const dests = (await doc.getDestinations()) ?? {};
  for (const [name, dest] of dests instanceof Map ? dests.entries() : Object.entries(dests)) {
    destinations[name] = await pageOf(doc, dest);
  }
  const meta = await doc.getMetadata();
  const xmp = {};
  for (const key of [
    "dc:title",
    "dc:creator",
    "dc:description",
    "pdf:keywords",
    "pdf:producer",
    "xmp:creatortool",
    "xmp:createdate",
    "xmp:modifydate",
  ]) {
    const value = meta.metadata?.get(key);
    if (value !== null && value !== undefined) {
      xmp[key] = Array.isArray(value) ? value.join("; ") : value;
    }
  }
  const info = {};
  for (const key of ["Title", "Author", "Subject", "Keywords", "Creator", "Producer", "CreationDate", "ModDate"]) {
    if (meta.info?.[key] !== undefined) {
      info[key] = meta.info[key];
    }
  }
  out.files[basename(file)] = {
    pages,
    links,
    outline: await outline(doc, await doc.getOutline(), 0, []),
    destinations,
    page_labels: await doc.getPageLabels(),
    info,
    xmp,
  };
}
stdout.write(`${JSON.stringify(out)}\n`);
