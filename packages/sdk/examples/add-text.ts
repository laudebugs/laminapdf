// Type on a page as page content: a note in Helvetica with one bold word, and a greeting in two
// scripts set in fonts given by their bytes, the second font drawing the Arabic the first lacks.
// The text becomes part of the page: it prints, every reader draws it, and it copies and searches
// as typed. The boxes are in page space (points from the page's top-left), as a viewer reports
// where a pointer went. In this tagged report each text is a paragraph that screen readers read.
// Fonts given by their bytes need the WebAssembly text module; Helvetica, Times and Courier need
// no bytes.
import { Document } from "@laminapdf/sdk";

// Where the texts go on the example's input, the quarterly report, in points from the page's
// top-left: for your own file, take them from your viewer's pointer.
/** The note: `[x0, y0, x1, y1]`. */
const NOTE = [56, 690, 300, 712] as const;
/** The greeting, under the note. */
const GREETING = [56, 718, 300, 746] as const;
/** The size of the greeting, in points. */
const GREETING_SIZE = 16;

/**
 * The document with a note and a greeting typed on its first page, saved incrementally. `at` is
 * when they were typed, as a PDF date.
 */
export function addText(bytes: Uint8Array, latin: Uint8Array, arabic: Uint8Array, at: string): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);

  doc.content.addText(0, {
    id: "note",
    rect: NOTE,
    runs: [{ text: "Reviewed " }, { text: "twice", bold: true }],
    fonts: ["helvetica"],
    modified: at,
  });

  // The first font draws the Latin, the second what the first lacks.
  doc.content.addText(0, {
    id: "greeting",
    rect: GREETING,
    runs: [{ text: "Hello مرحبا" }],
    fonts: [doc.addBlob(latin), doc.addBlob(arabic)],
    size: GREETING_SIZE,
    modified: at,
  });

  return doc.save({ mode: "incremental" });
}
