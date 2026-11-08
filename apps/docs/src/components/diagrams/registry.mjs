// Every diagram on the docs site, by name: its caption and its text description, the diagram's
// whole meaning in prose. Both the page (`<Diagram name="…" />`, src/components/Diagram.astro)
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
      "doc.transact, undo and redo write only to the overlay, and reading the document checks the overlay first. " +
      "A save then either appends the edits after the original bytes (incremental, which keeps signatures valid) " +
      "or writes every byte of a new file (fullRewrite).",
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
  return `${name.replace(/(^|-)([a-z])/g, (_, __, letter) => letter.toUpperCase())}.astro`;
}
