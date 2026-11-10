// Edits on screen (E1 plan §6). PDFium draws the pages from the bytes it opened, so an edit to
// Lamina PDF's model reaches the screen as bytes: after a transaction that touched pages, the
// model saves incrementally into memory (only the appended revision; the model rebases onto the
// file with it) and the backend opens the file with every revision appended so far. The new render
// document replaces the old one only once it is open, so a failure leaves the pages drawn as they
// were. The worker runs one call at a time, so no render runs while the documents are swapped: a
// render answered before this call drew the old file, one answered after it draws the new one.

import type { RenderSource } from "@laminapdf/render";
import type { Op, PageInfo } from "@laminapdf/sdk";
import type { EditResult } from "../protocol.js";
import { current, currentBackend, filling, type Open, syncTextCopy } from "./document.js";
import { forgetSearch } from "./find.js";
import { forgetLinks } from "./navigation.js";
import { forgetText } from "./text.js";

/**
 * Applies `ops` to document `id`'s model as one transaction; when it touched pages, writes the
 * edit and draws those pages from it from now on.
 *
 * @throws the op's error (nothing changed); a save or reopen error, after the transaction is
 *   undone, so the model again matches what is drawn.
 */
export async function applyEdit(ops: readonly Op[], id: number, signal: AbortSignal): Promise<EditResult> {
  const opened = current(id);
  const { model } = opened;
  const edited = await model.ensure(() => model.apply(ops), { signal });
  if (edited.touchedPages.length > 0) {
    try {
      await redraw(opened);
    } catch (e) {
      // The edit never reached the screen: take it back out of the model, so the model matches
      // what is drawn (the undone transaction becomes a redo entry, for the edit session to show).
      await model.ensure(() => model.undo()).catch(() => undefined);
      throw e;
    }
  }
  opened.edits.push(ops);
  syncTextCopy(opened);
  return { pages: await touched(opened, edited.touchedPages) };
}

/** The pages with these IDs, as they are now, in index order; what the worker kept of them is forgotten. */
async function touched(opened: Open, ids: readonly string[]): Promise<PageInfo[]> {
  if (ids.length === 0) return [];
  const wanted = new Set(ids);
  const { model } = opened;
  const pages = (await model.ensure(() => model.pages)).filter((page) => wanted.has(page.id));
  const indices = pages.map((page) => page.index);
  forgetText(indices);
  forgetSearch(indices);
  forgetLinks(indices);
  return pages;
}

/**
 * Saves the model's changes into memory and swaps in a render document opened from the result.
 * The old one keeps drawing until the new one is open, and is closed after.
 */
async function redraw(opened: Open): Promise<void> {
  const { model } = opened;
  // Only the appended revision: the model rebases onto the old file with it, copying nothing.
  const { bytes } = await model.ensure(() => model.saveWithReport({ mode: "incremental", output: "appendedOnly" }));
  // Kept even if the reopen below fails: the model's file now ends with this revision, and the
  // next save appends to it.
  opened.appended = concat(opened.appended, bytes);
  const source = appendedSource(opened.source, opened.appended);
  const options = opened.renderLimits === undefined ? {} : { limits: opened.renderLimits };
  // Not the caller's signal: the model is already saved, so the reopen finishes either way.
  const next = await filling(opened.remote, () => currentBackend().open(source, options), new AbortController().signal);
  const old = opened.render;
  opened.render = next;
  old.close();
}

function concat(a: Uint8Array, b: Uint8Array): Uint8Array {
  const out = new Uint8Array(a.length + b.length);
  out.set(a);
  out.set(b, a.length);
  return out;
}

/**
 * `base` with `appended` after it: what the render document reads after an edit. A remote base
 * still answers `undefined` for bytes not fetched yet, which `filling` fetches.
 */
function appendedSource(base: RenderSource, appended: Uint8Array): RenderSource {
  const size = base.size;
  return {
    size: size + appended.length,
    readSync(offset, length) {
      const end = Math.min(size + appended.length, offset + length);
      if (end <= size) return base.readSync(offset, length);
      if (offset >= size) return appended.subarray(offset - size, end - size);
      // Across the join: the base's end, then the start of the appended revisions.
      const head = base.readSync(offset, size - offset);
      if (head === undefined) return undefined;
      const out = new Uint8Array(end - offset);
      out.set(head);
      out.set(appended.subarray(0, end - size), head.length);
      return out;
    },
  };
}
