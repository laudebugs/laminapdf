// Make a new PDF from some pages of another.
//
// The new file holds only what its pages use, and its pages keep the labels they had (`iii`,
// `1`, `2` rather than 1, 2, 3).
import { Document } from "@laminapdf/sdk";

/** A new document of the third to the fifth page of `bytes`. Needs `init()`. */
export function extractPages(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const pages = doc.pages.slice(2, 5).map((page) => page.id);
  using part = doc.extractPages(pages);
  return part.save();
}
