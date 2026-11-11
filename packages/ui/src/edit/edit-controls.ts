// What the editor chrome needs of `@laminapdf/viewer`'s edit session (`viewer.edits`), written as
// a shape rather than an import, as the viewer is (viewer-controls.ts): its state as a store, and
// the calls Undo, Redo and the History tab make. The words for a step and the save state are pure,
// so they are tested in Node.

import { countMessage, message } from "../messages.js";
import type { ExternalStore } from "../state/hooks.js";

/** One step of the history: a transaction, as the worker lists it. */
export interface EditStepSlice {
  readonly id: number;
  /** What it did: "Rotate page 5". */
  readonly label: string;
  /** The pages it touched, 0-based. */
  readonly pages: readonly number[];
  /** When it was made, in ms since the epoch. */
  readonly at: number;
  /** Its last undo or redo restored only part of it. */
  readonly partial?: true;
}

/** An undo or redo of one step, and how it went. */
export interface RevertedSlice {
  readonly step: EditStepSlice;
  readonly way: "undo" | "redo";
  readonly outcome: "applied" | "partial" | "skipped";
}

/** Why the document can't be edited now. */
export type EditBlockReason = "no-document" | "encrypted" | "diverged";

/** The part of the edit session's state the chrome shows. */
export interface EditSlice {
  readonly history: {
    /** The steps kept, oldest first; the first `position` are done. */
    readonly steps: readonly EditStepSlice[];
    readonly position: number;
    /** How many of the oldest steps the depth limit dropped. */
    readonly dropped: number;
    /** Transactions between the document now and the saved one (0: nothing to save). */
    readonly unsaved: number;
  };
  /** A call is running. */
  readonly busy: boolean;
  readonly blocked: EditBlockReason | undefined;
  /** The last undo or redo that left part of a step alone, until the next edit. */
  readonly notice: RevertedSlice | undefined;
}

/** The edit session as the chrome drives it: `viewer.edits` fits. */
export interface EditControls extends ExternalStore<EditSlice> {
  undo(): Promise<unknown>;
  redo(): Promise<unknown>;
  /** Undoes or redoes until `position` steps are done. */
  goTo(position: number): Promise<unknown>;
}

/** The step Undo would take back, if any. */
export function stepToUndo(slice: EditSlice): EditStepSlice | undefined {
  return slice.history.steps[slice.history.position - 1];
}

/** The step Redo would make again, if any. */
export function stepToRedo(slice: EditSlice): EditStepSlice | undefined {
  return slice.history.steps[slice.history.position];
}

/** Whether there is an edit session to show: some edit was made since the document opened. */
export function sessionStarted(slice: EditSlice): boolean {
  return slice.history.steps.length > 0 || slice.history.dropped > 0;
}

/** A step's label inside a sentence: "Undo rotate page 5". */
export function inSentence(label: string): string {
  // Only the first letter of an ordinary word: a label starting with a name ("PDF/A") keeps it.
  return /^[A-Z][a-z]/.test(label) ? label[0]?.toLowerCase() + label.slice(1) : label;
}

/** What the top bar says of the changes: "No unsaved changes", "1 unsaved change", "4 unsaved changes". */
export function saveStateText(slice: EditSlice): string {
  return countMessage(slice.history.unsaved, {
    zero: "edit.saveState.clean",
    one: "edit.saveState.one",
    many: "edit.saveState.many",
  });
}

/** Why an edit command can't run: the session's block, else `undefined`. */
export function blockedReason(reason: EditBlockReason | undefined): string | undefined {
  if (reason === "encrypted") return message("edit.blocked.encrypted");
  if (reason === "diverged") return message("edit.blocked.diverged");
  if (reason === "no-document") return message("command.reason.noDocument");
  return undefined;
}

/** What a toast says after an undo or redo. */
export function revertedText(reverted: RevertedSlice): string {
  const step = reverted.step.label;
  if (reverted.outcome === "partial") {
    return message(reverted.way === "undo" ? "edit.toast.undidPart" : "edit.toast.redidPart", { step });
  }
  if (reverted.outcome === "skipped") {
    return message(reverted.way === "undo" ? "edit.toast.undoSkipped" : "edit.toast.redoSkipped", { step });
  }
  return message(reverted.way === "undo" ? "edit.toast.undid" : "edit.toast.redid", { step });
}
