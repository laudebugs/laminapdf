// Approve a report: a stamp with the approver's name at the top of its first page, and the
// table's figures attached as a CSV file beside the table.
// The box and the point are in page space (points from the page's top-left), as a viewer
// reports where a pointer went. The stamp's lines are set in Helvetica, which every reader has,
// so the WebAssembly extract module can draw it too.
import { type AnnotCommon, Document } from "@laminapdf/sdk";

// Where the marks go on the example's input, the quarterly report, in points from the page's
// top-left: for your own file, take them from your viewer's pointer.
/** The stamp, right of the title: `[x0, y0, x1, y1]`. */
const STAMP = [370, 56, 540, 106] as const;
/** The attachment's icon, right of the table's last column. */
const ICON = [370, 282] as const;
/** The table's figures. */
const FIGURES = "Quarter,Units,Sales\nQ1,120,1200\nQ2,310,3100\n";

/**
 * The document approved: a stamp and an attached file on its first page, saved incrementally.
 * `at` is when it was approved, as a PDF date.
 */
export function stampAndAttach(bytes: Uint8Array, at: string): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const byMe = (): AnnotCommon => ({ author: "Ada Lovelace", created: at, modified: at });

  // The stamp's lines are written into it now and never change, as on a rubber stamp.
  const stamp = doc.stampAt(0, STAMP, { dynamic: { lines: ["APPROVED", "by Ada Lovelace"], style: "green" } });
  stamp.common = byMe();

  const attach = doc.attachFileAt(0, ICON, "figures.csv", new TextEncoder().encode(FIGURES), "text/csv");
  if (attach.spec.type === "fileAttachment") {
    attach.spec.icon = "graph";
    attach.spec.file.description = "The table's figures";
    attach.spec.file.modified = at;
  }
  attach.common = byMe();

  doc.apply([
    { op: "addAnnot", args: stamp },
    { op: "addAnnot", args: attach },
  ]);
  return doc.save({ mode: "incremental" });
}
