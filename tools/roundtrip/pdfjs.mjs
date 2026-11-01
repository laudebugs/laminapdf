// The pdf.js half of the round-trip suite (todo B3c, ADR 0018 tier 7): opens every output
// listed in <out-dir>/manifest.json (written by laminapdf-save's `roundtrip` example) and checks
// the page count, each page's rotation, the Info /Title and whether the file is encrypted
// against it. `cargo xtask roundtrip`
// runs it; it prints one JSON line ({ tool, version, checked, failures }) and exits 1 on any
// failure.
//
//     node tools/roundtrip/pdfjs.mjs <path to node_modules/pdfjs-dist> <out-dir>
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { argv, exit, stdout } from "node:process";
import { pathToFileURL } from "node:url";

const PINNED = "6.4.299";
const [pdfjsDir, outDir] = argv.slice(2);
if (!pdfjsDir || !outDir) {
  throw new Error("usage: pdfjs.mjs <path to node_modules/pdfjs-dist> <out-dir>");
}
const version = JSON.parse(readFileSync(join(pdfjsDir, "package.json"), "utf8")).version;
if (version !== PINNED) {
  throw new Error(`pdfjs-dist ${version}, expected ${PINNED}`);
}
const pdfjs = await import(pathToFileURL(join(pdfjsDir, "legacy/build/pdf.mjs")).href);
const { cases } = JSON.parse(readFileSync(join(outDir, "manifest.json"), "utf8"));

const failures = [];
for (const c of cases) {
  try {
    const data = new Uint8Array(readFileSync(join(outDir, c.file)));
    const task = pdfjs.getDocument({ data, verbosity: 0, stopAtErrors: true });
    const doc = await task.promise;
    const rotations = [];
    for (let i = 1; i <= doc.numPages; i++) {
      rotations.push((await doc.getPage(i)).rotate);
    }
    const { info } = await doc.getMetadata();
    const title = info.Title ?? null;
    // getPermissions() is null for a document without encryption.
    const encrypted = (await doc.getPermissions()) !== null || info.IsEncrypted === true;
    // The setInfo cases also wrote /Author, /Subject and /Keywords (c.info); null elsewhere.
    const keys = c.info === null ? null : Object.fromEntries(Object.keys(c.info).map((k) => [k, info[k] ?? null]));
    const got = { pages: doc.numPages, rotations, title, encrypted, info: keys };
    const want = { pages: c.pages, rotations: c.rotations, title: c.title, encrypted: c.encrypted, info: c.info };
    if (JSON.stringify(got) !== JSON.stringify(want)) {
      failures.push(`${c.file}: got ${JSON.stringify(got)}, expected ${JSON.stringify(want)}`);
    }
    await task.destroy();
  } catch (error) {
    failures.push(`${c.file}: ${error}`);
  }
}
stdout.write(`${JSON.stringify({ tool: "pdf.js", version, checked: cases.length, failures })}\n`);
exit(failures.length === 0 ? 0 : 1);
