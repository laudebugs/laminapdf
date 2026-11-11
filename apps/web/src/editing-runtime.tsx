// The parts of the edit session that wait until a document is open (loaded with import(), as the
// print and properties dialogs are, #380): the edit commands registered, a toast after each undo
// or redo with the way back, and for one that restored only part of its step; the confirmation
// before the reader closes or replaces a document with unsaved changes; and the interaction mode
// that reads pointer input on the page (select, unless a later tool is chosen).

import {
  Button,
  type CommandRegistry,
  countMessage,
  Dialog,
  type EditControls,
  message,
  type ReaderController,
  type RevertedSlice,
  type ToastQueue,
  useStoreState,
} from "@laminapdf/ui";
import { attachTools, registerEditCommands, revertedText, ToolRegistry } from "@laminapdf/ui/editing";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useRef, useState } from "react";
import type { Engine } from "./session.ts";

/** The toast after an undo or redo: what it did, and the way back when it did all of it. */
function revertedToast(toasts: ToastQueue, edits: EditControls, reverted: readonly RevertedSlice[]): void {
  // Going to a step far away is one choice in the History tab, which shows where it went: only a
  // step left alone is worth a toast then.
  const notable = reverted.length === 1 ? reverted : reverted.filter((r) => r.outcome !== "applied");
  for (const step of notable) {
    const back =
      step.outcome === "skipped"
        ? undefined
        : step.way === "undo"
          ? { label: message("command.edit.redo"), onPress: () => void edits.redo().catch(() => undefined) }
          : { label: message("command.edit.undo"), onPress: () => void edits.undo().catch(() => undefined) };
    toasts.add({ message: revertedText(step), ...(back === undefined ? {} : { action: back }) });
  }
}

/** Toasts after undos and redos, and when a restarted worker lost the edits. */
function useEditToasts(viewer: Viewer, engine: Engine, toasts: ToastQueue): void {
  useEffect(() => {
    return viewer.edits.onReverted((reverted) => revertedToast(toasts, viewer.edits, reverted));
  }, [viewer, toasts]);
  const lost = useStoreState(engine, (s) => s.editsLost);
  const seen = useRef(lost);
  useEffect(() => {
    if (lost === seen.current) return;
    seen.current = lost;
    toasts.add({ message: message("edit.toast.lost") });
  }, [lost, toasts]);
}

/**
 * The reader's close guard while there are unsaved changes: the confirmation's state, resolved by
 * its buttons. Clean documents close without asking.
 */
function useCloseGuard(reader: ReaderController, edits: EditControls) {
  const [asking, setAsking] = useState<((leave: boolean) => void) | undefined>(undefined);
  useEffect(() => {
    return reader.setCloseGuard(() =>
      edits.getState().history.unsaved === 0
        ? Promise.resolve(true)
        : new Promise<boolean>((resolve) => setAsking(() => resolve)),
    );
  }, [reader, edits]);
  const answer = (leave: boolean) => {
    asking?.(leave);
    setAsking(undefined);
  };
  return { asking: asking !== undefined, answer };
}

/** The confirmation (O03 without Save, which comes with saving): close and lose the changes, or keep them. */
function CloseDialog({
  isOpen,
  fileName,
  unsaved,
  onAnswer,
}: {
  isOpen: boolean;
  fileName: string;
  unsaved: number;
  onAnswer: (leave: boolean) => void;
}): ReactNode {
  return (
    <Dialog
      isOpen={isOpen}
      isRequired
      role="alertdialog"
      onOpenChange={(open) => {
        if (!open) onAnswer(false);
      }}
      onEscape={() => onAnswer(false)}
      title={message("edit.close.title", { file: fileName })}
      description={countMessage(unsaved, { one: "edit.close.one", many: "edit.close.many" })}
      actions={
        <>
          <Button autoFocus onPress={() => onAnswer(false)}>
            {message("edit.close.cancel")}
          </Button>
          <Button variant="danger-filled" onPress={() => onAnswer(true)}>
            {message("edit.close.discard")}
          </Button>
        </>
      }
    />
  );
}

/** Passes pointer input on the view to the interaction mode on. */
function useTools(viewer: Viewer): void {
  const [tools] = useState(() => new ToolRegistry());
  useEffect(() => {
    const element = viewer.element;
    return attachTools(tools, { element, scrollBy: (dx, dy) => element.scrollBy(dx, dy) });
  }, [viewer, tools]);
}

/** Props of `EditingRuntime`. */
export interface EditingRuntimeProps {
  readonly engine: Engine;
  readonly viewer: Viewer;
  readonly registry: CommandRegistry;
  readonly toasts: ToastQueue;
  readonly fileName: string;
}

/** The edit session's runtime for the document `viewer` shows; draws the close confirmation. */
export function EditingRuntime({ engine, viewer, registry, toasts, fileName }: EditingRuntimeProps): ReactNode {
  useEffect(() => registerEditCommands(registry), [registry]);
  useEditToasts(viewer, engine, toasts);
  useTools(viewer);
  const guard = useCloseGuard(engine.reader, viewer.edits);
  const unsaved = useStoreState(viewer.edits, (s) => s.history.unsaved);
  return <CloseDialog isOpen={guard.asking} fileName={fileName} unsaved={unsaved} onAnswer={guard.answer} />;
}
