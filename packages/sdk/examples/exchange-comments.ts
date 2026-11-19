// Send a PDF's comments to a reviewer who uses another app, and bring their comments back:
// export them as XFDF, which Acrobat and most PDF apps import, and import the XFDF file they
// send back. The import keeps the comments already in the file and adds the new ones, with
// their replies and statuses, as one change that a single undo takes back.
import { Document } from "@laminapdf/sdk";

/**
 * The document's comments as XFDF, recording the file's name so an import can tell when it is
 * given to another PDF. Needs `init({ extract: true })`.
 */
export function exportComments(bytes: Uint8Array, name: string): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  return doc.exportAnnotations({ format: "xfdf", file: name });
}

/** The document with a reviewer's XFDF or FDF comments added to its own. */
export function importComments(bytes: Uint8Array, xfdf: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const { summary } = doc.importAnnotations(xfdf, { mode: "merge" });
  console.log(`${summary.added} comments added, ${summary.matched} already here`);
  // A sound, a form field's value or a comment for a missing page is left out, and said.
  for (const warning of summary.warnings) {
    console.warn(warning.code, warning.kind ?? "", warning.reason ?? "");
  }
  return doc.save();
}
