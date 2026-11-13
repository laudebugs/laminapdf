// Split a PDF into one file per chapter.
//
// The file is cut where each top-level bookmark's page begins, and each part keeps the
// bookmarks of its chapter.
import { Document } from "@laminapdf/sdk";

/** One file per chapter of `bytes`: a part from each top-level bookmark's page to the next. Needs `init()`. */
export function splitDocument(bytes: Uint8Array): Uint8Array<ArrayBuffer>[] {
  using doc = Document.open(bytes);
  const chapters = doc.split({ bookmarks: 1 });
  try {
    return chapters.map((chapter) => chapter.save());
  } finally {
    for (const chapter of chapters) chapter.free();
  }
}
