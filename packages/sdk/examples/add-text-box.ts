// Type on a page: a callout pointing at the chart, and a filled note beside the total with one
// word in bold.
// The boxes and points are in page space (points from the page's top-left), as a viewer reports
// where a pointer went. The boxes are in Helvetica, which every reader has, so the WebAssembly
// extract module can draw them too.
import { type AnnotColor, type AnnotCommon, Document, type RichText } from "@laminapdf/sdk";

// Where the boxes go on the example's input, the quarterly report, in points from the page's
// top-left: for your own file, take them from your viewer's pointer.
/** The callout's box, in the space right of the chart: `[x0, y0, x1, y1]`. */
const CALLOUT = [330, 168, 500, 190] as const;
/** The callout's line: from the chart's right edge, through a knee, to the box. */
const LEADER = [
  [257, 200],
  [300, 179],
  [330, 179],
] as const;
/** The note beside the total. */
const NOTE = [300, 452, 520, 486] as const;
/** A pale yellow, like a sticky note. */
const YELLOW: AnnotColor = [1, 0.96, 0.7];

/**
 * The document with a callout and a note typed on its first page, saved incrementally. `at` is
 * when the boxes were typed, as a PDF date.
 */
export function addTextBox(bytes: Uint8Array, at: string): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const byMe = (): AnnotCommon => ({ author: "Ada Lovelace", created: at, modified: at });

  const callout = doc.calloutAt(0, CALLOUT, LEADER, "Label the chart's axes");
  callout.common = byMe();

  const text: RichText = [
    [{ text: "Check: " }, { text: "€ 4,300", bold: true }, { text: " should match the table's total." }],
  ];
  const note = doc.textBoxAt(0, NOTE, text);
  if (note.spec.type === "freeText") note.spec.fill = YELLOW;
  note.common = byMe();

  doc.apply([
    { op: "addAnnot", args: callout },
    { op: "addAnnot", args: note },
  ]);
  return doc.save({ mode: "incremental" });
}
