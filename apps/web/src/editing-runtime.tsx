// The parts of the edit session that wait until a document is open (loaded with import(), as the
// print and properties dialogs are, #380): the edit commands registered, the toasts (edit-toasts.ts);
// the confirmation before the reader closes or replaces a document with unsaved changes (O03, with
// Save); saving's questions and its strip (save-flow.ts, O02); and the interaction mode that reads
// pointer input on the page (select, unless a later tool is chosen).

import {
  Button,
  type CommandRegistry,
  countMessage,
  Dialog,
  type EditControls,
  message,
  Notice,
  type RailTab,
  type ReaderController,
  type ShellLayout,
  type ToastQueue,
  useStoreState,
} from "@laminapdf/ui";
import {
  attachTools,
  ConformanceDialog,
  HistoryPanel,
  RemoveSignaturesDialog,
  registerEditCommands,
  SaveStrip,
  ToolRegistry,
} from "@laminapdf/ui/editing";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useRef, useState } from "react";
import { showEditToasts } from "./edit-toasts.ts";
import { type EditActions, HISTORY_TAB } from "./editing.tsx";
import { type PageInfo, pageLabel } from "./outline.ts";
import type { SaveFlow } from "./save-flow.ts";
import type { Engine } from "./session.ts";

/** Toasts after undos and redos, and when a restarted worker lost the edits. */
function useEditToasts(viewer: Viewer, engine: Engine, toasts: ToastQueue): void {
  useEffect(() => showEditToasts(viewer.edits, toasts), [viewer, toasts]);
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

/**
 * The confirmation (O03): Save, then close; Don't save, which loses the changes; or Cancel. A save
 * that fails or is cancelled keeps the document open, the failure shown under the top bar.
 */
function CloseDialog({
  isOpen,
  fileName,
  unsaved,
  onAnswer,
  onSave,
}: {
  isOpen: boolean;
  fileName: string;
  unsaved: number;
  onAnswer: (leave: boolean) => void;
  onSave: () => Promise<boolean>;
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
          <Button variant="danger" onPress={() => onAnswer(true)}>
            {message("edit.close.discard")}
          </Button>
          <Button onPress={() => onAnswer(false)}>{message("edit.close.cancel")}</Button>
          <Button variant="primary" autoFocus onPress={() => void onSave().then(onAnswer)}>
            {message("edit.close.save")}
          </Button>
        </>
      }
    />
  );
}

/** Passes pointer input on the view to the interaction mode on. */
function useTools(viewer: Viewer, reader: ReaderController): void {
  const [tools] = useState(() => new ToolRegistry());
  useEffect(() => {
    const element = viewer.element;
    return attachTools(tools, { element, scrollBy: (dx, dy) => element.scrollBy(dx, dy) });
  }, [viewer, tools]);
  // One mode: the reader machine's. A `set-mode` turns the tool on; a name no tool has goes back
  // to the tool on, so the machine never names a mode nothing reads.
  useEffect(() => {
    const follow = () => {
      const state = reader.getState();
      if (state.phase !== "open" || state.mode === tools.getState().current) return;
      if (!tools.use(state.mode)) reader.setMode(tools.getState().current);
    };
    follow();
    const stops = [reader.subscribe(follow), tools.subscribe(() => reader.setMode(tools.getState().current))];
    return () => {
      for (const stop of stops) stop();
    };
  }, [reader, tools]);
}

/** Props of `EditingRuntime`. */
export interface EditingRuntimeProps {
  readonly engine: Engine;
  readonly viewer: Viewer;
  readonly registry: CommandRegistry;
  readonly toasts: ToastQueue;
  readonly fileName: string;
  readonly flow: SaveFlow;
}

/** Asks the browser to confirm closing the tab while `unsaved`; its own prompt, whose words can't be set. */
function useBeforeUnload(unsaved: boolean): void {
  useEffect(() => {
    if (!unsaved) return;
    const ask = (event: BeforeUnloadEvent) => {
      event.preventDefault();
      // Older browsers ask only when `returnValue` is set.
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", ask);
    return () => window.removeEventListener("beforeunload", ask);
  }, [unsaved]);
}

/** The edit session's runtime for the document `viewer` shows; draws the close confirmation and saving's questions. */
export function EditingRuntime({ engine, viewer, registry, toasts, fileName, flow }: EditingRuntimeProps): ReactNode {
  useEffect(() => registerEditCommands(registry), [registry]);
  useEffect(() => flow.start(), [flow]);
  useEditToasts(viewer, engine, toasts);
  useTools(viewer, engine.reader);
  const guard = useCloseGuard(engine.reader, viewer.edits);
  const unsaved = useStoreState(viewer.edits, (s) => s.history.unsaved);
  const asking = useStoreState(flow, (s) => s.asking);
  useBeforeUnload(unsaved > 0);
  return (
    <>
      <CloseDialog
        isOpen={guard.asking}
        fileName={fileName}
        unsaved={unsaved}
        onAnswer={guard.answer}
        onSave={() => flow.save("save")}
      />
      <RemoveSignaturesDialog
        signatures={asking?.kind === "signatures" ? asking.signatures : undefined}
        onAnswer={(yes) => flow.answer(yes)}
      />
      <ConformanceDialog
        warnings={asking?.kind === "conformance" ? asking.warnings : undefined}
        onAnswer={(yes) => flow.answer(yes)}
      />
    </>
  );
}

/**
 * Under the top bar: how the file saves when that isn't plain (signed, rewritten, or a save that
 * breaks a signature), and the last save that failed, with Try again (O02).
 */
export function SaveNotices({ flow }: { flow: SaveFlow }): ReactNode {
  const plan = useStoreState(flow, (s) => s.plan);
  const failure = useStoreState(flow, (s) => s.failure);
  return (
    <>
      {failure === undefined ? null : (
        <div className="web-save-failure">
          <Notice tone="danger" title={message("save.failed", failure)} />
          <Button size={26} onPress={() => flow.retry()}>
            {message("save.failed.retry")}
          </Button>
          <Button variant="ghost" size={26} onPress={() => flow.dismissFailure()}>
            {message("save.failed.dismiss")}
          </Button>
        </div>
      )}
      <SaveStrip plan={plan} />
    </>
  );
}

/** The History tab (O01), once the document has an edit session: the rail lists it last. */
export function historyTab(edits: EditControls, pages: readonly PageInfo[], openedAt: number | undefined): RailTab {
  return {
    id: HISTORY_TAB,
    label: message("edit.history"),
    panel: (
      <HistoryPanel
        edits={edits}
        headingLevel={3}
        pageName={(index) => pageLabel(pages, index)}
        {...(openedAt === undefined ? {} : { openedAt })}
      />
    ),
  };
}

/**
 * The delegated edit commands' handlers for the document `viewer` shows: Show history, the page
 * turns, the saves, and Download a copy with the edits while there are any (`unsaved`).
 */
export function editActions(
  engine: Engine,
  layout: ShellLayout,
  viewer: Viewer,
  flow: SaveFlow,
  unsaved: boolean,
): EditActions {
  // A failed turn is said by the session's failure toast (edit-toasts.ts).
  const rotate = (delta: 90 | -90) => () =>
    void engine.rotatePage(viewer.getState().currentPage, delta).catch(() => undefined);
  return {
    "edit.history": () => {
      layout.setRailTab(HISTORY_TAB);
      layout.setRailOpen(true);
    },
    "pages.rotate-right": rotate(90),
    "pages.rotate-left": rotate(-90),
    "file.save": () => void flow.save("save"),
    "file.save-as": () => void flow.save("save-as"),
    "file.save-optimised": () => void flow.save("optimised"),
    ...(unsaved ? { "file.save-copy": () => void flow.copy() } : {}),
  };
}
