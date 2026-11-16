// Draw on a page: a cloud around a row of the table, an arrow pointing at the chart, and a
// freehand underline below the total.
// The points and boxes are in page space (points from the page's top-left), as a viewer reports
// where a pointer went. Each mark records who drew it and when, and describes itself for screen
// readers.
import { type AnnotColor, type AnnotCommon, Document } from "@laminapdf/sdk";

// Where the marks go on the example's input, the quarterly report, in points from the page's
// top-left: for your own file, take them from your viewer's pointer.
/** Around the table's Q2 row: `[x0, y0, x1, y1]`. */
const Q2_ROW = [48, 317, 340, 338] as const;
/** From the empty space right of the chart to its right edge: `[x, y]` each. */
const ARROW = [
  [420, 180],
  [264, 200],
] as const;
/** A wavy underline below "Total sales", as a pen traced it. */
const UNDERLINE = [
  [56, 482],
  [72, 478],
  [88, 484],
  [104, 478],
  [120, 484],
  [136, 478],
  [152, 484],
  [168, 478],
  [180, 482],
] as const;
/** A reviewer's red pen. */
const RED: AnnotColor = [0.85, 0.1, 0.1];

/**
 * The document with a cloud, an arrow and an underline drawn on its first page, saved
 * incrementally. `at` is when the marks were drawn, as a PDF date.
 */
export function drawOnPage(bytes: Uint8Array, at: string): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const byMe = (contents: string): AnnotCommon => ({
    contents,
    author: "Ada Lovelace",
    created: at,
    modified: at,
    color: RED,
  });

  const cloud = doc.shapeAt(0, "square", Q2_ROW);
  if (cloud.spec.type === "shape") cloud.spec.border = { width: 1.5, style: { type: "cloudy", intensity: 0.5 } };
  cloud.common = byMe("Check the Q2 figures");

  const [from, to] = ARROW;
  const arrow = doc.lineAt(0, from, to, ["none", "closedArrow"]);
  if (arrow.spec.type === "line") {
    arrow.spec.border = { width: 2, style: { type: "solid" } };
    arrow.spec.interior = RED;
  }
  arrow.common = byMe("Label the chart's axes");

  const underline = doc.inkAt(0, [UNDERLINE], 1.5);
  underline.common = byMe("Underlined: the total");

  doc.apply([
    { op: "addAnnot", args: cloud },
    { op: "addAnnot", args: arrow },
    { op: "addAnnot", args: underline },
  ]);
  return doc.save({ mode: "incremental" });
}
