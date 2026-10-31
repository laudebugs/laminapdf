// Per-page text from pdf.js and pdftotext for `cargo xtask differential --text --update`
// (todo B9c; ADR 0018 tier 6): writes fixtures/differential/text.json, which
// crates/lamina/tests/text_differential.rs compares lamina's `page_texts` with.
//
//   node tools/differential/text.mjs <out.json> <repo-relative path>…
//
// pdf.js: `getTextContent()` per page, items joined, a newline after `hasEOL`. pdftotext: the
// whole file (`-enc UTF-8`, reading order, no `-layout`), split on form feeds. Both are stored
// as they come; the test strips whitespace before comparing (each tool, and lamina, inserts
// spaces by its own heuristics). Inputs are only read: scripting off, no fonts loaded.
import { execFileSync, spawnSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { argv, exit } from "node:process";
import { getDocument, version } from "pdfjs-dist/legacy/build/pdf.mjs";

const [out, ...files] = argv.slice(2);
if (out === undefined || files.length === 0) {
  console.error("usage: node tools/differential/text.mjs <out.json> <path>…");
  exit(2);
}

/** The pages' text, or null when pdf.js can't open the file (an encrypted one, say). */
async function pdfjs(path) {
  const task = getDocument({
    data: new Uint8Array(readFileSync(path)),
    isEvalSupported: false,
    disableFontFace: true,
    useSystemFonts: false,
    verbosity: 0,
  });
  try {
    const doc = await task.promise;
    const pages = [];
    for (let i = 1; i <= doc.numPages; i++) {
      const content = await (await doc.getPage(i)).getTextContent();
      pages.push(content.items.map((item) => ("str" in item ? item.str + (item.hasEOL ? "\n" : "") : "")).join(""));
    }
    return pages;
  } catch {
    return null;
  } finally {
    await task.destroy();
  }
}

/** The pages' text, or null when pdftotext fails. */
function pdftotext(path) {
  let text;
  try {
    text = execFileSync("pdftotext", ["-enc", "UTF-8", path, "-"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    });
  } catch {
    return null;
  }
  const pages = text.split("\f");
  // pdftotext ends every page with a form feed: the last piece is empty.
  if (pages.at(-1) === "") pages.pop();
  return pages;
}

// `pdftotext -v` prints its version on stderr.
const popplerVersion = spawnSync("pdftotext", ["-v"], { encoding: "utf8" }).stderr.split("\n")[0]?.trim();
const result = { pdfjs_version: version, pdftotext_version: popplerVersion ?? "unknown", files: {} };
for (const path of files) {
  result.files[path] = { pdfjs: await pdfjs(path), pdftotext: pdftotext(path) };
}
writeFileSync(out, `${JSON.stringify(result, null, 2)}\n`);
