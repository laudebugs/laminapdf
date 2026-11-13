// Merge PDFs into one, with a bookmark for each file.
//
// Each file's own bookmarks go under a bookmark named after it, its form fields keep working
// (renamed when two files use one name), and its pages keep their labels.
import { Document } from "@laminapdf/sdk";

/** One document of `files` (each a name and its bytes), in order, with a bookmark per file. Needs `init()`. */
export function mergeDocuments(files: readonly (readonly [string, Uint8Array])[]): Uint8Array<ArrayBuffer> {
  using merged = Document.merge(files.map(([title, bytes]) => ({ bytes, title })));
  return merged.save();
}
