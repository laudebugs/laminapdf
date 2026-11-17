// Flatten a marked-up report for printing: every comment and mark becomes part of its page, so
// the copy prints and shows the same everywhere and no one can edit or remove the marks.
// The highlight, the note and the drawings stay where they were on the page; the annotations
// themselves go, with their pop-ups and replies. Saving as a full rewrite leaves no earlier
// revision that still holds them.
import { Document } from "@laminapdf/sdk";

/**
 * The document with every comment drawn into its page, as a printer would show it. Needs
 * `init({ extract: true })`.
 */
export function flattenComments(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const edited = doc.flattenAnnotations("all", "print");
  // Marks with no picture to keep, such as a file attachment, stay as they are.
  for (const warning of edited.warnings ?? []) {
    console.warn(warning.code);
  }
  // A full rewrite keeps only the flattened pages, not the revision with the comments.
  return doc.save({ mode: "fullRewrite" });
}
