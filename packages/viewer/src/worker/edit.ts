// Edits on screen (E1 plan §6). PDFium draws the pages from the bytes it opened, so an edit to
// Lamina PDF's model reaches the screen as bytes: after a transaction that touched pages, the
// model saves incrementally into memory (only the appended revision; the model rebases onto the
// file with it) and the backend opens the file with every revision appended so far. The new render
// document replaces the old one only once it is open, so a failure leaves the pages drawn as they
// were. The worker runs one call at a time, so no render runs while the documents are swapped: a
// render answered before this call drew the old file, one answered after it draws the new one.

import type { RenderSource } from "@laminapdf/render";
import type { Document, Op, PageInfo } from "@laminapdf/sdk";
import type { EditResult } from "../protocol.js";
import { current, currentBackend, type EditState, filling, followEdit, type Open } from "./document.js";
import { forgetSearch } from "./find.js";
import { forgetLinks } from "./navigation.js";
import { forgetText } from "./text.js";

/** Why an edit that failed left the model and the pages drawn apart: the document must be opened again. */
const DIVERGED = "an edit failed and couldn't be undone: the document and the pages drawn may differ; open it again";

/**
 * Applies `ops` to document `id`'s model as one transaction; when it touched pages, writes the
 * edit and draws those pages from it from now on.
 *
 * @throws the op's error (nothing changed); a save or reopen error, after the transaction is
 *   undone, so the model again matches what is drawn; an error saying the document must be opened
 *   again when that undo failed too, and for every edit after it.
 */
export async function applyEdit(ops: readonly Op[], id: number, signal: AbortSignal): Promise<EditResult> {
  const opened = current(id);
  const { model, edit } = opened;
  if (edit.diverged) throw new Error(DIVERGED);
  const count = await model.ensure(() => model.pageCount, { signal });
  const edited = await model.ensure(() => model.apply(ops), { signal });
  if (edited.touchedPages.length > 0) {
    try {
      await samePages(model, count);
      await redraw(opened);
    } catch (e) {
      // The edit never reached the screen: take it back out of the model, so the model matches
      // what is drawn (the undone transaction becomes a redo entry, for the edit session to show).
      await undo(model, edit, e);
      throw e;
    }
  }
  followEdit(opened, ops);
  return { pages: await touched(opened, edited.touchedPages) };
}

/**
 * Refuses an edit that added, removed or moved pages: the view finds pages by index, and
 * `EditResult` names pages by their indices after the edit.
 */
async function samePages(model: Document, before: number): Promise<void> {
  const after = await model.ensure(() => model.pageCount);
  if (after !== before)
    throw new Error(`an edit that changes the page count (${before} → ${after}) can't be shown yet`);
}

/** Undoes the transaction just applied; when that fails too, marks the document diverged. */
async function undo(model: Document, edit: EditState, failure: unknown): Promise<void> {
  try {
    await model.ensure(() => model.undo());
  } catch (e) {
    edit.diverged = true;
    throw new Error(DIVERGED, { cause: [failure, e] });
  }
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
  const { model, edit } = opened;
  // Only the appended revision: the model rebases onto the old file with it, copying nothing.
  const { bytes } = await model.ensure(() => model.saveWithReport({ mode: "incremental", output: "appendedOnly" }));
  // Kept even if the reopen below fails: the model's file now ends with this revision, and the
  // next save appends to it. Each edit adds one revision, and one more section to the chain of
  // cross-reference sections PDFium reads on open; compacting them is the edit session's to decide.
  edit.appended.push(bytes);
  const source = joined([edit.source, ...edit.appended.map(bytesSource)]);
  const options = edit.renderLimits === undefined ? {} : { limits: edit.renderLimits };
  // Not the caller's signal: the model is already saved, so the reopen finishes either way.
  const next = await filling(opened.remote, () => currentBackend().open(source, options), new AbortController().signal);
  const old = opened.render;
  opened.render = next;
  old.close();
}

function bytesSource(bytes: Uint8Array): RenderSource {
  return { size: bytes.length, readSync: (offset, length) => bytes.subarray(offset, offset + length) };
}

/**
 * `parts` one after another, read in place: the file and the revisions appended to it, never
 * copied into one buffer. A part that answers `undefined` (a remote file's bytes not fetched yet)
 * makes the read answer `undefined`, which `filling` fetches.
 */
function joined(parts: readonly RenderSource[]): RenderSource {
  const starts: number[] = [];
  let size = 0;
  for (const part of parts) {
    starts.push(size);
    size += part.size;
  }
  return {
    size,
    readSync(offset, length) {
      const end = Math.min(size, offset + length);
      const pieces: Uint8Array[] = [];
      for (let i = 0; i < parts.length; i++) {
        const start = starts[i] as number;
        const part = parts[i] as RenderSource;
        const from = Math.max(offset, start);
        const to = Math.min(end, start + part.size);
        if (from >= to) continue;
        const piece = part.readSync(from - start, to - from);
        if (piece === undefined) return undefined;
        pieces.push(piece);
      }
      return pieces.length === 1 ? pieces[0] : concat(pieces);
    },
  };
}

function concat(pieces: readonly Uint8Array[]): Uint8Array {
  const out = new Uint8Array(pieces.reduce((sum, piece) => sum + piece.length, 0));
  let at = 0;
  for (const piece of pieces) {
    out.set(piece, at);
    at += piece.length;
  }
  return out;
}
