// Edit a document's bookmarks and named destinations: add a bookmark, move one, and name a place links can jump to.
// A new "Summary" bookmark goes first, in bold, showing the top of the last page. The place comes
// from page space, as a viewer reports it, and becomes the destination the bookmark stores. The
// same place gets the name `summary`, the destination `chapter-2` is renamed with every bookmark
// that uses it, and the "Website" bookmark moves to the end of the "Chapters" bookmark, when the
// file has them.
import { Document } from "@laminapdf/sdk";

/** The document with its bookmarks edited, saved incrementally. Needs `init({ extract: true })`. */
export function editBookmarks(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);

  // The top of the last page, in page space (points from the page's top-left).
  const last = Math.max(doc.pageCount - 1, 0);
  const summary = doc.destinationAt(last, { kind: "at", x: 0, y: 0 });

  doc.apply([
    {
      op: "addOutlineItem",
      args: {
        parent: null,
        at: "first",
        title: "Summary",
        target: { destination: summary },
        style: { bold: true },
      },
    },
    { op: "setNamedDestination", args: { name: "summary", destination: summary } },
  ]);

  // Rename a destination, if the file has it: bookmarks and links that use it follow.
  if (doc.namedDestinations().some((d) => d.name === "chapter-2")) {
    doc.apply([{ op: "renameNamedDestination", args: { from: "chapter-2", to: "methods" } }]);
  }

  const outline = doc.outline();
  const find = (title: string) => outline.find((node) => node.title === title)?.id;
  const website = find("Website");
  const chapters = find("Chapters");
  if (website !== undefined && chapters !== undefined) {
    doc.apply([{ op: "moveOutlineItem", args: { item: website, parent: chapters, at: "last" } }]);
  }

  return doc.save({ mode: "incremental" });
}
