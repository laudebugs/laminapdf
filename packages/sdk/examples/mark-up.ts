// Mark up a page: highlight a line of text, leave a note beside the chart, and propose a better word.
// The boxes are in page space (points from the page's top-left), as a viewer reports a text
// selection and a click. Each mark records who made it and when. In a tagged document a
// highlight's description, which screen readers read, is the text it marks.
import { Document } from "@laminapdf/sdk";

// Where the marks go on the example's input, the quarterly report, in points from the page's
// top-left: for your own file, take them from your viewer's selection and clicks.
/** The line "Widgets sold out in May.", as a selection gives it: `[x0, y0, x1, y1]`. */
const WIDGETS_LINE = [56, 358.241, 196.335, 373.223] as const;
/** The words "sold out" on that line. */
const SOLD_OUT = [114.902, 358.241, 156.713, 373.223] as const;
/** The note's spot, right of the chart. */
const BESIDE_CHART = [310, 140] as const;
/** The colour of a paper sticky note. */
const STICKY_YELLOW: [number, number, number] = [1, 0.82, 0];

/**
 * The document with a highlight, a note and a proposed word on its first page, saved
 * incrementally. `at` is when the marks were made, as a PDF date. Needs `init({ extract: true })`.
 */
export function markUp(bytes: Uint8Array, at: string): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const byMe = { author: "Ada Lovelace", created: at, modified: at };

  // In a tagged document the highlight's description is already the marked text; add who made it.
  const highlight = doc.markText(0, "highlight", [WIDGETS_LINE]);
  highlight.common = { ...highlight.common, ...byMe };

  const note = doc.noteAt(0, BESIDE_CHART, "Add June's figures?", "comment");
  note.common = { ...note.common, ...byMe, color: STICKY_YELLOW };

  doc.apply([
    { op: "addAnnot", args: highlight },
    { op: "addAnnot", args: note },
  ]);

  // "sold out" reads better as "ran out": a strike-out and a caret, applied as one undo step.
  doc.replaceText(0, [SOLD_OUT], "ran out", byMe);

  return doc.save({ mode: "incremental" });
}
