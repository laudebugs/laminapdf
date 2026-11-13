// Record a review in a document's XMP metadata: properties of your own, beyond the title and
// authors, which a PDF/A file keeps without losing its claim.
// Each property is named by a namespace you own and a local name. In a PDF/A file, Lamina PDF also
// declares each new property in the metadata's extension schemas, as PDF/A requires, and the edit
// lists what it declared.
import { Document } from "@laminapdf/sdk";

/** The namespace of the review properties: a URI you own, so they can't clash with anyone else's. */
const REVIEW = "https://example.com/ns/review/1.0/";

/**
 * The document with the review recorded in its XMP metadata, saved incrementally.
 *
 * @param savedAt - the time of the save, as a PDF date (`D:20261110093000Z`) your app takes
 *   from its clock.
 */
export function editXmp(bytes: Uint8Array, savedAt: string): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);

  const reviewers = [
    { type: "text", value: "Ada Lovelace" },
    { type: "text", value: "Charles Babbage" },
  ] as const;
  const edits = [
    doc.setXmpProperty(REVIEW, "review", "status", { type: "text", value: "Approved" }),
    doc.setXmpProperty(REVIEW, "review", "reviewers", { type: "seq", value: [...reviewers] }),
    doc.setXmpProperty(REVIEW, "review", "round", { type: "integer", value: 2 }),
  ];
  for (const property of edits.flatMap((edited) => edited.declared)) {
    console.warn(`declared ${property.prefix}:${property.name} as ${property.valueType}`);
  }

  return doc.save({ mode: "incremental", metadataDate: savedAt });
}
