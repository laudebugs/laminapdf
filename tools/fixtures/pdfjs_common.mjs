// Helpers shared by the pdf.js facts scripts in tools/fixtures/ (pdfjs_text.mjs,
// pdfjs_build.mjs): loading the pinned pdfjs-dist and a page's text as lines. Tools only, never
// run by tests.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

// pdf.js from `pdfjsDir` (node_modules/pdfjs-dist), which must be version `pinned`.
export async function loadPdfjs(pdfjsDir, pinned, usage) {
  if (!pdfjsDir) {
    throw new Error(usage);
  }
  const version = JSON.parse(readFileSync(join(pdfjsDir, "package.json"), "utf8")).version;
  if (version !== pinned) {
    throw new Error(`pdfjs-dist ${version}, expected ${pinned}`);
  }
  const pdfjs = await import(pathToFileURL(join(pdfjsDir, "legacy/build/pdf.mjs")).href);
  return { pdfjs, version };
}

// A page's `getTextContent()` items joined, a line break wherever pdf.js marks one (`hasEOL`),
// empty lines dropped.
export async function pageLines(page) {
  const content = await page.getTextContent();
  let text = "";
  for (const item of content.items) {
    text += item.str ?? "";
    if (item.hasEOL) {
      text += "\n";
    }
  }
  return text.split("\n").filter((line) => line.length > 0);
}
