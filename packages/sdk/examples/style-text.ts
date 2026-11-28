// Change how text a page already shows looks: its weight, colour and underline.
//
// Finds the first run that says "world" and makes just that word bold, red and underlined,
// leaving the rest of the run as it was. A style names runs, or glyphs inside one, and changes
// only the fields it sets: a font family, a size, a colour, bold, italic, underline, raised or
// lowered text, and the alignment and spacing of the run's lines. The text itself doesn't change,
// so it still copies and searches as before. Bold and italic pick the face of a standard font
// such as Helvetica; for another font, name the bold face as the font.
import { Document } from "@laminapdf/sdk";

/** The document with "world" made bold, red and underlined on its first page. Needs `init({ extract: true })`. */
export function styleText(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const found = doc.content.textRuns(0).find((r) => !r.readOnly && r.text.includes("world"));
  if (!found) {
    throw new Error('no editable run says "world"');
  }
  // The run's glyphs are numbered from `run.start`; the word is five of them.
  const at = found.text.indexOf("world");
  const word = { ...found.run, start: found.run.start + at, end: found.run.start + at + 5 };
  doc.content.setTextStyle(0, [word], { bold: true, color: [0.8, 0, 0], underline: true });
  return doc.save();
}
