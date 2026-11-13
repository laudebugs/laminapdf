// Duplicate a form page, so a second copy can be filled in on its own.
//
// A copy shares its page's content but gets new annotations, so filling in a field on one page
// leaves the other alone: each copied field gets a name of its own, `<name>#2` (or the next free
// number). The edit lists each one, and each signature field it left out.
import { Document } from "@laminapdf/sdk";

/** The document with a copy of its first page added at the end. Needs `init()`. */
export function duplicatePages(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const first = doc.pages[0]?.id;
  if (first === undefined) {
    throw new Error("the document has no pages");
  }
  const edited = doc.duplicatePages([first], "end");
  for (const field of edited.fields ?? []) {
    if (field.outcome === "renamed") {
      console.warn(`the copy's field is ${field.name}`);
    } else {
      console.warn(`field ${field.field} was not copied: it is a signature`);
    }
  }
  return doc.save();
}
