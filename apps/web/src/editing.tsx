// The edit session in the reader (Paper O01): the view's session (`viewer.edits`) in the commands'
// context, with the shell's handlers for Show history and turning the current page; the More
// menu's edit items; the History tab once an edit was made; the browser tab's "•" while there are
// unsaved changes, and the browser's own prompt when the tab closes (`beforeunload`). What waits
// until a document is open (the commands, toasts, the close confirmation, the tools) is in
// editing-runtime.tsx, which loads then, with the History tab's panel, so none of it is in the
// startup script (#380).

import {
  type CommandRegistry,
  type EditControls,
  type GatedEntry,
  inSentence,
  type KeyPlatform,
  message,
  type RailTab,
  type ShellLayout,
  sessionStarted,
  stepToRedo,
  stepToUndo,
  type ToastQueue,
  useStoreState,
} from "@laminapdf/ui";
import type * as Editor from "@laminapdf/ui/editing";
import type { Viewer } from "@laminapdf/viewer";
import { lazy, type ReactNode, Suspense, useEffect, useMemo } from "react";
import { type PageInfo, pageLabel } from "./outline.ts";
import type { Engine } from "./session.ts";

/** The History tab's id in the rail. */
export const HISTORY_TAB = "history";

/** The runtime and the History tab's panel load once a document is open (later.ts). */
const loadRuntime = () => import("./later.ts");
const HistoryPanel = lazy<typeof Editor.HistoryPanel>(() => loadRuntime().then((m) => ({ default: m.HistoryPanel })));
const EditingRuntime = lazy(() => loadRuntime().then((m) => ({ default: m.EditingRuntime })));
const EditArea = lazy<typeof Editor.EditArea>(() => loadRuntime().then((m) => ({ default: m.EditArea })));

/** A session for a top bar without a document: nothing done, nothing to do. Its state is one object, as a store's must be. */
const NOTHING: ReturnType<EditControls["getState"]> = {
  history: { steps: [], position: 0, dropped: 0, unsaved: 0 },
  busy: false,
  blocked: "no-document",
  notice: undefined,
};
const done = () => Promise.resolve();
const NO_SESSION: EditControls = {
  getState: () => NOTHING,
  subscribe: () => () => undefined,
  undo: done,
  redo: done,
  goTo: done,
};

/** The More menu's edit items (the web's Edit menu, O01). */
export type EditMenuId = "undo" | "redo" | "history" | "rotate-right" | "rotate-left";

/** What the reader gets from `useEditing`. */
export interface Editing {
  /** The open document's session, for the commands, the top bar and the History tab. */
  readonly edits: EditControls | undefined;
  /** Handlers of the delegated edit commands. */
  readonly actions: Readonly<Record<"edit.history" | "pages.rotate-right" | "pages.rotate-left", () => void>>;
  /** The top bar's save state, Undo and Redo, while a document is open. */
  readonly topBar: ReactNode;
  /** The History tab, once the document has an edit session. */
  readonly tab: RailTab | undefined;
  /** The More menu's edit items and what each does. */
  readonly menu: { readonly items: GatedEntry<EditMenuId>[]; readonly run: (id: EditMenuId) => void };
  /** Whether the document has changes no save has written. */
  readonly unsaved: boolean;
  /**
   * What runs while a document is open, given the commands' registry: the edit commands, the
   * toasts, the tools, and the confirmation before a document with unsaved changes closes.
   */
  readonly runtime: (registry: CommandRegistry) => ReactNode;
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

/** The More menu's edit items, named as their commands are. */
function menuItems(edits: EditControls | undefined): GatedEntry<EditMenuId>[] {
  const state = (edits ?? NO_SESSION).getState();
  const blocked = state.blocked !== undefined;
  /** Undo or Redo, named by the step it would take. */
  const step = (way: "undo" | "redo", other: string): GatedEntry<EditMenuId> => {
    const next = (way === "undo" ? stepToUndo : stepToRedo)(state);
    return {
      id: way,
      label:
        next === undefined
          ? message(`command.edit.${way}`)
          : message(`edit.${way}.step`, { step: inSentence(next.label) }),
      icon: way,
      shortcut: { mac: way === "undo" ? "Mod+Z" : "Mod+Shift+Z", other },
      isDisabled: blocked || next === undefined,
    };
  };
  return [
    step("undo", "Mod+Z"),
    step("redo", "Mod+Y"),
    { id: "history", label: message("command.edit.history"), icon: "clock", isDisabled: edits === undefined },
    { id: "rotate-right", label: message("command.pages.rotate-right"), icon: "rotate-right", isDisabled: blocked },
    { id: "rotate-left", label: message("command.pages.rotate-left"), icon: "rotate-left", isDisabled: blocked },
  ];
}

/** The edit session's part of the reader, for the document `viewer` shows (undefined: none). */
export function useEditing(options: {
  readonly engine: Engine;
  readonly layout: ShellLayout;
  readonly viewer: Viewer | undefined;
  readonly toasts: ToastQueue;
  readonly fileName: string | undefined;
  readonly pages: readonly PageInfo[];
  /** How shortcut chips are written. */
  readonly keys: KeyPlatform;
}): Editing {
  const { engine, layout, viewer, toasts } = options;
  const edits = viewer?.edits;
  const slice = useStoreState(edits ?? NO_SESSION, (s) => s);
  const started = sessionStarted(slice);
  const unsaved = slice.history.unsaved;
  const openedAt = useStoreState(engine, (s) => s.openedAt);
  useBeforeUnload(unsaved > 0);
  const { pages, fileName } = options;
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
              <Suspense fallback={null}>
                <HistoryPanel
                  edits={edits}
                  headingLevel={3}
                  pageName={(index) => pageLabel(pages, index)}
                  {...(openedAt === undefined ? {} : { openedAt })}
                />
              </Suspense>
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
    topBar:
      edits === undefined ? undefined : (
        <Suspense fallback={null}>
          <EditArea edits={edits} keys={options.keys} />
        </Suspense>
      ),
    runtime: (registry) =>
      viewer === undefined || fileName === undefined ? null : (
        <Suspense fallback={null}>
          <EditingRuntime engine={engine} viewer={viewer} registry={registry} toasts={toasts} fileName={fileName} />
        </Suspense>
      ),
  };
}
