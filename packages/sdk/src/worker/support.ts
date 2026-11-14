// Which members of the sync API the worker client serves, with their arity (0 for a
// getter): the client splits a call's arguments into the method's and the trailing
// `CallOptions`. Each map must name every member
// of its class except the excluded ones (`satisfies` makes a missing or extra name a `tsc`
// error), so adding a sync method forces a decision here: serve it, or exclude it with a reason.
import type { Document, PageRef } from "../document.js";

/**
 * Document members with no remote twin:
 * - `transact` takes a callback, which can't cross to a worker: use `apply` with ops;
 * - `saveJob` returns a job the caller steps: the remote `save`/`saveWithReport` are stepped
 *   jobs already, cancelled through `signal`;
 * - `extractJob` returns a job whose model results the caller provides between steps: run the
 *   model in the worker's own context, or extract on the main thread;
 * - `ensure` fetches a range document's bytes: the worker does that for every call;
 * - `extractPages`, `exportRanges` and `split` return new documents, which stay in the worker: save one
 *   on the main thread, or open the saved bytes with the client;
 * - `free` and `Symbol.dispose`: the remote handle's `close()` and `Symbol.asyncDispose`.
 *
 * @since 0.2.0
 */
export type ExcludedDocumentMembers =
  | "transact"
  | "saveJob"
  | "extractJob"
  | "extractPages"
  | "exportRanges"
  | "split"
  | "ensure"
  | "free"
  | typeof Symbol.dispose;

/**
 * PageRef members with no remote twin: `free` and `Symbol.dispose`, since the worker frees a page
 * with its document.
 *
 * @since 0.2.0
 */
export type ExcludedPageMembers = "free" | typeof Symbol.dispose;

export type RemotableDocumentMember = Exclude<keyof Document, ExcludedDocumentMembers>;
export type RemotablePageMember = Exclude<keyof PageRef, ExcludedPageMembers>;

export const DOCUMENT_METHODS = {
  addSource: 2,
  adoptCopy: 2,
  apply: 1,
  applyLog: 1,
  canExtractText: 0,
  cropPages: 2,
  caretAt: 2,
  deletePages: 2,
  destinationAt: 2,
  duplicatePages: 2,
  extract: 1,
  extractImages: 1,
  hasUnsavedChanges: 0,
  info: 0,
  initialView: 0,
  insertBlankPages: 4,
  insertPagesFrom: 4,
  isEncrypted: 0,
  isTagged: 0,
  linkOnArea: 3,
  linkOnText: 3,
  linksCsv: 0,
  linksJson: 0,
  markText: 3,
  mergeDocuments: 3,
  metadata: 0,
  movePages: 2,
  namedDestination: 1,
  namedDestinations: 0,
  noteAt: 4,
  outline: 0,
  outlineCsv: 0,
  outlineJson: 0,
  page: 1,
  pageAnnotations: 1,
  pageById: 1,
  pageCount: 0,
  pageInfo: 1,
  pageLinks: 1,
  pages: 0,
  plan: 1,
  properties: 0,
  rebase: 1,
  redo: 0,
  reflow: 0,
  removeMetadata: 2,
  removeXmpProperty: 3,
  replacePages: 4,
  replaceText: 4,
  replaceXmpPacket: 1,
  reversePages: 1,
  rotatePages: 3,
  save: 1,
  saveCopy: 1,
  saveWithReport: 1,
  setInitialView: 1,
  setMetadata: 2,
  setPageBoxes: 2,
  setPageLabels: 1,
  setXmpProperty: 5,
  structureTree: 0,
  threads: 0,
  undo: 0,
  warnings: 0,
  wouldViolate: 1,
  xmpPacket: 0,
} as const satisfies Record<RemotableDocumentMember, number>;

export const PAGE_METHODS = {
  id: 0,
  index: 0,
  info: 0,
  text: 1,
} as const satisfies Record<RemotablePageMember, number>;
