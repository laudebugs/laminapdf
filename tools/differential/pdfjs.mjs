// pdf.js facts for the nightly lane's live differential (ADR 0018 tier 6):
//
//   node tools/differential/pdfjs.mjs [--forms] <list.tsv> <out.json>
//
// <list.tsv> has one `<name>\t<path>` line per input; <out.json> has the shape
// tools/differential/facts.py writes (outcome, page count, outline titles per file). pdf.js
// repairs silently, so its outcome is "opened" or "failed". Inputs are only read, never run:
// scripting is off (no eval) and fonts are not loaded. The per-file timer below only catches
// stalls between awaits: a synchronous parse loop (or a hanging destroy) blocks it, so
// `cargo xtask differential --live` also kills an overrunning process and reruns its files one
// per process (tools/xtask/src/runner.rs).
//
// With --forms it is pdf.js's side of the form differential (`cargo xtask differential --live
// --forms`): per file {outcome, fields: [{name, kind, value}]}, read from every page's widget
// annotations (`fieldName`, `fieldType`, `fieldValue`), a field once, by its first widget whose
// value isn't "Off". pdf.js
// knows a form only through the widgets the pages list.
import { readFileSync, writeFileSync } from "node:fs";
import { argv, exit } from "node:process";

import { getDocument, version } from "pdfjs-dist/legacy/build/pdf.mjs";

const PER_FILE_MS = 60_000;
const failed = { outcome: "failed", page_count: null, outline_titles: null };
const formsFailed = { outcome: "failed", fields: null };
const timedOut = { outcome: "timeout", page_count: null, outline_titles: null };

/** Lamina PDF's name for a widget's kind (laminapdf_model::FieldKind); null when it has no type. */
function kind(widget) {
  switch (widget.fieldType) {
    case "Tx":
      return "text";
    case "Btn":
      return widget.pushButton ? "pushButton" : widget.radioButton ? "radioGroup" : "checkBox";
    case "Ch":
      return widget.combo ? "comboBox" : "listBox";
    case "Sig":
      return "signature";
    default:
      return null;
  }
}

/** Adds a widget annotation's field to `fields` (by name), or its value to the field there. */
function addWidget(fields, annot) {
  const fieldKind = annot.subtype === "Widget" ? kind(annot) : null;
  const value = annot.fieldValue ?? null;
  const known = fields.get(annot.fieldName);
  if (fieldKind && !known) {
    fields.set(annot.fieldName, { name: annot.fieldName, kind: fieldKind, value });
  } else if (known && (known.value === null || known.value === "Off")) {
    // pdf.js gives each check box widget whose on-state isn't the value "Off": the field's value
    // is the one a widget turns on to.
    known.value = value;
  }
}

async function formFields(doc) {
  const fields = new Map();
  for (let number = 1; number <= doc.numPages; number++) {
    const page = await doc.getPage(number);
    for (const annot of await page.getAnnotations({ intent: "any" })) {
      addWidget(fields, annot);
    }
  }
  return [...fields.values()];
}

function flatten(items, out) {
  for (const item of items ?? []) {
    out.push(item.title);
    flatten(item.items, out);
  }
  return out;
}

async function facts(path, forms) {
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
          return forms ? formsFailed : failed;
        }
        if (forms) {
          try {
            return { outcome: "opened", fields: await formFields(doc) };
          } catch {
            return formsFailed;
          }
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
  const forms = argv[2] === "--forms";
  const [listPath, outPath] = argv.slice(forms ? 3 : 2);
  if (!listPath || !outPath) {
    console.error("usage: node pdfjs.mjs [--forms] <list.tsv> <out.json>");
    exit(2);
  }
  const results = {};
  for (const line of readFileSync(listPath, "utf8").split("\n")) {
    if (!line) continue;
    const tab = line.indexOf("\t");
    results[line.slice(0, tab)] = await facts(line.slice(tab + 1), forms);
  }
  const report = { tool: "pdfjs", version: `pdfjs-dist ${version}`, reports_repair: false, facts: results };
  writeFileSync(outPath, `${JSON.stringify(report, null, 2)}\n`);
  console.log(`pdfjs (${version}): facts for ${Object.keys(results).length} files in ${outPath}`);
}

await main();
