// Choose how a PDF opens: on a given page at the page's width, with its bookmarks shown and its title in the window.
// The edit changes only what the file asks a reader to do when it opens it. A file whose open
// action runs a script keeps that action: the edit leaves the opening page alone and changes the
// rest.
import { Document, type InitialViewPatch } from "@laminapdf/sdk";

/**
 * The document set to open on page `index` (0-based) at the page's width, with the bookmarks
 * panel and the document's title in the window, saved incrementally. Needs
 * `init({ extract: true })`.
 */
export function openOn(bytes: Uint8Array, index: number): Uint8Array<ArrayBuffer> {
  using doc = Document.open(bytes);

  const view: InitialViewPatch = { pageMode: "outlines", preferences: { displayDocTitle: true } };
  // An open action that runs a script or another action is never replaced, so leave the page out.
  const open = doc.initialView().open;
  if (open.kind === "action") {
    console.warn(`the file runs a ${open.action} action when it opens; keeping it`);
  } else {
    view.open = { page: doc.page(index).id, zoom: "fitWidth" };
  }
  doc.setInitialView(view);
  return doc.save({ mode: "incremental" });
}
