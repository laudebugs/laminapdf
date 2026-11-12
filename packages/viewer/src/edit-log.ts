// What the page keeps of an edit session so a restarted worker can take it back (ViewerClient's
// restart): the ops of every step still in the document, with their labels and times, and the
// history the worker last answered. The worker holds the session itself; this copy only ever
// replays it into a new worker, which applies the steps again and undoes the undone ones. After a
// save, the new worker opens the saved file, so only the steps since the save are replayed.

import type { Op } from "@laminapdf/sdk";
import { type EditHistory, type EditResult, EMPTY_HISTORY, type ReplayedStep } from "./protocol.js";

/** One step's ops, under the ID its worker gave it. */
interface LoggedStep extends ReplayedStep {
  readonly id: number;
}

/** The steps of the open document's edit session and how many of them are undone. */
export class EditLog {
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
  take(result: EditResult, edit?: { readonly ops: readonly Op[]; readonly label: string }): void {
    if (result.reverted?.some((step) => step.outcome !== "applied")) this.#replayable = false;
    const { steps, dropped } = result.history;
    const newest = steps.at(-1);
    if (edit !== undefined && newest !== undefined && !this.#steps.some((s) => s.id === newest.id)) {
      this.#steps.push({ id: newest.id, ops: edit.ops, label: newest.label, at: newest.at });
    }
    // The depth limit dropped the oldest steps of the last answer, as many as `dropped` grew by:
    // still part of the document, so a replay needs those this log holds (since a save, the older
    // ones are in the saved file). A step gone otherwise was cleared by a new edit.
    const newlyDropped = Math.max(0, dropped - this.#history.dropped);
    const droppedIds = new Set(this.#history.steps.slice(0, newlyDropped).map((s) => s.id));
    for (const { id, ops, label, at } of this.#steps) if (droppedIds.has(id)) this.#dropped.push({ ops, label, at });
    const kept = new Set(steps.map((s) => s.id));
    this.#steps = this.#steps.filter((s) => kept.has(s.id));
    // Undone past the save: the steps it would take apply to a file the new worker never opens.
    if (this.#savedAt !== undefined && dropped + result.history.position < this.#savedAt) this.#replayable = false;
    this.#history = result.history;
  }

  /**
   * What a new worker replays: every step, then how many of the newest to undo; undefined when
   * the replay couldn't reproduce the document (a partial or skipped undo or redo).
   */
  replay(): { readonly steps: readonly ReplayedStep[]; readonly undone: number } | undefined {
    if (!this.#replayable) return undefined;
    const { steps, position, dropped } = this.#history;
    const logged = [...this.#dropped, ...this.#steps.map(({ ops, label, at }) => ({ ops, label, at }))];
    // Since a save, the steps done are those past its position.
    const undone =
      this.#savedAt === undefined ? steps.length - position : logged.length - (dropped + position - this.#savedAt);
    return { steps: logged, undone };
  }

  /** Takes a new worker's answer to the replay: its steps have new IDs, in the same order. */
  replayed(result: EditResult): void {
    const ids = result.history.steps.map((s) => s.id);
    this.#steps = this.#steps.map((step, i) => ({ ...step, id: ids[i] ?? step.id }));
    this.#history = result.history;
  }
}
