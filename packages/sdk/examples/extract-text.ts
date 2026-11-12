// Read a PDF's text in reading order: paragraphs, headings and table rows, a blank line between
// blocks, and nothing a reader can't see.
import { Document } from "@laminapdf/sdk";

/** The text of `bytes`, in reading order. Needs `init({ extract: true })`. */
export function textOf(bytes: Uint8Array): string {
  using doc = Document.open(bytes);
  return doc.extract({ format: "text" });
}
