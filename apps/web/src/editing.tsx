// The edit session in the reader (Paper O01): the view's session (`viewer.edits`) in the commands'
// context, with the shell's handlers for Show history and turning the current page; the History
// tab once an edit was made; a toast after each undo or redo, with the way back, and for one that
// restored only part of its step; the browser tab's "•" while there are unsaved changes; and the
// close guards: the browser's own prompt when the tab closes (`beforeunload`), and a confirmation
// before the reader closes or replaces a document with unsaved changes. Pointer input on the page
// goes to the interaction mode on (select, unless a later tool is chosen).

import {
  attachTools,
  Button,
  countMessage,
  Dialog,
  type EditControls,
  type GatedEntry,
  HistoryPanel,
  inSentence,
  message,
  type RailTab,
  type ReaderController,
  type RevertedSlice,
  revertedText,
  type ShellLayout,
  sessionStarted,
  stepToRedo,
  stepToUndo,
  type ToastQueue,
  ToolRegistry,
  useStoreState,
} from "@laminapdf/ui";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useMemo, useRef, useState } from "react";
import { type PageInfo, pageLabel } from "./outline.ts";
import type { Engine } from "./session.ts";

/** The History tab's id in the rail. */
export const HISTORY_TAB = "history";

/** A session for a top bar without a document: nothing done, nothing to do. Its state is one object, as a store's must be. */
const NOTHING: ReturnType<EditControls["getState"]> = {
  history: { steps: [], position: 0, dropped: 0, unsaved: 0 },
  busy: false,
  blocked: "no-document",
  notice: undefined,
};
const NO_SESSION: EditControls = {
  getState: () => NOTHING,
  subscribe: () => () => undefined,
  undo: () => Promise.resolve(),
  redo: () => Promise.resolve(),
  goTo: () => Promise.resolve(),
};

/** The More menu's edit items (the web's Edit menu, O01). */
export type EditMenuId = "undo" | "redo" | "history" | "rotate-right" | "rotate-left";

/** What the reader gets from `useEditing`. */
export interface Editing {
  /** The open document's session, for the commands, the top bar and the History tab. */
  readonly edits: EditControls | undefined;
  /** Handlers of the delegated edit commands. */
  readonly actions: Readonly<Record<"edit.history" | "pages.rotate-right" | "pages.rotate-left", () => void>>;
  /** The History tab, once the document has an edit session. */
  readonly tab: RailTab | undefined;
  /** The More menu's edit items and what each does. */
  readonly menu: { readonly items: GatedEntry<EditMenuId>[]; readonly run: (id: EditMenuId) => void };
  /** Whether the document has changes no save has written. */
  readonly unsaved: boolean;
  /** The confirmation before a document with unsaved changes closes. */
  readonly dialog: ReactNode;
}

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
function useEditToasts(viewer: Viewer | undefined, engine: Engine, toasts: ToastQueue): void {
  useEffect(() => {
    if (viewer === undefined) return;
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

/**
 * The reader's close guard while there are unsaved changes: the confirmation's state, resolved by
 * its buttons. Clean documents close without asking.
 */
function useCloseGuard(reader: ReaderController, edits: EditControls | undefined) {
  const [asking, setAsking] = useState<((leave: boolean) => void) | undefined>(undefined);
  useEffect(() => {
    if (edits === undefined) return;
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

/** The More menu's edit items, named as their commands are. */
function menuItems(edits: EditControls | undefined): GatedEntry<EditMenuId>[] {
  const state = (edits ?? NO_SESSION).getState();
  const blocked = state.blocked !== undefined;
  const undo = stepToUndo(state);
  const redo = stepToRedo(state);
  return [
    {
      id: "undo",
      label:
        undo === undefined ? message("command.edit.undo") : message("edit.undo.step", { step: inSentence(undo.label) }),
      icon: "undo",
      shortcut: { mac: "Mod+Z", other: "Mod+Z" },
      isDisabled: blocked || undo === undefined,
    },
    {
      id: "redo",
      label:
        redo === undefined ? message("command.edit.redo") : message("edit.redo.step", { step: inSentence(redo.label) }),
      icon: "redo",
      shortcut: { mac: "Mod+Shift+Z", other: "Mod+Y" },
      isDisabled: blocked || redo === undefined,
    },
    { id: "history", label: message("command.edit.history"), icon: "clock", isDisabled: edits === undefined },
    { id: "rotate-right", label: message("command.pages.rotate-right"), icon: "rotate-right", isDisabled: blocked },
    { id: "rotate-left", label: message("command.pages.rotate-left"), icon: "rotate-left", isDisabled: blocked },
  ];
}

/** Passes pointer input on the view to the interaction mode on. */
function useTools(viewer: Viewer | undefined): void {
  const [tools] = useState(() => new ToolRegistry());
  useEffect(() => {
    if (viewer === undefined) return;
    const element = viewer.element;
    return attachTools(tools, { element, scrollBy: (dx, dy) => element.scrollBy(dx, dy) });
  }, [viewer, tools]);
}

/** The edit session's part of the reader, for the document `viewer` shows (undefined: none). */
export function useEditing(options: {
  readonly engine: Engine;
  readonly layout: ShellLayout;
  readonly viewer: Viewer | undefined;
  readonly toasts: ToastQueue;
  readonly fileName: string | undefined;
  readonly pages: readonly PageInfo[];
}): Editing {
  const { engine, layout, viewer, toasts } = options;
  const edits = viewer?.edits;
  const slice = useStoreState(edits ?? NO_SESSION, (s) => s);
  const started = sessionStarted(slice);
  const unsaved = slice.history.unsaved;
  const openedAt = useStoreState(engine, (s) => s.openedAt);
  useEditToasts(viewer, engine, toasts);
  useBeforeUnload(unsaved > 0);
  useTools(viewer);
  const guard = useCloseGuard(engine.reader, edits);
  const { pages } = options;
  const actions = useMemo(() => {
    const current = () => viewer?.getState().currentPage ?? 0;
    const rotate = (delta: 90 | -90) => () =>
      void engine.rotatePage(current(), delta).catch(() => toasts.add({ message: message("edit.toast.failed") }));
    return {
      "edit.history": () => {
        layout.setRailTab(HISTORY_TAB);
        layout.setRailOpen(true);
      },
      "pages.rotate-right": rotate(90),
      "pages.rotate-left": rotate(-90),
    };
  }, [engine, layout, viewer, toasts]);
  const tab = useMemo<RailTab | undefined>(
    () =>
      edits === undefined || !started
        ? undefined
        : {
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
          },
    [edits, started, pages, openedAt],
  );
  const run = (id: EditMenuId) => {
    if (id === "undo") void edits?.undo().catch(() => undefined);
    else if (id === "redo") void edits?.redo().catch(() => undefined);
    else if (id === "history") actions["edit.history"]();
    else actions[id === "rotate-right" ? "pages.rotate-right" : "pages.rotate-left"]();
  };
  return {
    edits,
    actions,
    tab,
    menu: { items: menuItems(edits), run },
    unsaved: unsaved > 0,
    dialog: (
      <CloseDialog isOpen={guard.asking} fileName={options.fileName ?? ""} unsaved={unsaved} onAnswer={guard.answer} />
    ),
  };
}
