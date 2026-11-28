// What the page keeps of an edit session so a restarted worker can take it back (ViewerClient's
// restart): the ops of every step still in the document, with their labels and times, and the
// history the worker last answered. The worker holds the session itself; this copy only ever
// replays it into a new worker, which applies the steps again and undoes the undone ones. After a
// save, the new worker opens the saved file, so only the steps since the save are replayed.
// Steps that copy pages from another file need it again: the log keeps the bytes of each source
// document a logged step names, and a new worker adds them before the steps. The page never keeps
// a password, so a step that names an encrypted source can't be replayed.

import type { Op } from "@laminapdf/sdk";
import {
  type ContentsInsert,
  type EditHistory,
  type EditResult,
  EMPTY_HISTORY,
  type ImportedFile,
  type ReplayedStep,
} from "./protocol.js";

/** One step's ops, under the ID its worker gave it. */
interface LoggedStep extends ReplayedStep {
  readonly id: number;
}

/** A source document's bytes, or that it was encrypted: its password died with the worker. */
type LoggedSource = { readonly bytes: Uint8Array } | { readonly encrypted: true };

/** The source documents `ops` copy pages from, by ID. */
function sourcesOf(ops: readonly Op[]): string[] {
  const ids: string[] = [];
  for (const op of ops) {
    const args = op.args as { readonly source?: unknown; readonly sources?: readonly { readonly source?: unknown }[] };
    if (typeof args.source === "string") ids.push(args.source);
    for (const entry of args.sources ?? []) if (typeof entry.source === "string") ids.push(entry.source);
  }
  return ids;
}

/** The blob IDs `ops` name: the files `attachFile` attaches. */
function blobsOf(ops: readonly Op[]): string[] {
  return ops.flatMap((op) => {
    const file = op.op === "attachFile" ? (op.args as { readonly file?: unknown }).file : undefined;
    return typeof file === "string" ? [file] : [];
  });
}

/** The steps of the open document's edit session and how many of them are undone. */
export class EditLog {
  /** The source documents added to the document, by ID, while a logged step may name them. */
  #sources = new Map<string, LoggedSource>();
  /** The bytes added to the document's session, by ID, while a logged step may name them. */
  #blobs = new Map<string, Uint8Array>();
  /** The steps the depth limit dropped from the history, oldest first: still part of the document. */
  #dropped: ReplayedStep[] = [];
  /** The history's steps, oldest first. */
  #steps: LoggedStep[] = [];
  #history: EditHistory = EMPTY_HISTORY;
  /**
   * Whether applying the steps again reproduces the document: not once an undo or redo restored
   * only part of a step, or nothing (a later change kept its value), since the replay can't
   * recreate the change that made it so.
   */
  #replayable = true;
  /**
   * The absolute position of the last save this log saw (`saved`), whose file a new worker opens:
   * the steps before it are in that file. Undefined while the file is the one opened.
   */
  #savedAt: number | undefined;

  /** The history the worker last answered. */
  get history(): EditHistory {
    return this.#history;
  }

  /** Whether there is anything a new worker would lose. */
  get empty(): boolean {
    return this.#dropped.length === 0 && this.#steps.length === 0;
  }

  /** Forgets everything: a document opened, closed, or its edits lost. */
  reset(): void {
    this.#dropped = [];
    this.#steps = [];
    this.#history = EMPTY_HISTORY;
    this.#replayable = true;
    this.#savedAt = undefined;
    this.#sources.clear();
    this.#blobs.clear();
  }

  /** The document took a source document: its bytes, kept while a step names it, or that it was encrypted. */
  addedSource(id: string, bytes: Uint8Array, encrypted: boolean): void {
    this.#sources.set(id, encrypted ? { encrypted: true } : { bytes });
  }

  /** The document's session took `bytes` as blob `id`, kept while a step names it. */
  addedBlob(id: string, bytes: Uint8Array): void {
    this.#blobs.set(id, bytes);
  }

  /**
   * A save wrote the document as `history` has it, and a new worker opens that file: every step
   * so far is in it, so the log starts over from here.
   */
  saved(history: EditHistory): void {
    this.reset();
    this.#history = history;
    this.#savedAt = history.dropped + history.position;
  }

  /**
   * Takes the worker's answer to a call: an edit's `ops` under the step it made, the oldest steps
   * the depth limit dropped kept apart (they are still part of the document, so a replay needs
   * them), and the steps the answer no longer lists forgotten.
   */
  take(
    result: EditResult,
    edit?: {
      readonly ops: readonly Op[];
      readonly label: string;
      readonly contents?: ContentsInsert;
      readonly imported?: ImportedFile;
    },
  ): void {
    if (result.reverted?.some((step) => step.outcome !== "applied")) this.#replayable = false;
    const { steps, dropped } = result.history;
    const newest = steps.at(-1);
    if (edit !== undefined && newest !== undefined && !this.#steps.some((s) => s.id === newest.id)) {
      // A contents page's step replays its laid-out page, which the worker's answer carries.
      const contents = edit.contents ?? result.contents;
      // An import's step replays its file, which the worker's answer carries.
      const imported = edit.imported ?? result.imported;
      this.#steps.push({
        id: newest.id,
        ops: edit.ops,
        ...(contents === undefined ? {} : { contents }),
        ...(imported === undefined ? {} : { imported }),
        label: newest.label,
        at: newest.at,
      });
    }
    // The depth limit dropped the oldest steps of the last answer, as many as `dropped` grew by:
    // still part of the document, so a replay needs those this log holds (since a save, the older
    // ones are in the saved file). A step gone otherwise was cleared by a new edit.
    const newlyDropped = Math.max(0, dropped - this.#history.dropped);
    const droppedIds = new Set(this.#history.steps.slice(0, newlyDropped).map((s) => s.id));
    for (const { id, ...step } of this.#steps) if (droppedIds.has(id)) this.#dropped.push(step);
    const kept = new Set(steps.map((s) => s.id));
    this.#steps = this.#steps.filter((s) => kept.has(s.id));
    // Undone past the save: the steps it would take apply to a file the new worker never opens.
    if (this.#savedAt !== undefined && dropped + result.history.position < this.#savedAt) this.#replayable = false;
    this.#history = result.history;
    // A source no step names any more is never replayed: its bytes go. Only an edit clears
    // steps for good, and only after an edit is a source just added sure to be named already.
    if (edit !== undefined) this.#forgetUnnamed();
  }

  /** Forgets the sources and blobs no logged step names: a replay never needs them. */
  #forgetUnnamed(): void {
    const named = new Set(this.#logged().flatMap((step) => sourcesOf(step.ops)));
    for (const id of this.#sources.keys()) if (!named.has(id)) this.#sources.delete(id);
    const blobs = new Set(this.#logged().flatMap((step) => blobsOf(step.ops)));
    for (const id of this.#blobs.keys()) if (!blobs.has(id)) this.#blobs.delete(id);
  }

  /** Every step a replay applies, oldest first. */
  #logged(): ReplayedStep[] {
    return [...this.#dropped, ...this.#steps.map(({ id: _, ...step }) => step)];
  }

  /**
   * What a new worker replays: the source documents and the blobs the steps name, every step, then how many of
   * the newest to undo; undefined when the replay couldn't reproduce the document (a partial or
   * skipped undo or redo, or a step that names an encrypted source).
   */
  replay():
    | {
        readonly steps: readonly ReplayedStep[];
        readonly undone: number;
        readonly sources: readonly Uint8Array[];
        readonly blobs: readonly Uint8Array[];
      }
    | undefined {
    if (!this.#replayable) return undefined;
    const { steps, position, dropped } = this.#history;
    const logged = this.#logged();
    const sources: Uint8Array[] = [];
    for (const id of new Set(logged.flatMap((step) => sourcesOf(step.ops)))) {
      const source = this.#sources.get(id);
      // An encrypted source, or one this log never saw added: the steps can't be applied again.
      if (source === undefined || !("bytes" in source)) return undefined;
      sources.push(source.bytes);
    }
    const blobs: Uint8Array[] = [];
    for (const id of new Set(logged.flatMap((step) => blobsOf(step.ops)))) {
      const blob = this.#blobs.get(id);
      // Bytes this log never saw added: the steps can't be applied again.
      if (blob === undefined) return undefined;
      blobs.push(blob);
    }
    // Since a save, the steps done are those past its position.
    const undone =
      this.#savedAt === undefined ? steps.length - position : logged.length - (dropped + position - this.#savedAt);
    return { steps: logged, undone, sources, blobs };
  }

  /** Takes a new worker's answer to the replay: its steps have new IDs, in the same order. */
  replayed(result: EditResult): void {
    const ids = result.history.steps.map((s) => s.id);
    this.#steps = this.#steps.map((step, i) => ({ ...step, id: ids[i] ?? step.id }));
    this.#history = result.history;
  }
}
