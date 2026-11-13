// Give a document page labels, the names viewers show for its pages: "Cover" for the first,
// lowercase roman numerals for the front matter, and numbers from 1 where the body starts.
// A range names its first page by ID and labels the pages up to the next range. The edit is one
// op, saved incrementally after the original bytes.
import { Document } from "@laminapdf/sdk";

/** The document with a cover, roman front matter from its second page and numbers from its third. */
export function pageLabels(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const [cover, contents, body] = doc.pages;
  if (cover === undefined || contents === undefined || body === undefined) {
    throw new Error("the example needs three pages or more");
  }

  doc.setPageLabels([
    { page: cover.id, prefix: "Cover" },
    { page: contents.id, style: "lowerRoman" },
    { page: body.id, style: "decimal" },
  ]);

  // `doc.pages[i].label` now reads "Cover", "i", "1", "2", …
  return doc.save({ mode: "incremental" });
}
