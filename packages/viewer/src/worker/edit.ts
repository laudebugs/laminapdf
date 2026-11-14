// Edits on screen (E1 plan §6) and saving them (§7). PDFium draws the pages from the bytes it
// opened, so an edit to Lamina PDF's model reaches the screen as bytes: after a transaction that
// touched pages, the model writes a copy of every edit since the last save, as one revision
// appended to the saved file (`saveCopy`: the model isn't rebased, so its plan still sees every
// change), and the backend opens the file with it. The new render document replaces the old one
// only once it is open, so a failure leaves the pages drawn as they were. The worker runs one call
// at a time, so no render runs while the documents are swapped: a render answered before this call
// drew the old file, one answered after it draws the new one.
//
// A save is a copy too (`copyEdits`), which the page writes where the file lives; only once that
// worked does the model adopt it as its saved file (`adoptEdits`), so a failed write leaves every
// edit unsaved.

import type { RenderSource } from "@laminapdf/render";
import type {
  Document,
  DocumentMetadata,
  Op,
  PageInfo,
  PlannedSave,
  Reverted,
  SaveOptions,
  XmpText,
} from "@laminapdf/sdk";
import type { EditHistory, EditResult, ReplayedStep, RevertedStep, SavedCopy } from "../protocol.js";
import { current, currentBackend, type EditState, filling, followEdit, type Open } from "./document.js";
import { forgetSearch } from "./find.js";
import { forgetLinks } from "./navigation.js";
import {
  clearHistory,
  clearRedo,
  historyOf,
  markSaved,
  orderOf,
  type PageOrder,
  placed,
  record,
  removeStep,
  restore,
  type Session,
  snapshot,
  stepBack,
  walkTo,
} from "./session.js";
import { forgetText } from "./text.js";

/** Why an edit that failed left the model and the pages drawn apart: the document must be opened again. */
const DIVERGED = "an edit failed and couldn't be undone: the document and the pages drawn may differ; open it again";

/** Why an encrypted document takes no edit: its edits can't be written into memory to be drawn. */
const ENCRYPTED = "an encrypted document can't be edited yet: its edits can't be saved (encrypted-base)";

/** The edit session's history of document `id`. */
export function editHistory(id: number): EditHistory {
  const { edit } = current(id);
  return historyOf(edit.session, edit.diverged, edit.order);
}

/** Throws why document `opened` can't take an edit, undo or redo now, if it can't. */
function editable(opened: Open): void {
  if (opened.edit.diverged) throw new Error(DIVERGED);
  if (opened.model.isEncrypted) throw new Error(ENCRYPTED);
}

/**
 * Applies `ops` to document `id`'s model as one transaction; when it touched pages, writes the
 * edit and draws those pages from it from now on. The edit session records it as a step named
 * `label`; an edit that changed nothing records none.
 *
 * @throws the op's error (nothing changed); a save or reopen error, after the transaction is
 *   undone, so the model again matches what is drawn; an error saying the document must be opened
 *   again when that undo failed too, and for every edit after it; an encrypted document's refusal.
 */
export function applyEdit(
  ops: readonly Op[],
  id: number,
  signal: AbortSignal,
  label = describe(ops),
): Promise<EditResult> {
  return serial(current(id), () => apply(ops, id, signal, label));
}

async function apply(ops: readonly Op[], id: number, signal: AbortSignal, label: string): Promise<EditResult> {
  const opened = current(id);
  editable(opened);
  const { model, edit } = opened;
  const known = await knownOrder(opened, signal);
  const edited = await model.ensure(() => model.apply(ops), { signal });
  if (!edited.changed) return { pages: [], history: historyOf(edit.session, false, edit.order) };
  edit.copy = undefined;
  // The transaction cleared the model's redo stack, whatever happens next.
  clearRedo(edit.session);
  // Pages that only moved draw as they did: nothing to redraw but the new order.
  const shown = onlyMoves(ops) ? [] : edited.touchedPages;
  let after: PagesNow;
  try {
    after = await pagesAfter(model, known, edited.pageOrder, shown.length > 0);
    if (shown.length > 0 || after.moved) await redraw(opened);
  } catch (e) {
    // The edit never reached the screen: take it back out of the model, so the model matches
    // what is drawn. The undone transaction is left on the model's redo stack, but the session
    // has no step for it, so nothing redoes it, and the next edit clears it.
    await revert(model, edit, "undo", e);
    throw e;
  }
  followEdit(opened, ops);
  const pages = touched(opened, after, shown);
  const step = record(edit.session, label, pages.length > 0 ? pages : listed(after, edited.touchedPages), Date.now());
  if (after.moved) edit.reorders.set(step.id, onlyMoves(ops));
  return {
    pages,
    history: historyOf(edit.session, false, edit.order),
    ...(after.moved && after.list !== undefined ? { order: after.list } : {}),
    createdOutlineItems: edited.createdOutlineItems,
    createdAnnotations: edited.createdAnnotations,
    outlineRepaired: edited.outlineRepaired,
  };
}

/**
 * Undoes or redoes steps of document `id`'s edit session until `position` of them are done, then
 * draws the pages they touched, once. A move that fails half way is taken back, so the document,
 * the history and the pages drawn stay together.
 */
export function goToStep(position: number, id: number, signal: AbortSignal): Promise<EditResult> {
  return serial(current(id), () => moveTo(() => position, id, signal));
}

/**
 * Undoes (`delta` -1) or redoes (+1) one step of document `id`'s edit session, reading where it
 * stands only once the calls before it are done.
 */
export function stepEdit(delta: -1 | 1, id: number, signal: AbortSignal): Promise<EditResult> {
  const opened = current(id);
  return serial(opened, () => moveTo(() => opened.edit.session.position + delta, id, signal));
}

/**
 * Runs `call` after document `opened`'s edit calls before it have settled: the worker serves
 * calls as they come, and an edit's awaits would otherwise let another start against the history
 * it is changing.
 */
function serial<T>(opened: Open, call: () => Promise<T>): Promise<T> {
  const run = opened.edit.queue.then(call);
  opened.edit.queue = run.catch(() => undefined);
  return run;
}

async function moveTo(position: () => number, id: number, signal: AbortSignal): Promise<EditResult> {
  const opened = current(id);
  editable(opened);
  const { model, edit } = opened;
  const { session } = edit;
  const kept = snapshot(session);
  edit.copy = undefined;
  const done: RevertedStep[] = [];
  const ids = new Set<string>();
  const known = await knownOrder(opened, signal);
  let after: PagesNow;
  try {
    await walkTo(
      session,
      position(),
      async (way) => {
        const reverted = await model.ensure(() => (way === "undo" ? model.undo() : model.redo()), { signal });
        // The model has no entry where the session has a step: they disagree, which only a bug does.
        if (reverted === undefined) throw new Error(`the document has nothing to ${way}`);
        for (const page of reverted.touchedPages) ids.add(page);
        followEdit(opened, way);
        return reverted.outcome;
      },
      done,
    );
    // Undo and redo report no page order: the steps that changed the page list say to read it.
    const reordered = done.some((move) => edit.reorders.has(move.step.id));
    const moves = reordered && done.every((move) => edit.reorders.get(move.step.id) === true);
    if (moves) ids.clear();
    after = await pagesAfter(model, known, reordered ? "read" : undefined, ids.size > 0);
    if (ids.size > 0 || after.moved) await redraw(opened);
  } catch (e) {
    await takeBack(opened, kept, done, e);
    throw e;
  }
  const pages = touched(opened, after, [...ids]);
  return {
    pages,
    history: historyOf(session, false, edit.order),
    reverted: done.map((move) => ({ ...move, step: placed(move.step, edit.order) })),
    ...(after.moved && after.list !== undefined ? { order: after.list } : {}),
  };
}

/**
 * Takes back the undos and redos in `done`, newest first, and puts the history back as `kept`
 * was, less the steps the model dropped (a skipped undo or redo leaves it no entry).
 */
async function takeBack(opened: Open, kept: Session, done: readonly RevertedStep[], failure: unknown): Promise<void> {
  const { model, edit } = opened;
  for (const step of [...done].reverse()) {
    if (step.outcome === "skipped") continue;
    await revert(model, edit, step.way === "undo" ? "redo" : "undo", failure);
    followEdit(opened, step.way === "undo" ? "redo" : "undo");
  }
  const { session } = edit;
  restore(session, kept);
  for (const step of done) {
    if (step.outcome !== "skipped") continue;
    const index = session.steps.findIndex((s) => s.id === step.step.id);
    if (index >= 0) removeStep(session, index, step.way);
  }
}

/**
 * Applies `steps` to document `id` again, one transaction each, then undoes the newest `undone`
 * of them, and draws once: a restarted worker's document takes back the edits the old one had.
 * The history keeps their labels and times.
 *
 * @throws an op's error, or a save or reopen error: the document is then marked diverged, since
 *   the edits applied so far can't all be taken back (the depth limit may have dropped some).
 */
export function restoreEdits(
  steps: readonly ReplayedStep[],
  undone: number,
  id: number,
  signal: AbortSignal,
): Promise<EditResult> {
  return serial(current(id), () => replay(steps, undone, id, signal));
}

async function replay(
  steps: readonly ReplayedStep[],
  undone: number,
  id: number,
  signal: AbortSignal,
): Promise<EditResult> {
  const opened = current(id);
  editable(opened);
  const { model, edit } = opened;
  const ids = new Set<string>();
  edit.copy = undefined;
  let after: PagesNow;
  try {
    const known = await knownOrder(opened, signal);
    let order = known;
    for (const step of steps) order = await replayStep(opened, step, order, ids, signal);
    for (let i = 0; i < undone && edit.session.position > 0; i++) {
      const reverted = await model.ensure(() => model.undo(), { signal });
      if (reverted === undefined) break;
      stepBack(edit.session, reverted.outcome);
      followEdit(opened, "undo");
    }
    after = await pagesAfter(model, known, "read", true);
    if (ids.size > 0 || after.moved) await redraw(opened);
  } catch (e) {
    edit.diverged = true;
    throw new Error(DIVERGED, { cause: e });
  }
  // The file reopened as it was: the saved document is the one opened.
  edit.session.saved = 0;
  return {
    pages: touched(opened, after, [...ids]),
    history: historyOf(edit.session, false, edit.order),
    ...(after.moved && after.list !== undefined ? { order: after.list } : {}),
  };
}

/**
 * Applies one replayed step and records it, adding the pages to draw again to `ids`; returns the
 * page order after it (`order` before it).
 */
async function replayStep(
  opened: Open,
  step: ReplayedStep,
  order: PageOrder,
  ids: Set<string>,
  signal: AbortSignal,
): Promise<PageOrder> {
  const { model, edit } = opened;
  const edited = await model.ensure(() => model.apply(step.ops), { signal });
  if (!edited.changed) return order;
  const moves = onlyMoves(step.ops);
  if (!moves) for (const page of edited.touchedPages) ids.add(page);
  followEdit(opened, step.ops);
  const pages = await model.ensure(() => model.pages);
  const wanted = new Set(edited.touchedPages);
  const recorded = record(
    edit.session,
    step.label,
    pages.filter((page) => wanted.has(page.id)),
    step.at,
  );
  if (samePages(pages, order)) return order;
  edit.reorders.set(recorded.id, moves);
  return orderOf(pages.map((page) => page.id));
}

/** A label for an edit whose caller gave none: its ops' names. */
function describe(ops: readonly Op[]): string {
  return ops.map((op) => op.op).join(", ");
}

/** Ops that only move pages: the moved pages draw as they did, so nothing is drawn again. */
const MOVE_OPS: ReadonlySet<string> = new Set(["movePages", "reversePages"]);

/** Whether every op of `ops` only moves pages. */
function onlyMoves(ops: readonly Op[]): boolean {
  return ops.length > 0 && ops.every((op) => MOVE_OPS.has(op.op));
}

/** The pages after an edit, undo or redo: read when needed, and whether the page list changed. */
interface PagesNow {
  /** Every page now, when it was read: when the list changed, or pages are to be drawn again. */
  readonly list: readonly PageInfo[] | undefined;
  readonly moved: boolean;
}

/**
 * Where each page of document `opened` is: kept after each call that read the pages, and read once
 * the first time an edit needs it.
 */
async function knownOrder(opened: Open, signal: AbortSignal): Promise<PageOrder> {
  const { model, edit } = opened;
  if (edit.order === undefined) {
    const pages = await model.ensure(() => model.pages, { signal });
    edit.order = orderOf(pages.map((page) => page.id));
  }
  return edit.order;
}

/**
 * The pages after an edit, undo or redo, from what it says it did: `reported` is the page order an
 * edit reports (the façade's `Edited::page_order`), `"read"` for an undo or redo of a step that
 * changed the page list, and undefined when it changed none. The pages are read when the list may
 * have changed or `drawn` pages need their sizes. An edit must leave the pages in the order it
 * reports, and one that reports none must keep their count: the view follows the pages by what the
 * edit says it did, and would otherwise show some pages in another's place.
 */
async function pagesAfter(
  model: Document,
  known: PageOrder,
  reported: readonly string[] | "read" | undefined,
  drawn: boolean,
): Promise<PagesNow> {
  if (reported === undefined) {
    const count = await model.ensure(() => model.pageCount);
    if (count !== known.size) throw new Error("the edit changed the page list without reporting its order");
    return { list: drawn ? await model.ensure(() => model.pages) : undefined, moved: false };
  }
  const list = await model.ensure(() => model.pages);
  if (reported !== "read" && (list.length !== reported.length || list.some((page, i) => page.id !== reported[i]))) {
    throw new Error("the edit left the pages in another order than it reported");
  }
  return { list, moved: !samePages(list, known) };
}

/** Whether `pages` are the pages of `order`, in its order. */
function samePages(pages: readonly PageInfo[], order: PageOrder): boolean {
  return pages.length === order.size && pages.every((page, i) => order.get(page.id) === i);
}

/** The pages with these IDs, by ID and index, from the pages read after the edit (none when it read none). */
function listed(after: PagesNow, ids: readonly string[]): PageInfo[] {
  const wanted = new Set(ids);
  return (after.list ?? []).filter((page) => wanted.has(page.id));
}

/**
 * Undoes (or redoes) the transaction just applied (or undone); when that fails too, marks the
 * document diverged.
 */
async function revert(model: Document, edit: EditState, way: "undo" | "redo", failure: unknown): Promise<void> {
  let reverted: Reverted | undefined;
  try {
    reverted = await model.ensure(() => (way === "undo" ? model.undo() : model.redo()));
  } catch (e) {
    edit.diverged = true;
    throw new Error(DIVERGED, { cause: [failure, e] });
  }
  // A partial outcome still restored what it could; only nothing at all leaves them apart.
  if (reverted === undefined || reverted.outcome === "skipped") {
    edit.diverged = true;
    throw new Error(DIVERGED, { cause: failure });
  }
}

/**
 * The pages with these IDs among `after`, in index order. What the worker kept of them is
 * forgotten: of every page when the page list changed, since it is kept by index. The edit session
 * names its steps' pages from `after` from now on.
 */
function touched(opened: Open, after: PagesNow, ids: readonly string[]): PageInfo[] {
  const { edit } = opened;
  if (after.moved && after.list !== undefined) edit.order = orderOf(after.list.map((page) => page.id));
  const pages = listed(after, ids);
  const indices = after.moved ? undefined : pages.map((page) => page.index);
  forgetText(indices);
  forgetSearch(indices);
  forgetLinks(indices);
  return pages;
}

/**
 * Writes a copy of the model's changes since the last save and swaps in a render document opened
 * from the saved file with it. The old one keeps drawing until the new one is open, and is closed
 * after. The copy overrides signatures a change breaks (`invalidateSignatures`): the pages must
 * show the edit, and only a save asks before it breaks one.
 */
async function redraw(opened: Open): Promise<void> {
  const { model, edit } = opened;
  // Only the appended revision, unless the saved file must be rewritten (a repaired one).
  const { bytes, report } = await model.ensure(() =>
    model.saveCopy({ mode: "auto", output: "appendedOnly", invalidateSignatures: true }),
  );
  edit.drawn = { bytes, whole: report.mode === "fullRewrite" };
  await reopen(opened);
}

/** Swaps in a render document opened from the saved file and the copy drawn after it. */
async function reopen(opened: Open): Promise<void> {
  const { edit } = opened;
  const { drawn } = edit;
  const source =
    drawn === undefined
      ? edit.source
      : drawn.whole
        ? bytesSource(drawn.bytes)
        : joined([edit.source, bytesSource(drawn.bytes)]);
  const options = edit.renderLimits === undefined ? {} : { limits: edit.renderLimits };
  // Not the caller's signal: the model has moved on, so the reopen finishes either way.
  const next = await filling(opened.remote, () => currentBackend().open(source, options), new AbortController().signal);
  const old = opened.render;
  opened.render = next;
  old.close();
}

/** How saving document `id` with `options` would go: its constraint, signatures, mode and warnings. */
export function planSave(options: SaveOptions, id: number, signal: AbortSignal): Promise<PlannedSave> {
  const opened = current(id);
  return serial(opened, () => opened.model.ensure(() => opened.model.plan(options), { signal }));
}

/**
 * Document `id`'s metadata as its edits leave it (Info and XMP read together), for the properties
 * dialog's form. Read from the edited document, never the text copy, which takes the edits only once it has
 * replayed them.
 */
export function readMetadata(id: number, signal: AbortSignal): Promise<DocumentMetadata> {
  const opened = current(id);
  return serial(opened, () => opened.model.ensure(() => opened.model.metadata(), { signal }));
}

/** Document `id`'s XMP metadata as text, as its edits leave it, for the properties dialog's XMP tab. */
export function readXmpPacket(id: number, signal: AbortSignal): Promise<XmpText> {
  const opened = current(id);
  return serial(opened, () => opened.model.ensure(() => opened.model.xmpPacket(), { signal }));
}

/** The token of the next copy a save writes. */
let nextCopy = 1;

/**
 * Writes document `id` as a save with `options` would, for the page to write where the file
 * lives; the document stays unsaved until `adoptEdits` takes the copy. `whole` copies are the
 * file a save writes; Save a copy writes one too and never adopts it.
 *
 * @throws the save's refusals (`would-invalidate-signatures`, `requires-rewrite`, …); a diverged
 *   or encrypted document's.
 */
export function copyEdits(options: SaveOptions, id: number, signal: AbortSignal): Promise<SavedCopy> {
  const opened = current(id);
  return serial(opened, async () => {
    editable(opened);
    const { model, edit } = opened;
    const { bytes, report } = await model.ensure(() => model.saveCopy({ ...options, output: "whole" }), { signal });
    const token = nextCopy++;
    edit.copy = { token, bytes, historyCleared: report.historyCleared };
    return { token, bytes, report };
  });
}

/**
 * Makes the copy `token` names document `id`'s saved file, once the page has written it where
 * the file lives: the model rebases onto it, the pages are drawn from it, and the history marks
 * this point saved (or empties, when the save couldn't keep it).
 *
 * @throws when an edit, undo or redo since the copy made it stale: the copy written isn't the
 *   document any more, which stays unsaved.
 */
export function adoptEdits(token: number, id: number): Promise<EditHistory> {
  const opened = current(id);
  return serial(opened, async () => {
    const { model, edit } = opened;
    const { copy } = edit;
    if (copy?.token !== token) throw new Error("the document changed after this copy was written: it stays unsaved");
    edit.copy = undefined;
    await model.ensure(() => model.adoptCopy(copy.bytes, copy.historyCleared));
    edit.source = bytesSource(copy.bytes);
    edit.drawn = undefined;
    if (copy.historyCleared) clearHistory(edit.session);
    else markSaved(edit.session);
    await reopen(opened);
    return historyOf(edit.session, false, edit.order);
  });
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
