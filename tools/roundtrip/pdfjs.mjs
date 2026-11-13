// The pdf.js half of the round-trip suite (todo B3c, ADR 0018 tier 7): opens every output
// listed in <out-dir>/manifest.json (written by laminapdf-save's `roundtrip` example) and checks
// the page count, each page's rotation, the Info /Title, whether the file is encrypted, for the
// outline cases the outline (depth, title, page per item), and in the setInitialView cases the
// page the file opens on, its page mode and its page layout, against it. `cargo xtask roundtrip`
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

/** In a setInitialView case (c.initial_view), how pdf.js reads the file's opening view; else null. */
async function initialView(doc, c) {
  if ((c.initial_view ?? null) === null) return null;
  // pdf.js answers with a Map whose "dest" is the explicit destination: [page ref, /FitH, …].
  const dest = (await doc.getOpenAction())?.get("dest");
  return {
    open_page: Array.isArray(dest) ? await doc.getPageIndex(dest[0]) : null,
    page_mode: await doc.getPageMode(),
    page_layout: await doc.getPageLayout(),
  };
}

/** The outline as [depth, title, page index or null] per item in document order. */
async function outlineOf(doc) {
  const out = [];
  const walk = async (items, depth) => {
    for (const item of items ?? []) {
      let dest = item.dest;
      if (typeof dest === "string") dest = await doc.getDestination(dest);
      let page = null;
      if (Array.isArray(dest) && dest[0] !== null && typeof dest[0] === "object") {
        page = await doc.getPageIndex(dest[0]).catch(() => null);
      }
      out.push([depth, item.title, page]);
      await walk(item.items, depth + 1);
    }
  };
  await walk(await doc.getOutline(), 0);
  return out;
}

// Each page's label (getPageLabels() is null for a document without labels).
async function pageLabels(doc) {
  const labels = await doc.getPageLabels();
  return Array.from({ length: doc.numPages }, (_, i) => labels?.[i] ?? null);
}

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
    // The setPageLabels cases list every page's label (c.labels, null for none); null elsewhere.
    const labels = c.labels == null ? null : await pageLabels(doc);
    const got = {
      pages: doc.numPages,
      rotations,
      title,
      encrypted,
      info: keys,
      labels,
      initial_view: await initialView(doc, c),
    };
    const want = {
      pages: c.pages,
      rotations: c.rotations,
      title: c.title,
      encrypted: c.encrypted,
      info: c.info,
      labels: c.labels ?? null,
      // In the order initialView() writes, since the two are compared as JSON text.
      initial_view:
        (c.initial_view ?? null) === null
          ? null
          : {
              open_page: c.initial_view.open_page,
              page_mode: c.initial_view.page_mode,
              page_layout: c.initial_view.page_layout,
            },
    };
    if (c.outline !== undefined) {
      got.outline = await outlineOf(doc);
      want.outline = c.outline;
    }
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
