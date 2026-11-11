// The edit session's toasts (Paper O01): after an undo or redo, what it did and the way back
// (Redo after an undo, Undo after a redo); after one that restored only part of a step, or none,
// what was left alone; and after a call that failed, what couldn't be done. One edit toast shows
// at a time: the next one replaces it, and any move of the history dismisses it, so its action
// never acts on a step it doesn't name.

import { inSentence, message, type RevertedSlice, type ToastHandle, type ToastQueue } from "@laminapdf/ui";
import { revertedText } from "@laminapdf/ui/editing";
import type { EditFailure, EditSession } from "@laminapdf/viewer";

/** What the toasts read of the session: its history, its events and the way back. */
export type ToastedSession = Pick<EditSession, "getState" | "subscribe" | "onReverted" | "onFailed" | "undo" | "redo">;

/** The words for a failed call: "Couldn't rotate page 5. Try again." */
export function failureText(failure: Omit<EditFailure, "error">): string {
  const action =
    failure.call === "apply"
      ? failure.label === undefined || failure.label === ""
        ? message("edit.failed.edit")
        : inSentence(failure.label)
      : message(
          failure.call === "goTo"
            ? "edit.failed.goTo"
            : failure.call === "undo"
              ? "edit.failed.undo"
              : "edit.failed.redo",
        );
  return message("edit.toast.failed", { action });
}

/** Where the history stands: its steps (by ID) and how many are done, as one comparable value. */
function where(history: ReturnType<ToastedSession["getState"]>["history"]): string {
  return `${history.position}:${history.steps.map((step) => step.id).join(",")}`;
}

/** Shows `session`'s toasts in `toasts`; returns the function that stops. */
export function showEditToasts(session: ToastedSession, toasts: ToastQueue): () => void {
  let shown: ToastHandle | undefined;
  let at = where(session.getState().history);
  const dismiss = () => {
    shown?.dismiss();
    shown = undefined;
  };
  const show = (text: string, action?: { label: string; onPress: () => void }) => {
    dismiss();
    shown = toasts.add({ message: text, ...(action === undefined ? {} : { action }) });
  };
  const way = (step: RevertedSlice) =>
    step.outcome === "skipped"
      ? undefined
      : step.way === "undo"
        ? { label: message("command.edit.redo"), onPress: () => void session.redo().catch(() => undefined) }
        : { label: message("command.edit.undo"), onPress: () => void session.undo().catch(() => undefined) };
  const stops = [
    // The history moved: an older toast's action would act on another step. A history read again
    // after a failed call, the same steps at the same place, leaves the toast.
    session.subscribe(() => {
      const next = where(session.getState().history);
      if (next !== at) dismiss();
      at = next;
    }),
    session.onReverted((reverted) => {
      // Going far away is one choice in the History tab, which shows where it went: only a step
      // left alone is worth a toast then.
      const notable = reverted.length === 1 ? reverted : reverted.filter((r) => r.outcome !== "applied");
      const [first] = notable;
      if (first === undefined) return;
      // One toast says it all: several steps left alone in part or whole are counted, not listed.
      if (notable.length === 1) show(revertedText(first), way(first));
      else show(message("edit.toast.several", { count: notable.length }));
    }),
    session.onFailed((failure) => show(failureText(failure))),
  ];
  return () => {
    for (const stop of stops) stop();
    // The document went: a toast about its steps would act on the next one's.
    dismiss();
  };
}
