// The edit session in the reader (Paper O01): the view's session (`viewer.edits`) in the commands'
// context, with the shell's handlers for Show history and turning the current page; the More
// menu's edit items; the History tab once an edit was made; the browser tab's "•" while there are
// unsaved changes, and the browser's own prompt when the tab closes (`beforeunload`). What waits
// until a document is open (the commands, toasts, the close confirmation, the tools) is in
// editing-runtime.tsx, which loads then, with the History tab's panel, so none of it is in the
// startup script (#380).

import {
  type CommandContext,
  type CommandId,
  type CommandRegistry,
  chipKeys,
  type EditControls,
  type GatedEntry,
  type KeyPlatform,
  message,
  type Platform,
  type RailTab,
  type ShellLayout,
  sessionStarted,
  type ToastQueue,
  useStoreState,
} from "@laminapdf/ui";
import type * as Editor from "@laminapdf/ui/editing";
import { CLOSED_SESSION, type Viewer } from "@laminapdf/viewer";
import { lazy, type ReactNode, Suspense, useEffect, useMemo, useSyncExternalStore } from "react";
import { pageLabel } from "./outline.ts";
import type { Engine } from "./session.ts";

/** The History tab's id in the rail. */
export const HISTORY_TAB = "history";

/** The runtime and the History tab's panel load once a document is open (later.ts). */
const loadRuntime = () => import("./later.ts");
const HistoryPanel = lazy<typeof Editor.HistoryPanel>(() => loadRuntime().then((m) => ({ default: m.HistoryPanel })));
const EditingRuntime = lazy(() => loadRuntime().then((m) => ({ default: m.EditingRuntime })));
const EditArea = lazy<typeof Editor.EditArea>(() => loadRuntime().then((m) => ({ default: m.EditArea })));

/** The More menu's edit items (the web's Edit menu, O01), and the command each runs. */
const EDIT_MENU = {
  undo: "edit.undo",
  redo: "edit.redo",
  history: "edit.history",
  "rotate-right": "pages.rotate-right",
  "rotate-left": "pages.rotate-left",
} as const satisfies Record<string, CommandId>;

/** An edit item of the More menu. */
export type EditMenuId = keyof typeof EDIT_MENU;

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
  /** How long the last edit, undo or redo took from the call to its pages redrawn, in ms (#debug). */
  readonly lastMs: number | undefined;
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

/**
 * The More menu's edit items, from their registered commands (ADR 0024 §3): the title, the
 * shortcut and whether each can run now. Before the commands register (they load once a document
 * is open) there are none.
 */
function menuItems(
  registry: CommandRegistry,
  context: CommandContext,
  /**
   * What the items are read from, besides the context: the registry's version and the session's
   * state. Named here so the React Compiler makes the items again when either changes.
   */
  _readFrom: { readonly version: number; readonly state: unknown },
): GatedEntry<EditMenuId>[] {
  return (Object.entries(EDIT_MENU) as [EditMenuId, CommandId][]).flatMap(([id, commandId]) => {
    const command = registry.get(commandId);
    const availability = registry.availability(commandId, context);
    if (command === undefined || availability.state === "hidden") return [];
    const shortcut = chipKeys(command, context);
    return [
      {
        id,
        label: registry.title(command, context),
        ...(command.icon === undefined ? {} : { icon: command.icon }),
        ...(shortcut === undefined ? {} : { shortcut }),
        ...(availability.state === "disabled" ? { isDisabled: true, disabledReason: availability.reason } : {}),
      },
    ];
  });
}

/** The edit session's part of the reader, for the document `viewer` shows (undefined: none). */
export function useEditing(options: {
  readonly engine: Engine;
  /** The rail's layout, and the commands' registry, which the edit commands join once a document is open. */
  readonly shell: { readonly layout: ShellLayout; readonly registry: CommandRegistry };
  readonly viewer: Viewer | undefined;
  readonly toasts: ToastQueue;
  readonly fileName: string | undefined;
  /** How shortcut chips are written. */
  readonly keys: KeyPlatform;
  readonly platform: Platform;
}): Editing {
  const { engine, viewer, toasts, platform } = options;
  const { layout, registry } = options.shell;
  const pages = useStoreState(engine, (s) => s.pages);
  const edits = viewer?.edits;
  const slice = useStoreState(edits ?? CLOSED_SESSION, (s) => s);
  // The menu follows the registry too: the edit commands register after the first paint.
  const version = useSyncExternalStore(
    (listener) => registry.subscribe(listener),
    () => registry.version,
  );
  const started = sessionStarted(slice);
  const unsaved = slice.history.unsaved;
  const openedAt = useStoreState(engine, (s) => s.openedAt);
  useBeforeUnload(unsaved > 0);
  const { fileName } = options;
  const actions = useMemo(() => {
    const current = () => viewer?.getState().currentPage ?? 0;
    // A failed turn is said by the session's failure toast (edit-toasts.ts).
    const rotate = (delta: 90 | -90) => () => void engine.rotatePage(current(), delta).catch(() => undefined);
    return {
      "edit.history": () => {
        layout.setRailTab(HISTORY_TAB);
        layout.setRailOpen(true);
      },
      "pages.rotate-right": rotate(90),
      "pages.rotate-left": rotate(-90),
    };
  }, [engine, layout, viewer]);
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
  const context: CommandContext = { platform, ...(edits === undefined ? {} : { edits }), actions };
  // Through the registry, so a command that can't run now doesn't; a failure is the session's to say.
  const run = (id: EditMenuId) => void registry.run(EDIT_MENU[id], context).catch(() => undefined);
  return {
    edits,
    actions,
    tab,
    menu: { items: menuItems(registry, context, { version, state: slice }), run },
    unsaved: unsaved > 0,
    lastMs: slice.lastMs,
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
