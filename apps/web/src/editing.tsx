// The edit session in the reader (Paper O01): the view's session (`viewer.edits`) in the commands'
// context, with the shell's handlers for Show history and turning the current page; the More
// menu's edit items; the History tab once an edit was made; the browser tab's "•" while there are
// unsaved changes, and the browser's own prompt when the tab closes (`beforeunload`). What waits
// until a document is open (the commands, toasts, the close confirmation, the tools) is in
// editing-runtime.tsx, which loads then, with the History tab's panel, so none of it is in the
// startup script (#380).

import {
  type CommandContext,
  type CommandRegistry,
  type EditControls,
  type GatedEntry,
  type KeyPlatform,
  type RailTab,
  type ShellLayout,
  sessionStarted,
  type ToastQueue,
  useStoreState,
} from "@laminapdf/ui";
import type { Viewer } from "@laminapdf/viewer";
import { type ReactNode, useEffect, useMemo, useState, useSyncExternalStore } from "react";
import type { EditMenuId } from "./edit-menu.ts";
import type { Engine } from "./session.ts";

/** The History tab's id in the rail. */
export const HISTORY_TAB = "history";

/** The runtime and the History tab's panel load once a document is open (later.ts). */
/** What later.ts exports. */
export type Later = typeof import("./later.ts");

export type { EditMenuId } from "./edit-menu.ts";

/**
 * later.ts once a document is open (undefined until it has loaded): one import, read directly,
 * which costs the startup script less than a React.lazy wrapper per part. A chunk that can't load
 * (a lost connection) is reported, and the parts it holds stay away.
 */
export function useLater(open: boolean): Later | undefined {
  const [later, setLater] = useState<Later | undefined>(undefined);
  useEffect(() => {
    if (open && later === undefined) void import("./later.ts").then(setLater, reportError);
  }, [open, later]);
  return later;
}

/** A store's subscribe with nothing to hear, for the session while no document is open. */
const NOTHING_TO_HEAR = () => () => undefined;

/** The delegated edit commands' handlers: Show history and turning the current page. */
export type EditActions = Readonly<
  Partial<Record<"edit.history" | "pages.rotate-right" | "pages.rotate-left", () => void>>
>;

/** No handlers, before the edit commands load. */
const NO_ACTIONS: EditActions = {};

/** The More menu before the edit commands load: no edit items. */
const NO_MENU: ReturnType<Editing["menu"]> = { items: [], run: () => undefined };

/** What the reader gets from `useEditing`. */
export interface Editing {
  /** The open document's session, for the commands, the top bar and the History tab. */
  readonly edits: EditControls | undefined;
  /** Handlers of the delegated edit commands. */
  readonly actions: EditActions;
  /** later.ts, once a document is open and it has loaded. */
  readonly later: Later | undefined;
  /** The top bar's save state, Undo and Redo, while a document is open. */
  readonly topBar: ReactNode;
  /** The History tab, once the document has an edit session. */
  readonly tab: RailTab | undefined;
  /** The More menu's edit items and what each does, in the commands' context. */
  readonly menu: (context: CommandContext) => {
    readonly items: GatedEntry<EditMenuId>[];
    readonly run: (id: EditMenuId) => void;
  };
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
}): Editing {
  const { engine, viewer, toasts } = options;
  const { layout, registry } = options.shell;
  const pages = useStoreState(engine, (s) => s.pages);
  const edits = viewer?.edits;
  const later = useLater(viewer !== undefined);
  // Nothing to read without a document: the slice is undefined then.
  const slice = useSyncExternalStore(edits?.subscribe ?? NOTHING_TO_HEAR, () => edits?.getState());
  // The menu follows the registry too: the edit commands register after the first paint.
  const version = useSyncExternalStore(
    (listener) => registry.subscribe(listener),
    () => registry.version,
  );
  const started = slice !== undefined && sessionStarted(slice);
  const unsaved = slice?.history.unsaved ?? 0;
  const openedAt = useStoreState(engine, (s) => s.openedAt);
  const { fileName } = options;
  // With the commands they run, which register once later.ts has loaded.
  const actions = useMemo(
    () => (later === undefined || viewer === undefined ? NO_ACTIONS : later.editActions(engine, layout, viewer)),
    [engine, layout, viewer, later],
  );
  const tab = useMemo<RailTab | undefined>(
    () =>
      edits === undefined || !started || later === undefined ? undefined : later.historyTab(edits, pages, openedAt),
    [edits, started, pages, openedAt, later],
  );
  return {
    edits,
    actions,
    tab,
    menu: (context) => (later === undefined ? NO_MENU : later.editMenu(registry, context, { version, state: slice })),
    unsaved: unsaved > 0,
    lastMs: slice?.lastMs,
    later,
    topBar:
      edits === undefined || later === undefined ? undefined : <later.EditArea edits={edits} keys={options.keys} />,
    runtime: (registry) =>
      viewer === undefined || fileName === undefined || later === undefined ? null : (
        <later.EditingRuntime engine={engine} viewer={viewer} registry={registry} toasts={toasts} fileName={fileName} />
      ),
  };
}
