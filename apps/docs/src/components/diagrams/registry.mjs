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
