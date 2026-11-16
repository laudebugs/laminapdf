// Add a clickable contents page to a PDF, made from its bookmarks.
// The page lists the bookmarks that go to a page, two levels deep, each with the label its page
// shows and a link to it, and gets a bookmark of its own. It's one edit, so one undo takes it out
// again, and it saves incrementally.
import { Document, Font } from "@laminapdf/sdk";

/**
 * The document with a contents page before the first page, set in the font `font` (embedded, so
 * a PDF/A or PDF/UA-1 file keeps its claim), saved incrementally. Needs `init({ text: true })`.
 */
export function addContents(bytes: Uint8Array, font: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  doc.createContentsPage({ font: Font.fromBytes(font), bookmark: true });
  return doc.save({ mode: "incremental" });
}
