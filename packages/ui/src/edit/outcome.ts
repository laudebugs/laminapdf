// The words of an edit command that can't run and of an undo's outcome, which only the parts an
// editor loads once a document is open use (the commands, the History tab, the toasts), so they
// stay out of the startup script (#380).

import { message } from "../messages.js";
import type { EditBlockReason, RevertedSlice } from "./edit-controls.js";

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
    // The verb is the failure words' ("undo", "redo"), so the sentence is written once.
    const action = message(reverted.way === "undo" ? "edit.failed.undo" : "edit.failed.redo");
    return message("edit.toast.skipped", { action, step });
  }
  return message(reverted.way === "undo" ? "edit.toast.undid" : "edit.toast.redid", { step });
}
