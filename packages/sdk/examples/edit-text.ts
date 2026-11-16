// Change the text a page already shows, in the font it is set in.
//
// Lists the first page's runs of text, then rewrites the first one that says "world" to say
// "Lamina PDF" instead. The new text uses the font the run was set in, so every character must
// be one that font can draw; a character it can't is refused with `missing-glyph`, and a run
// that can't be edited at all (a Type 3 font, vertical text) says why. The text after the run
// moves by the difference in width; the lines below stay where they were.
import { Document } from "@laminapdf/sdk";

/** The document with "world" rewritten on its first page. Needs `init({ extract: true })`. */
export function editText(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const runs = doc.content.textRuns(0);
  for (const run of runs) {
    const readOnly = run.readOnly ? ` (read-only: ${run.readOnly})` : "";
    console.log(`line ${run.line}: ${JSON.stringify(run.text)} in ${run.fontName} ${run.fontSize}${readOnly}`);
  }
  const run = runs.find((r) => !r.readOnly && r.text.includes("world"));
  if (!run) {
    throw new Error('no editable run says "world"');
  }
  doc.content.editText(0, run.run, run.text.replace("world", "Lamina PDF"));
  return doc.save();
}
