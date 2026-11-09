// Every diagram on the docs site, by name: its caption and its text description, the diagram's
// whole meaning in prose, with API names in backticks (code in the Markdown twin; the page strips
// the backticks for screen readers). Both the page (`<Diagram name="…" />`, src/components/Diagram.astro)
// and the page's Markdown twin (plugins/markdown.mjs) read them from here, so the picture, its
// description for screen readers and the twin that agents read never drift apart. Each name has a
// drawing in this folder: the name in PascalCase, `open-edit-save` → `OpenEditSave.astro`.

/** @typedef {{ caption: string, text: string }} DiagramText */

/** @type {Record<string, DiagramText>} */
export const DIAGRAMS = {
  "open-edit-save": {
    caption: "Edits go into an overlay; saving appends them or writes a new file.",
    text:
      "Opening a PDF gives you a document that holds the file's original bytes, which never change, and an overlay for your edits. " +
      "`doc.transact`, `undo` and `redo` write only to the overlay, and reading the document reads the overlay first. " +
      "A save then either appends the edits after the original bytes (`incremental`) or writes every byte of a new file (`fullRewrite`). " +
      "An incremental save leaves the original bytes untouched, so signatures over them stay valid.",
  },
  "save-modes": {
    caption: "An incremental save appends a revision; a full rewrite writes every byte again.",
    text:
      "An incremental save (`incremental`) keeps the original bytes and appends one revision after them. " +
      "A signature over those bytes stays valid for the edits it allows. " +
      "A full rewrite (`fullRewrite`) writes every byte again as one new file. Signatures over the original bytes are no longer valid, so a signed file saves this way only when you pass `invalidateSignatures: true`, which removes them.",
  },
  "save-decision": {
    caption: "What a file allows, and the options you pass, decide its save mode or the error the save fails with.",
    text:
      "Every save plans before it writes, and an unchanged document comes first: in `auto` and `incremental` mode it saves incrementally and returns the bytes you opened, whatever else is true of it. " +
      "A signed file saves incrementally, with the edits its signatures allow. Any other save fails with `would-invalidate-signatures` unless you pass `invalidateSignatures: true`, which saves anyway and removes the signatures that no longer hold. " +
      "A repaired file needs a full rewrite: `auto` picks `fullRewrite`, and asking for `incremental` fails with `requires-rewrite`. " +
      "An encrypted file you edited saves as `fullRewrite` without its encryption when you pass `removeEncryption: true`; otherwise every mode fails with `encrypted-base`. " +
      "An edited file in two of these states meets both rules: a signed file that was repaired fails until you pass `invalidateSignatures: true`.",
  },
  "package-map": {
    caption: "Each JavaScript package and what it depends on, beside the Rust API.",
    text:
      "In JavaScript, `@laminapdf/sdk` is the TypeScript API, and it depends on `@laminapdf/wasm`, whose WebAssembly the SDK loads as three modules: base, extract and text. " +
      "`@laminapdf/viewer`, a web PDF viewer, depends on `@laminapdf/sdk`, on `@laminapdf/render`, the interface that renderers implement, and on both renderers. " +
      "Those renderers are `@laminapdf/render-pdfium`, the default, and `@laminapdf/render-hayro`, which also uses `@laminapdf/wasm`. " +
      "In Rust, the API is one crate, `laminapdf`, with features instead of modules.",
  },
  "package-map-modules": {
    caption: "The three modules come in @laminapdf/wasm, under @laminapdf/sdk.",
    text:
      "`@laminapdf/sdk` depends on `@laminapdf/wasm`, whose WebAssembly the SDK loads as three modules: base, extract and text. " +
      "`@laminapdf/viewer` depends on `@laminapdf/sdk`, on `@laminapdf/render` and on both renderers that implement it, `@laminapdf/render-pdfium` and `@laminapdf/render-hayro`. " +
      "In Rust, the API is one crate, `laminapdf`, with features instead of modules.",
  },
  "worker-calls": {
    caption:
      "Each call crosses to the Web Worker as a message; an abort skips a call not started and stops a running save.",
    text:
      "Your page's client sends each call, such as `await doc.pageCount()`, to the Web Worker as a message. " +
      "There `@laminapdf/sdk` runs one call at a time, and each answer comes back as a Promise; saved bytes and page text come back transferred rather than copied. " +
      "When a call's `AbortSignal` aborts, the call rejects with `cancelled`. A call not started never runs, and a running `save` or `saveWithReport` stops at its next step. " +
      "Any other call runs to the end, though the byte fetches of a file opened with `openUrl` stop.",
  },
  "errors-and-warnings": {
    caption: "A call stops with an error or completes with warnings, and both carry the same core fields.",
    text:
      "A call either stops with a `LaminaPdfError` or completes, with any warnings on `doc.warnings`. " +
      "Errors and warnings carry the same fields: `code`, `kind`, `message`, `hint`, `location`, `data` and `docsUrl`; an error can also carry `didYouMean`. " +
      "Text from the document goes only in `data`, never in `message` or `hint`.",
  },
  "undo-timeline": {
    caption: "One undo reverts a whole transaction; redo applies it again.",
    text:
      "Three transactions run in order: the first runs `setInfo` and `rotatePage`, the second `rotatePage`, and the third both. " +
      "`doc.undo()` reverts the last transaction as a whole, and `doc.redo()` applies it again. " +
      "A new transaction clears what redo could apply.",
  },
  "drawing-pages": {
    caption: "A renderer draws the pixels; Lamina PDF reads the text from the same bytes.",
    text:
      "Two readers use your PDF's bytes. To draw, a render request names the page, the scale, the tile to draw (its clip) and a rotation. " +
      "The request becomes a tile plan, which fixes the size in pixels, and a renderer, PDFium or hayro, draws the tile as RGBA pixels. " +
      "Text, links and selection never come from the renderer. Lamina PDF reads them from its own model of the same bytes, the model that also edits and saves the file.",
  },
  "range-loading": {
    caption: "A read that needs missing bytes fetches them and runs again.",
    text:
      "`Document.openRange` opens a remote file from the few byte ranges the open needs. " +
      "A later read, such as `doc.pageInfo(0)`, may need bytes that haven't arrived: it then throws `data-pending`, naming the ranges it needs. " +
      "`doc.ensure` catches the error, fetches those ranges through the source (`httpRangeSource`), supplies them to the document and runs the read again. " +
      "Once the bytes are there, the read returns its result.",
  },
  "page-space": {
    caption:
      "Page space starts at the top-left corner you see; PDF's user space starts at the bottom-left, before rotation.",
    text:
      "On an upright US Letter page, 612 by 792 points, page space starts at the top-left corner with y going down, and PDF's user space starts at the bottom-left corner with y going up. " +
      "The point `[72, 96]` is one inch from the left edge and 96 points from the top, on the baseline of a heading whose box runs from `[72, 72]` to `[272, 100]`. " +
      "With `/Rotate 90` a viewer shows the same page on its side, 792 by 612 points. " +
      "Page space still starts at the top-left corner you see, so `[72, 96]` lands in the same place on the screen. " +
      "User space starts at that corner too, with its x axis going down and its y axis going right. " +
      "The heading in the file now runs down the right edge, from `[692, 72]` to `[720, 272]` in page space.",
  },
  "extract-pipeline": {
    caption:
      "Extraction joins a page's text runs into lines and blocks, orders them, then writes Markdown, JSON and chunks.",
    text:
      "Extraction starts from each page's text runs. " +
      "It sets aside runs a reader can't see, and runs the file marks as artifacts, into `dropped` with their reason: `invisible`, `off-page`, `tiny`, `clipped`, `low-contrast`, `covered` or `artifact`. " +
      "The other runs are joined into lines, and the lines into blocks: paragraphs, headings and tables. " +
      "Once every page is read, it also sets aside headers, footers and page numbers that repeat across pages, as `running-header`, `running-footer` or `page-number`. " +
      "Reading order then puts the blocks in sequence: from the file's structure tree when it is tagged and the tree covers the page's text, else by cutting the page into columns. " +
      "The ordered blocks become the three outputs: Markdown, the `laminapdf.extract/1` JSON and chunks.",
  },
  "reading-order": {
    caption: "On a two-column page, a title across both columns comes first, then the left column, then the right.",
    text:
      "A page has a title across both columns and three blocks in each column. " +
      "Extraction numbers the title 1, the left column's blocks 2 to 4 from top to bottom, and the right column's blocks 5 to 7.",
  },
};

/**
 * The caption and text description of the diagram `name`.
 *
 * @param {string} name
 * @returns {DiagramText}
 */
export function diagramText(name) {
  const entry = DIAGRAMS[name];
  if (entry === undefined) throw new Error(`no diagram named "${name}" in src/components/diagrams/registry.mjs`);
  return entry;
}

/**
 * The file that draws the diagram `name`: `open-edit-save` → `OpenEditSave.astro`.
 *
 * @param {string} name
 * @returns {string}
 */
export function drawingFile(name) {
  return `${name.replace(/(^|-)([a-z0-9])/g, (_, __, first) => first.toUpperCase())}.astro`;
}
