// Per-page text from pdf.js and pdftotext (todo B9c, B9f; ADR 0018 tier 6), in two modes:
//
//   node tools/differential/text.mjs <out.json> <repo-relative path>…
//     `cargo xtask differential --text --update`: writes fixtures/differential/text.json, which
//     crates/laminapdf/tests/text_differential.rs compares lamina's `page_texts` with.
//
//   node tools/differential/text.mjs --facts pdfjs|pdftotext <list.tsv> <out.json>
//     `cargo xtask differential --live --text`: one tool's text for every `<name>\t<path>` line
//     of the list, in the report shape of the other differential runners ({tool, version,
//     reports_repair, facts: {name: {outcome, pages, page_errors?}}}; a pdf.js page that throws
//     is null, its message in page_errors), run under tools/xtask/src/runner.rs.
//
// pdf.js: `getTextContent()` per page, items joined, a newline after `hasEOL`. pdftotext: the
// whole file (`-enc UTF-8`, reading order, no `-layout`), split on form feeds. Both are stored
// as they come; the comparison strips whitespace (each tool, and lamina, inserts spaces by its
// own heuristics). Inputs are only read: scripting off, no fonts loaded.
import { execFileSync, spawnSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { argv, exit } from "node:process";
import { getDocument, version } from "pdfjs-dist/legacy/build/pdf.mjs";

/** Per-file limit in --facts mode; runner.rs also kills an overrunning process. */
const PER_FILE_MS = 60_000;

/**
 * The pages' text, or null when pdf.js can't open the file (an encrypted one, say). `started`
 * gets the loading task, so a caller that gives up can destroy it. With `pageErrors` (an
 * object), a page whose getPage or getTextContent throws is null and its message is recorded
 * by page index; without it, such a page fails the whole file (the PR lane's text.json).
 */
async function pdfjs(path, started = () => {}, pageErrors = undefined) {
  const task = getDocument({
    data: new Uint8Array(readFileSync(path)),
    isEvalSupported: false,
    disableFontFace: true,
    useSystemFonts: false,
    verbosity: 0,
  });
  started(task);
  try {
    const doc = await task.promise;
    const pages = [];
    for (let i = 1; i <= doc.numPages; i++) {
      try {
        const content = await (await doc.getPage(i)).getTextContent();
        pages.push(content.items.map((item) => ("str" in item ? item.str + (item.hasEOL ? "\n" : "") : "")).join(""));
      } catch (error) {
        if (pageErrors === undefined) throw error;
        pageErrors[i - 1] = String(error?.message ?? error);
        pages.push(null);
      }
    }
    return pages;
  } catch {
    return null;
  } finally {
    await task.destroy();
  }
}

/** The pages' text, null when pdftotext fails, or "timeout" when it overruns `timeout` ms. */
function pdftotext(path, timeout = 0) {
  let text;
  try {
    text = execFileSync("pdftotext", ["-enc", "UTF-8", path, "-"], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
      maxBuffer: 1 << 30,
      timeout,
    });
  } catch (error) {
    return timeout > 0 && error?.code === "ETIMEDOUT" ? "timeout" : null;
  }
  const pages = text.split("\f");
  // pdftotext ends every page with a form feed: the last piece is empty.
  if (pages.at(-1) === "") pages.pop();
  return pages;
}

// `pdftotext -v` prints its version on stderr.
const popplerVersion = spawnSync("pdftotext", ["-v"], { encoding: "utf8" }).stderr?.split("\n")[0]?.trim();

/** One file's facts for --facts mode: `{outcome: "opened"|"failed"|"timeout", pages}`. */
async function textFacts(tool, path) {
  if (tool === "pdftotext") {
    return outcome(pdftotext(path, PER_FILE_MS));
  }
  let timer;
  let task;
  const pageErrors = {};
  const timeout = new Promise((resolve) => {
    timer = setTimeout(() => resolve("timeout"), PER_FILE_MS);
  });
  try {
    const pages = await Promise.race([pdfjs(path, (t) => (task = t), pageErrors), timeout]);
    if (pages === "timeout") await task?.destroy();
    const facts = outcome(pages);
    if (facts.pages !== null && Object.keys(pageErrors).length > 0) facts.page_errors = pageErrors;
    return facts;
  } finally {
    clearTimeout(timer);
  }
}

function outcome(pages) {
  if (pages === "timeout") return { outcome: "timeout", pages: null };
  return pages === null ? { outcome: "failed", pages: null } : { outcome: "opened", pages };
}

async function facts(tool, listPath, outPath) {
  const results = {};
  for (const line of readFileSync(listPath, "utf8").split("\n")) {
    if (!line) continue;
    const tab = line.indexOf("\t");
    results[line.slice(0, tab)] = await textFacts(tool, line.slice(tab + 1));
  }
  const toolVersion = tool === "pdfjs" ? `pdfjs-dist ${version}` : (popplerVersion ?? "unknown");
  // Neither tool says whether it repaired the file.
  const report = { tool, version: toolVersion, reports_repair: false, facts: results };
  writeFileSync(outPath, `${JSON.stringify(report)}\n`);
  console.log(`${tool} (${toolVersion}): text for ${Object.keys(results).length} files in ${outPath}`);
}

const args = argv.slice(2);
if (args[0] === "--facts") {
  const [, tool, listPath, outPath] = args;
  if (!["pdfjs", "pdftotext"].includes(tool) || !listPath || !outPath) {
    console.error("usage: node tools/differential/text.mjs --facts pdfjs|pdftotext <list.tsv> <out.json>");
    exit(2);
  }
  await facts(tool, listPath, outPath);
} else {
  const [out, ...files] = args;
  if (out === undefined || files.length === 0) {
    console.error("usage: node tools/differential/text.mjs <out.json> <path>…");
    exit(2);
  }
  const result = { pdfjs_version: version, pdftotext_version: popplerVersion ?? "unknown", files: {} };
  for (const path of files) {
    result.files[path] = { pdfjs: await pdfjs(path), pdftotext: pdftotext(path) };
  }
  writeFileSync(out, `${JSON.stringify(result, null, 2)}\n`);
}
