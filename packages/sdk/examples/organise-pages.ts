// Organise a document's pages: delete one, move one to the front, add a blank page at the end
// and turn two pages a quarter clockwise, in one edit that one undo reverts.
//
// Pages are named by their IDs, which stay the same while other pages move. Deleting a page
// points the bookmarks, links and named destinations that went to it at the nearest page left,
// and the edit lists each one, so an app can say what moved.
import { Document } from "@laminapdf/sdk";

/** US Letter, in points. */
const LETTER = { width: 612, height: 792 };

/**
 * The document with its third page deleted, its last page moved to the front, a blank Letter
 * page added at the end and its first two pages turned a quarter clockwise. Needs `init()`.
 */
export function organisePages(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  const ids = doc.pages.map((page) => page.id);
  const [first, second, third] = ids;
  const last = ids.at(-1);
  if (first === undefined || second === undefined || third === undefined || last === undefined) {
    throw new Error("the document needs at least three pages");
  }

  // One transaction: every op applies, in order, or none does. Each op names its pages by the
  // IDs read before the edit, which the moves and the deletion leave as they are.
  const edited = doc.apply([
    { op: "deletePages", args: { pages: [third] } },
    { op: "movePages", args: { pages: [last], to: "start" } },
    { op: "insertBlankPages", args: { at: "end", count: 1, size: LETTER, rotate: 0 } },
    { op: "rotatePages", args: { pages: [first, second], angle: 90, mode: "turn" } },
  ]);
  for (const reference of edited.references) {
    if (reference.outcome === "retargeted") {
      console.warn(`${reference.kind} now points to page ${reference.to}`);
    }
  }
  return doc.save();
}
