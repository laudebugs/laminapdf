// What pdf.js's viewer shows for a form whose fields format, validate and calculate (todo E8f):
//
//   node tools/differential/forms.mjs <form.pdf> <steps.json> <out.json>
//
// pdf.js runs a form's scripts in its QuickJS sandbox, built to WebAssembly. This tool starts that
// sandbox in Node as pdf.js's viewer does (PDFScriptingManager), with a stub window that opens no
// dialog and reaches no network, and records what the viewer would show: each field's formatted
// value on opening, then after each step of <steps.json> (`[{ "field": name, "value": typed }]`),
// a commit as when the user leaves the field. Lamina PDF never runs scripts (ADR 0021); this is a
// tool that writes reference values for its tests (fixtures/forms/*.pdfjs.json), never shipped.
import { readFileSync, writeFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { argv, exit } from "node:process";
import { pathToFileURL } from "node:url";

import { getDocument, version } from "pdfjs-dist/legacy/build/pdf.mjs";

const require = createRequire(import.meta.url);
const dist = dirname(require.resolve("pdfjs-dist/package.json"));

/** The latest `send` of each field id: its value and the text it shows. */
const shown = new Map();

globalThis.window = {
  setTimeout,
  clearTimeout,
  setInterval,
  clearInterval,
  URL,
  console,
  CustomEvent: class {
    constructor(type, init) {
      this.type = type;
      this.detail = init?.detail;
    }
  },
  alert() {},
  confirm: () => false,
  prompt: () => null,
  dispatchEvent(event) {
    const detail = event.detail;
    if (event.type !== "updatefromsandbox" || !detail?.id) {
      return;
    }
    const before = shown.get(detail.id) ?? {};
    const after = { ...before };
    if ("value" in detail) {
      after.value = detail.value;
    }
    if ("formattedValue" in detail) {
      after.formattedValue = detail.formattedValue;
    }
    shown.set(detail.id, after);
  },
};

// The sandbox's loader fetches its WebAssembly by URL, and Node's fetch reads no file: URLs.
const nodeFetch = globalThis.fetch;
globalThis.fetch = (url, init) =>
  String(url).startsWith("file:")
    ? Promise.resolve(
        new Response(readFileSync(new URL(String(url))), { headers: { "content-type": "application/wasm" } }),
      )
    : nodeFetch(url, init);

const { QuickJSSandbox } = await import(pathToFileURL(join(dist, "build/pdf.sandbox.mjs")).href);

async function sandboxFor(pdf) {
  // pdf.js answers with Maps, which the viewer hands the sandbox as they are.
  const fields = await pdf.getFieldObjects();
  const objects = fields instanceof Map ? Object.fromEntries(fields) : (fields ?? {});
  const sandbox = await QuickJSSandbox(pathToFileURL(join(dist, "wasm/")).href);
  sandbox.create({
    objects,
    calculationOrder: await pdf.getCalculationOrderIds(),
    appInfo: { platform: "Linux", language: "en-US" },
    docInfo: {
      numPages: pdf.numPages,
      baseURL: "",
      filesize: 0,
      title: "",
      author: "",
      subject: "",
      keywords: "",
      creator: "",
      producer: "",
      creationDate: null,
      modDate: null,
      actions: await pdf.getJSActions(),
    },
  });
  const ids = new Map();
  for (const [name, widgets] of Object.entries(objects)) {
    for (const w of widgets) {
      ids.set(w.id, name);
    }
  }
  return { sandbox, ids, objects };
}

/** Each field's shown text, by name, from what the sandbox sent. */
function snapshot(ids) {
  const out = {};
  for (const [id, detail] of shown) {
    const name = ids.get(id);
    if (name !== undefined) {
      out[name] = detail.formattedValue ?? (detail.value === undefined ? undefined : String(detail.value));
    }
  }
  return Object.fromEntries(Object.entries(out).sort(([a], [b]) => a.localeCompare(b)));
}

const [, , pdfPath, stepsPath, outPath] = argv;
if (!pdfPath || !stepsPath || !outPath) {
  console.error("usage: node tools/differential/forms.mjs <form.pdf> <steps.json> <out.json>");
  exit(2);
}
const task = getDocument({
  data: new Uint8Array(readFileSync(pdfPath)),
  isEvalSupported: false,
  disableFontFace: true,
  useSystemFonts: false,
  verbosity: 0,
});
const pdf = await task.promise;
const { sandbox, ids, objects } = await sandboxFor(pdf);
sandbox.dispatchEvent({ id: "doc", name: "Open" });
const result = { pdfjs: version, open: snapshot(ids), steps: [] };
for (const step of JSON.parse(readFileSync(stepsPath, "utf8"))) {
  const id = objects[step.field]?.[0]?.id;
  if (id === undefined) {
    throw new Error(`no field ${step.field}`);
  }
  sandbox.dispatchEvent({
    id,
    name: "Keystroke",
    value: step.value,
    willCommit: true,
    commitKey: 1,
    selStart: 0,
    selEnd: 0,
  });
  result.steps.push({ ...step, shown: snapshot(ids) });
}
sandbox.nukeSandbox();
await task.destroy();
writeFileSync(outPath, `${JSON.stringify(result, null, 2)}\n`);
