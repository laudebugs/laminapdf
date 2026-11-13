// Copy a page from another PDF into this one.
//
// The copy keeps its annotations and form fields. A field whose name the document already uses
// takes the next free `<name>#2`, and a link to a page that didn't come along is removed; the
// edit lists both.
import { Document } from "@laminapdf/sdk";

/** `bytes` with the third page of `other` added at the end. Needs `init()`. */
export function insertPages(bytes: Uint8Array, other: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const source = doc.addSource(other);
  const third = source.pages[2];
  if (third === undefined) {
    throw new Error("the other file has no third page");
  }
  const edited = doc.insertPagesFrom(source.id, [third], "end");
  for (const field of edited.fields ?? []) {
    if (field.outcome === "renamed") {
      console.warn(`a copied field was renamed ${field.name}`);
    }
  }
  console.warn(`${edited.linksRemoved?.length ?? 0} links removed`);
  return doc.save();
}
