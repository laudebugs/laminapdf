// Mark up a page: highlight a line of text, leave a note beside the chart, and propose a better word.
// The boxes are in page space (points from the page's top-left), as a viewer reports a text
// selection and a click. Each mark records who made it and when. In a tagged document a
// highlight's description, which screen readers read, is the text it marks.
import { Document } from "@laminapdf/sdk";

/**
 * The document with a highlight, a note and a proposed word on its first page, saved
 * incrementally. `at` is when the marks were made, as a PDF date. Needs `init({ extract: true })`.
 */
export function markUp(bytes: Uint8Array, at: string): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const byMe = { author: "Ada Lovelace", created: at, modified: at };

  // The line "Widgets sold out in May.", as a selection gives it. In a tagged document the
  // highlight's description is already the marked text; add who made it.
  const highlight = doc.markText(0, "highlight", [[56, 358.241, 196.335, 373.223]]);
  highlight.common = { ...highlight.common, ...byMe };

  // A note to the right of the chart, in the colour of a paper sticky note.
  const note = doc.noteAt(0, [310, 140], "Add June's figures?", "comment");
  note.common = { ...note.common, author: byMe.author, created: at, color: [1, 0.82, 0] };

  doc.apply([
    { op: "addAnnot", args: highlight },
    { op: "addAnnot", args: note },
  ]);

  // "sold out" reads better as "ran out": a strike-out and a caret, applied as one undo step.
  doc.replaceText(0, [[114.902, 358.241, 156.713, 373.223]], "ran out", byMe);

  return doc.save({ mode: "incremental" });
}
