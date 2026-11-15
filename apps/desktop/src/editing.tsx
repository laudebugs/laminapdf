// The edit session in the desktop window: the view's session (`viewer.edits`) in the
// commands' context, so Undo and Redo work from the keys, the Edit menu and the palette; turning
// the current page (View ▸ Rotate page); the window's title and the main process told while edits
// aren't saved; and the question before they're lost: closing the tab, the window or the app.
// The desktop doesn't save edits yet: until it does, the question can only keep them or let them go.

import { Button, Dialog, type EditControls, message, quarterTurn, type ReaderController } from "@laminapdf/ui";
import { type ReactNode, useEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import type { DesktopEngine } from "./app.tsx";
import { stopper } from "./engine/stopper.ts";

/** The events a window gets before it closes, or the app quits, with unsaved edits (src-tauri/src/unsaved.rs). */
const CONFIRM_CLOSE_EVENT = "window://confirm-close";
const CONFIRM_QUIT_EVENT = "window://confirm-quit";

/** What losing the edits ends: a document's tab, the window, or the app. */
type Leaving = "tab" | "window" | "quit";

const LEAVE_TEXT: Readonly<Record<Leaving, { title: (file: string) => string; go: string }>> = {
  tab: { title: (file) => `Close ${file} without saving?`, go: "Close without saving" },
  window: { title: () => "Close this window without saving?", go: "Close without saving" },
  quit: { title: () => "Quit without saving?", go: "Quit without saving" },
};

/** The question in words: how many changes go, and that the desktop can't save edits yet. */
function leaveDescription(unsaved: number): string {
  const lost =
    unsaved === 1
      ? "Your change isn't saved, so it will be lost."
      : `Your ${unsaved} changes aren't saved, so they will be lost.`;
  return `${lost} The desktop app can't save edits yet.`;
}

/** A store's subscribe with nothing to hear, while no document is shown. */
const NOTHING_TO_HEAR = () => () => undefined;

/** What the window gets from `useDesktopEditing`. */
export interface DesktopEditing {
  /** The shown document's session, for the commands' context. */
  readonly edits: EditControls | undefined;
  /** The handlers of Rotate page right and left. */
  readonly actions: Readonly<Record<"pages.rotate-right" | "pages.rotate-left", () => void>>;
  /** Whether the shown document has edits not yet saved. */
  readonly unsaved: boolean;
  /** The question before edits are lost, while one is asked. */
  readonly dialog: ReactNode;
}

/** The edit session's part of the window. */
export function useDesktopEditing(
  engine: DesktopEngine | undefined,
  reader: ReaderController,
  fileName: string | undefined,
): DesktopEditing {
  const edits = engine?.viewer.edits;
  const slice = useSyncExternalStore(edits?.subscribe ?? NOTHING_TO_HEAR, () => edits?.getState());
  const unsaved = (slice?.history.unsaved ?? 0) > 0;
  useUnsavedMark(engine, reader);
  const question = useLeaveQuestion(engine, reader);
  const actions = useMemo(() => {
    const rotate = (delta: 90 | -90) => () => {
      if (engine === undefined) return;
      const index = engine.viewer.getState().currentPage;
      const page = engine.client.document?.pages[index];
      if (page === undefined) return;
      const ops = [quarterTurn(page, delta)] as unknown as Parameters<DesktopEngine["viewer"]["edits"]["apply"]>[0];
      void engine.viewer.edits.apply(ops, message("edit.rotate.label", { page: String(index + 1) }));
    };
    return { "pages.rotate-right": rotate(90), "pages.rotate-left": rotate(-90) };
  }, [engine]);
  const dialog =
    question.asking === undefined ? null : (
      <LeaveDialog
        leaving={question.asking.leaving}
        fileName={fileName ?? ""}
        unsaved={question.asking.unsaved}
        onAnswer={question.answer}
      />
    );
  return { edits, actions, unsaved, dialog };
}

/**
 * Tells the main process whether this window holds unsaved edits, in any of its documents, after
 * every change of the shown document's session and of the reader (a document opened or closed).
 * Each new edit tells it again, so a window that agreed to lose its edits to a quit is asked
 * again about the ones it made since.
 */
function useUnsavedMark(engine: DesktopEngine | undefined, reader: ReaderController): void {
  useEffect(() => {
    if (engine === undefined) return;
    const tell = () => {
      engine.shell.invoke("set_unsaved", { unsaved: engine.client.unsaved }).catch(() => undefined);
    };
    tell();
    const stops = [engine.viewer.edits.subscribe(tell), reader.subscribe(tell)];
    return () => {
      for (const stop of stops) stop();
    };
  }, [engine, reader]);
}

/** The question waiting for an answer, and how it resolves. */
interface Asking {
  readonly leaving: Leaving;
  readonly unsaved: number;
  readonly resolve: (leave: boolean) => void;
}

/**
 * Asks before edits are lost: the reader's close guard for the shown tab, and the main process's
 * questions before the window closes or the app quits. Leaving clears the window's mark first, so
 * the close or quit it asks again goes through.
 */
function useLeaveQuestion(
  engine: DesktopEngine | undefined,
  reader: ReaderController,
): { asking: Asking | undefined; answer: (leave: boolean) => void } {
  const [asking, setAsking] = useState<Asking | undefined>(undefined);
  const pending = useRef<Asking | undefined>(undefined);
  useEffect(() => {
    if (engine === undefined) return;
    const ask = (leaving: Leaving, unsaved: number): Promise<boolean> => {
      // A second question while one shows (two closes in a row) answers the first "stay".
      pending.current?.resolve(false);
      return new Promise<boolean>((resolve) => {
        const next = { leaving, unsaved, resolve };
        pending.current = next;
        setAsking(next);
      });
    };
    const leaveWindow = (leaving: "window" | "quit") => () => {
      void ask(leaving, engine.client.unsavedSteps).then((leave) => afterAnswer(engine, leaving, leave));
    };
    const stopGuard = reader.setCloseGuard(() => {
      const unsaved = engine.viewer.edits.getState().history.unsaved;
      return unsaved === 0 ? Promise.resolve(true) : ask("tab", unsaved);
    });
    const stopClose = stopper(
      engine.shell.listen(CONFIRM_CLOSE_EVENT, leaveWindow("window")),
      "listening for a close to confirm",
    );
    const stopQuit = stopper(
      engine.shell.listen(CONFIRM_QUIT_EVENT, leaveWindow("quit")),
      "listening for a quit to confirm",
    );
    return () => {
      stopGuard();
      stopClose();
      stopQuit();
    };
  }, [engine, reader]);
  const answer = (leave: boolean) => {
    pending.current?.resolve(leave);
    pending.current = undefined;
    setAsking(undefined);
  };
  return { asking, answer };
}

/**
 * Closes or quits once the user answered. A close clears this window's mark and closes again. A
 * quit goes on saying this window may lose its edits, which keeps its mark should another window
 * cancel; a Cancel here tells the main process, which asks every window again next time.
 */
async function afterAnswer(engine: DesktopEngine, leaving: "window" | "quit", leave: boolean): Promise<void> {
  const { shell } = engine;
  if (leaving === "quit") {
    await shell.invoke(leave ? "quit" : "cancel_quit", leave ? { discard: true } : {}).catch(() => undefined);
    return;
  }
  if (!leave) return;
  await shell.invoke("set_unsaved", { unsaved: false }).catch(() => undefined);
  await shell.invoke("close_window").catch(() => undefined);
}

/** The question (Paper O03 without Save, which the desktop doesn't have yet): lose the changes, or Cancel. */
function LeaveDialog({
  leaving,
  fileName,
  unsaved,
  onAnswer,
}: {
  leaving: Leaving;
  fileName: string;
  unsaved: number;
  onAnswer: (leave: boolean) => void;
}): ReactNode {
  const text = LEAVE_TEXT[leaving];
  return (
    <Dialog
      isOpen
      isRequired
      role="alertdialog"
      onOpenChange={(open) => {
        if (!open) onAnswer(false);
      }}
      onEscape={() => onAnswer(false)}
      title={text.title(fileName)}
      description={leaveDescription(unsaved)}
      actions={
        <>
          <Button variant="danger" onPress={() => onAnswer(true)}>
            {text.go}
          </Button>
          <Button variant="primary" autoFocus onPress={() => onAnswer(false)}>
            {message("edit.close.cancel")}
          </Button>
        </>
      }
    />
  );
}
