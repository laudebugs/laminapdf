// Turn a scan's landscape pages upright, in one transaction, and save the edit after the original
// bytes. A page is named by its ID, which stays the same when other pages move.
import { Document, type Rotation } from "@laminapdf/sdk";

/** A quarter turn clockwise from `degrees`. */
function quarterTurn(degrees: number): Rotation {
  return ((degrees + 90) % 360) as Rotation;
}

/** `bytes` with every page wider than it is tall turned a quarter clockwise. Needs `init()`. */
export function rotateLandscapePages(bytes: Uint8Array): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);
  // A page's width and height are as displayed, with its current rotation applied.
  const landscape = doc.pages.filter((page) => page.width > page.height);
  // One transaction: every page turns, or none does, and one `undo()` reverts them all.
  doc.transact((tx) => {
    for (const page of landscape) tx.rotatePage(page.id, quarterTurn(page.rotation));
  });
  // An incremental save keeps the original bytes and appends the edit.
  return doc.save({ mode: "incremental" });
}
