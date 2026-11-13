// Add links to a page: one on a box drawn over a chart, one on a line of text, then point the first somewhere else.
// The boxes are in page space (points from the page's top-left), as a viewer reports a drawn box
// and a text selection. Each link gets a description, which screen readers read. In a tagged
// document the link on text joins the structure tree around the words it covers.
import { Document } from "@laminapdf/sdk";

/** The document with two links added to its first page, saved incrementally. Needs `init({ extract: true })`. */
export function addLinks(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);

  // A box drawn over the chart.
  const onChart = doc.linkOnArea(0, [56, 140, 300, 240], { uri: "https://example.com/sales" });
  onChart.contents = "Sales figures";

  // The line "Widgets sold out in May.", as a selection gives it.
  const onText = doc.linkOnText(0, [56, 358.241, 196.335, 373.223], { uri: "https://example.com/widgets" });
  onText.contents = "Widget stock";

  const added = doc.apply([
    { op: "addLink", args: onChart },
    { op: "addLink", args: onText },
  ]);

  // The new links' IDs come back with the edit: change the first one's address.
  const chartLink = added.createdAnnotations[0];
  if (chartLink !== undefined) {
    doc.apply([{ op: "updateLink", args: { link: chartLink, target: { uri: "https://example.com/sales/2026" } } }]);
  }

  return doc.save({ mode: "incremental" });
}
