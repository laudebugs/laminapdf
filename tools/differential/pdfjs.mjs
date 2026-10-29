// pdf.js facts for the nightly lane's live differential (ADR 0018 tier 6):
//
//   node tools/differential/pdfjs.mjs <list.tsv> <out.json>
//
// <list.tsv> has one `<name>\t<path>` line per input; <out.json> has the shape
// tools/differential/facts.py writes (outcome, page count, outline titles per file). pdf.js
// repairs silently, so its outcome is "opened" or "failed". Inputs are only read, never run:
// scripting is off (no eval) and fonts are not loaded.
import { readFileSync, writeFileSync } from "node:fs";
import { argv, exit } from "node:process";

import { getDocument, version } from "pdfjs-dist/legacy/build/pdf.mjs";

const PER_FILE_MS = 60_000;
const failed = { outcome: "failed", page_count: null, outline_titles: null };
const timedOut = { outcome: "timeout", page_count: null, outline_titles: null };

function flatten(items, out) {
  for (const item of items ?? []) {
    out.push(item.title);
    flatten(item.items, out);
  }
  return out;
}

async function facts(path) {
  const task = getDocument({
    data: new Uint8Array(readFileSync(path)),
    isEvalSupported: false,
    disableFontFace: true,
    useSystemFonts: false,
    stopAtErrors: false,
    verbosity: 0,
  });
  let timer;
  const timeout = new Promise((resolve) => {
    timer = setTimeout(() => resolve("timeout"), PER_FILE_MS);
  });
  try {
    const result = await Promise.race([
      (async () => {
        let doc;
        try {
          doc = await task.promise;
        } catch {
          return failed;
        }
        let titles;
        try {
          titles = flatten(await doc.getOutline(), []);
        } catch {
          titles = null;
        }
        return { outcome: "opened", page_count: doc.numPages, outline_titles: titles };
      })(),
      timeout,
    ]);
    return result === "timeout" ? timedOut : result;
  } finally {
    clearTimeout(timer);
    await task.destroy();
  }
}

async function main() {
  const [listPath, outPath] = argv.slice(2);
  if (!listPath || !outPath) {
    console.error("usage: node pdfjs.mjs <list.tsv> <out.json>");
    exit(2);
  }
  const results = {};
  for (const line of readFileSync(listPath, "utf8").split("\n")) {
    if (!line) continue;
    const tab = line.indexOf("\t");
    results[line.slice(0, tab)] = await facts(line.slice(tab + 1));
  }
  const report = { tool: "pdfjs", version: `pdfjs-dist ${version}`, reports_repair: false, facts: results };
  writeFileSync(outPath, `${JSON.stringify(report, null, 2)}\n`);
  console.log(`pdfjs (${version}): facts for ${Object.keys(results).length} files in ${outPath}`);
}

await main();
