// The pdf.js half of the round-trip suite (todo B3c, ADR 0018 tier 7): opens every output
// listed in <out-dir>/manifest.json (written by laminapdf-save's `roundtrip` example) and checks
// the page count, each page's rotation, the Info /Title, whether the file is encrypted (opened with
// the case's password, and refused without it), for the
// outline cases the outline (depth, title, page per item), in the setInitialView cases the
// page the file opens on, its page mode and its page layout, in the attachment cases each
// attached file's key, name and SHA-256 (pdf.js lists the files lamina adds), in the form cases
// each filled field's value, read-only and required flags, and in the cases that list phrases (the comment summaries) how often each occurs in the text pdf.js extracts,
// against it, and in the page-mark cases whether the layers panel lists the hideable watermark (getOptionalContentConfig). `cargo xtask roundtrip`
// runs it; it prints one JSON line ({ tool, version, checked, failures }) and exits 1 on any
// failure.
//
//     node tools/roundtrip/pdfjs.mjs <path to node_modules/pdfjs-dist> <out-dir>
import { createHash } from "node:crypto";
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

/** Where a Link annotation goes: ["uri", address], ["page", index] or ["other"]. */
async function linkTarget(doc, annotation) {
  if (typeof annotation.url === "string") return ["uri", annotation.url];
  let dest = annotation.dest;
  if (typeof dest === "string") dest = await doc.getDestination(dest);
  if (!Array.isArray(dest) || dest[0] === null || typeof dest[0] !== "object") return ["other"];
  const index = await doc.getPageIndex(dest[0]).catch(() => null);
  return index === null ? ["other"] : ["page", index];
}

/** Each page's links, sorted: ["page", index], ["uri", address] or ["other"], as lamina lists them. */
async function linksOf(doc) {
  const pages = [];
  for (let i = 1; i <= doc.numPages; i++) {
    const page = await doc.getPage(i);
    const found = [];
    for (const a of await page.getAnnotations()) {
      if (a.subtype === "Link") found.push(await linkTarget(doc, a));
    }
    const key = (x) => JSON.stringify(x);
    pages.push(found.sort((x, y) => (key(x) < key(y) ? -1 : key(x) > key(y) ? 1 : 0)));
  }
  return pages;
}

/** The subtypes lamina's mark cases make. */
const MARKS = new Set([
  "Highlight",
  "Underline",
  "StrikeOut",
  "Squiggly",
  "Text",
  "Caret",
  "Ink",
  "Square",
  "Circle",
  "Line",
  "Polygon",
  "PolyLine",
  "FreeText",
  "Stamp",
  "FileAttachment",
]);

/**
 * How often each of `phrases` occurs in the document's text, with the spaces left out of both:
 * pdf.js starts a new item (and so a space) where it finds a Devanagari vowel sign moved before
 * its consonant, though the characters are the ones that were typed.
 */
async function phrasesOf(doc, phrases) {
  const items = [];
  for (let i = 1; i <= doc.numPages; i++) {
    const content = await (await doc.getPage(i)).getTextContent();
    items.push(...content.items.map((item) => item.str ?? ""));
  }
  const bare = (text) => text.split(/\s+/).join("");
  const text = bare(items.join(" "));
  return Object.fromEntries(phrases.map((p) => [p, text.split(bare(p)).length - 1]));
}

/** Each page's text markup, notes, carets and drawing marks, sorted: [subtype, contents], as lamina lists them. */
async function marksOf(doc) {
  const pages = [];
  for (let i = 1; i <= doc.numPages; i++) {
    const page = await doc.getPage(i);
    const found = [];
    for (const a of await page.getAnnotations()) {
      if (!MARKS.has(a.subtype)) continue;
      const mark = [a.subtype, a.contentsObj?.str ?? ""];
      // A file attachment's bytes, which pdf.js reads on request by the annotation's `fileId`.
      if (a.subtype === "FileAttachment") {
        const content = await doc.getAttachmentContent(a.fileId);
        mark.push(createHash("sha256").update(content).digest("hex"));
      }
      found.push(mark);
    }
    const key = (x) => JSON.stringify(x);
    pages.push(found.sort((x, y) => (key(x) < key(y) ? -1 : key(x) > key(y) ? 1 : 0)));
  }
  return pages;
}

/**
 * For each object an addContentObject case added, whether the first page paints a form with its
 * box. pdf.js gives a form's `/BBox` with the operator that begins painting it.
 */
async function addedDrawn(doc, added) {
  const ops = await (await doc.getPage(1)).getOperatorList();
  const boxes = ops.fnArray.flatMap((fn, i) =>
    fn === pdfjs.OPS.paintFormXObjectBegin ? [Array.from(ops.argsArray[i]?.[1] ?? [])] : [],
  );
  return added.map(({ bbox }) => boxes.some((box) => bbox.every((v, k) => Math.abs(v - box[k]) < 0.01)));
}

/** The /EmbeddedFiles tree's files, sorted by key: { key, name, sha256 }, as lamina lists them. */
async function attachmentsOf(doc) {
  // pdf.js lists the files by their tree key (a Map, or null when there are none) and gives each
  // one's bytes on request.
  const files = (await doc.getAttachments()) ?? new Map();
  const out = [];
  for (const [key, file] of files) {
    const content = await doc.getAttachmentContent(key);
    out.push({ key, name: file.filename, sha256: createHash("sha256").update(content).digest("hex") });
  }
  return out.sort((x, y) => (JSON.stringify(x.key) < JSON.stringify(y.key) ? -1 : 1));
}

/**
 * The fields `names` lists as their widgets show them, sorted by name: [name, value, read-only,
 * required], a field's first widget read, a one-option choice as its option.
 */
async function fieldsOf(doc, names) {
  const found = new Map();
  for (let i = 1; i <= doc.numPages; i++) {
    for (const a of await (await doc.getPage(i)).getAnnotations()) {
      if (a.subtype !== "Widget" || !names.includes(a.fieldName) || found.has(a.fieldName)) continue;
      const value = Array.isArray(a.fieldValue) ? a.fieldValue.join(", ") : (a.fieldValue ?? "");
      found.set(a.fieldName, [a.fieldName, value, a.readOnly === true, a.required === true]);
    }
  }
  return [...found.values()].sort((x, y) => (JSON.stringify(x[0]) < JSON.stringify(y[0]) ? -1 : 1));
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
    // pdf.js takes ownership of the buffer it is given, so the refusal check below gets a copy.
    const spare = c.password != null ? data.slice() : undefined;
    const task = pdfjs.getDocument({ data, verbosity: 0, stopAtErrors: true, password: c.password ?? undefined });
    const doc = await task.promise;
    if (c.password != null) {
      // A file lamina encrypted opens only with its password (and not without one).
      try {
        const bare = pdfjs.getDocument({ data: spare, verbosity: 0, stopAtErrors: true });
        await bare.promise;
        await bare.destroy();
        failures.push(`${c.file}: opened without its password`);
      } catch (error) {
        if (error?.name !== "PasswordException") throw error;
      }
    }
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
    if (c.links !== undefined) {
      got.links = await linksOf(doc);
      want.links = c.links;
    }
    if (c.marks !== undefined) {
      got.marks = await marksOf(doc);
      want.marks = c.marks;
    }
    if (c.attachments !== undefined && c.attachments !== null) {
      got.attachments = await attachmentsOf(doc);
      want.attachments = c.attachments;
    }
    if (c.added !== undefined && c.added !== null) {
      got.added = await addedDrawn(doc, c.added);
      want.added = c.added.map(() => true);
    }
    if (c.fields !== undefined) {
      got.fields = await fieldsOf(
        doc,
        c.fields.map((f) => f[0]),
      );
      want.fields = c.fields;
    }
    if (c.flattened !== undefined) {
      // Flattened fields have no widget left to find.
      got.flattened = await fieldsOf(doc, c.flattened);
      want.flattened = [];
    }
    if (c.layers !== undefined && c.layers !== null) {
      // The layers panel lists each of `layers`, on, among the file's own (pdf.js's optional content config).
      const config = await doc.getOptionalContentConfig();
      const shown = [...config].filter(([, group]) => group.visible).map(([, group]) => group.name);
      got.layers = c.layers.filter((name) => shown.includes(name));
      want.layers = c.layers;
    }
    if (c.phrases !== undefined && c.phrases !== null) {
      got.phrases = await phrasesOf(doc, Object.keys(c.phrases));
      want.phrases = c.phrases;
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
