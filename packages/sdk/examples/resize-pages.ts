// Make every page Letter, scaling each page's content to fit.
//
// A page keeps its proportions: an A4 page is scaled down to Letter's height and centred, with
// a little room on each side. The bookmarks, links and named places that point into a page move
// with its content, so they still land on the same words.
import { Document } from "@laminapdf/sdk";

/** Letter, in points. */
const LETTER = { width: 612, height: 792 };

/** The document with every page on Letter paper, its content scaled to fit. Needs `init()`. */
export function resizePages(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const pages = doc.pages.map((page) => page.id);
  doc.resizePages(pages, LETTER);
  return doc.save();
}
