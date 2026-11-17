// Edit an accessible (tagged, PDF/UA) document and keep its claim, or save without it.
//
// Deletes the summary paragraph of the report from `pdf-ua-report` and renames its table's
// heading. Lamina PDF keeps the tags in step: the deleted paragraph leaves the structure tree,
// and the heading's element still finds its text. Before saving, the plan lists the PDF/A and
// PDF/UA claims the edits broke; this example removes only those claims, so the file doesn't
// promise what it no longer meets.
import { Document } from "@laminapdf/sdk";

/** The report with its summary deleted and its table's heading renamed. Needs `init({ extract: true })`. */
export function editTaggedPdf(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const runs = doc.content.textRuns(0);
  const summary = runs.find((r) => r.text.startsWith("Sales grew"));
  const heading = runs.find((r) => r.text === "Sales by quarter");
  if (!summary || !heading) {
    throw new Error("not the report from pdf-ua-report");
  }
  // Each edit names its run as the page is before it: the heading comes later on the page, so
  // renaming it first leaves the summary's name as it was.
  doc.content.editText(0, heading.run, "Quarterly sales");
  // Empty text deletes the run, and the paragraph it was.
  doc.content.editText(0, summary.run, "");

  // A `pdfa-…` code (and `imported-pdfa-unchecked`) breaks the PDF/A claim; the others, the
  // PDF/UA claim.
  const broken = doc.plan().conformanceWarnings.map((w) => w.code);
  const pdfa = broken.some((code) => code.includes("pdfa"));
  const pdfua = broken.some((code) => !code.includes("pdfa"));
  if (pdfa || pdfua) {
    console.warn(`the edits broke a claim: ${broken.join(", ")}`);
    doc.removeConformanceClaims({ pdfa, pdfua });
  }
  return doc.save();
}
