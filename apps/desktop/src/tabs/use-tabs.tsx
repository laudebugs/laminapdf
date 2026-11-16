// The window's tab strip and what drives it from outside the webview: tabs moved here from other
// windows (`tab://arrived`, with the ones that came before the webview was ready handed over by
// `take_pending_tabs`), the last session's tabs (`take_session_tabs`), a second open of a file
// showing its tab (`tab://activate`), the other windows for "Move tab to window ▸", the window's
// title, tabs not shown when the engine restarts, and another window's tab dragged over this one
// (`tab://drag-over`). A tab dragged off the strip goes where Rust finds the cursor when the drag
// ends (`tab_drag_end`). The strip shows while the window holds a document, one tab included, so a
// second tab opens into a strip already there and moves nothing below it (motion plan §5). A tab whose file is missing carries a warning mark, and the strip's menu shows each tab's
// page or "Not found". After a restore, the restore notice says what came back (`useRestoreNotice`).
import {
  type CommandContext,
  type DocumentTab,
  DocumentTabs,
  type DocumentTabsProps,
  type KeyPlatform,
  message,
  type ReaderController,
  RestoreNotice,
  type ToastQueue,
  useStoreState,
} from "@laminapdf/ui";
import { type ReactNode, useEffect, useMemo, useState } from "react";
import {
  type DesktopShell,
  engineError,
  RESTARTED_EVENT,
  type RestartedPayload,
} from "../engine/desktop-engine-client.ts";
import { documentOf } from "../engine/open.ts";
import { stopper } from "../engine/stopper.ts";
import type { PageStore } from "./restore-notice.ts";
import {
  ACTIVATE_EVENT,
  ARRIVED_EVENT,
  type Arrived,
  DRAG_OVER_EVENT,
  type MoveTarget,
  REOPEN_EVENT,
  type RestoredWindow,
  type Tab,
  WINDOWS_CHANGED_EVENT,
  type WindowTabs,
} from "./window-tabs.ts";

const NO_PAGE: PageStore = { getState: () => ({ currentPage: 0 }), subscribe: () => () => undefined };

/** Another window, as `window_list` gives it. */
interface OtherWindow {
  readonly label: string;
  readonly title: string;
}

/** The prefix of the strip's menu entries that move the shown tab to the window labelled after it. */
const MOVE_TO = "move:";
/** The strip's menu entry that moves the shown tab to a new window (no window is labelled "new"). */
const NEW_WINDOW = `${MOVE_TO}new`;

/**
 * The handlers of the tab and window commands (and of the Window menu's Move tab to window ▸),
 * and the shown tab's move, which the strip's menu uses too.
 */
function useTabActions(
  shell: DesktopShell | undefined,
  tabs: WindowTabs | undefined,
  toasts: ToastQueue,
): { move: (to: MoveTarget) => void; actions: NonNullable<CommandContext["actions"]> } {
  const move = useMemo(
    () => (to: MoveTarget) => {
      void tabs?.moveShown(to).catch((error: unknown) => toasts.add({ message: engineError(error).message }));
    },
    [tabs, toasts],
  );
  const actions = useMemo(() => {
    if (tabs === undefined || shell === undefined) return {};
    return {
      "tab.close": () => {
        const shown = tabs.shown;
        if (shown !== undefined) void tabs.close(shown.id);
      },
      "tab.next": () => void tabs.step(1),
      "tab.previous": () => void tabs.step(-1),
      "tab.move-to-new-window": () => move({ to: "newWindow" }),
      "window.new": () => {
        void shell.invoke("new_window").catch((error: unknown) => toasts.add({ message: engineError(error).message }));
      },
      "window.close": () => {
        void shell
          .invoke("close_window")
          .catch((error: unknown) => toasts.add({ message: engineError(error).message }));
      },
      "app.quit": () => void shell.invoke("quit"),
      // The Window menu's "Move tab to window ▸": not a command of its own, as its targets come and go.
      "tab.move-to-window": (label: unknown) => {
        if (typeof label === "string") move({ to: "window", label });
      },
      // The main process asks where and copies the file's bytes (File ▸ Save a copy…).
      "file.save-copy": () => {
        const doc = tabs.shown === undefined ? undefined : documentOf(tabs.shown.file)?.doc;
        if (doc === undefined) return;
        void shell
          .invoke<string | null>("save_copy", { doc })
          .then((name) => {
            if (name !== null) toasts.add({ message: `Saved a copy as ${name}` });
          })
          .catch((error: unknown) => toasts.add({ message: engineError(error).message }));
      },
    };
  }, [tabs, shell, toasts, move]);
  return { move, actions };
}

/** The strip (or nothing) and the handlers of the tab and window commands. */
export function useTabStrip(
  shell: DesktopShell | undefined,
  tabs: WindowTabs | undefined,
  reader: ReaderController,
  toasts: ToastQueue,
  keys: KeyPlatform,
  pages?: PageStore,
  unsaved = false,
): { strip: ReactNode; notice: ReactNode; actions: NonNullable<CommandContext["actions"]> } {
  useTabEvents(shell, tabs, reader, toasts);
  const state = useStoreState(tabs ?? NO_TABS, (s) => s);
  const notice = useRestoreNotice(
    tabs,
    state.tabs.some((tab) => tab.missing === true),
  );
  const others = useOtherWindows(shell, state.tabs.length);
  useWindowTitle(shell, tabs, state.active, unsaved);
  const { move, actions } = useTabActions(shell, tabs, toasts);
  // From the first tab on (motion plan §5): a second tab then moves nothing below the strip.
  const shows = state.tabs.length >= 1;
  if (tabs === undefined || !shows) return { strip: null, notice, actions };
  // The moves act on the shown tab: none while nothing shows.
  const moves =
    state.active === undefined
      ? []
      : [
          { id: NEW_WINDOW, label: message("command.tab.move-to-new-window") },
          ...others.map((w) => ({
            id: `${MOVE_TO}${w.label}`,
            label: message("tabs.moveToWindow", { name: w.title }),
          })),
        ];
  const strip = (
    <TabStrip
      tabs={tabs}
      pages={pages ?? NO_PAGE}
      active={state.active}
      onActivate={(id) => void tabs.choose(id)}
      onClose={(id) => void tabs.close(id)}
      onReorder={(ids) => tabs.reorder(ids)}
      onDragStart={(id) => tabs.dragStarted(id)}
      onDragOut={(id, outside) => {
        void tabs
          .draggedOut(id, outside)
          .catch((error: unknown) => toasts.add({ message: engineError(error).message }));
      }}
      onDragCancel={(id) => {
        void tabs.dragCancelled(id).catch((error: unknown) => console.warn("cancelling a tab's drag", error));
      }}
      moves={moves}
      onMove={(id) => move(id === NEW_WINDOW ? { to: "newWindow" } : { to: "window", label: id.slice(MOVE_TO.length) })}
      platform={keys}
    />
  );
  return { strip, notice, actions };
}

/**
 * The strip over the window's tabs. It alone follows the viewer's page, for the shown tab's entry
 * in its menu, so turning a page redraws the strip and not the window.
 */
function TabStrip({
  tabs,
  pages,
  ...props
}: Omit<DocumentTabsProps, "tabs"> & { tabs: WindowTabs; pages: PageStore }): ReactNode {
  const state = useStoreState(tabs, (s) => s);
  useStoreState(pages, (s) => s.currentPage);
  return <DocumentTabs {...props} tabs={state.tabs.map((tab) => stripTab(tab, tabs.pageOf(tab)))} />;
}

/** A tab as the strip draws it: its name, and its 0-based `page` or its missing file in the menu. */
function stripTab(tab: Tab, page: number | undefined): DocumentTab {
  const title = documentOf(tab.file)?.name ?? tab.file.name;
  if (tab.missing === true) return { id: tab.id, title, missing: true };
  return page === undefined
    ? { id: tab.id, title }
    : { id: tab.id, title, detail: message("tabs.page", { page: page + 1 }) };
}

/**
 * The restore notice under the top bar while it shows, with Show missing file (the first missing
 * tab) while `anyMissing`, when the restore found any: a file found gone later has its own tab's page.
 */
function useRestoreNotice(tabs: WindowTabs | undefined, anyMissing: boolean): ReactNode {
  const summary = useStoreState(tabs?.notice ?? NO_NOTICE, (s) => s);
  if (tabs === undefined || summary === undefined) return undefined;
  return (
    <RestoreNotice
      tabs={summary.tabs}
      missing={summary.missing}
      {...(anyMissing && summary.missing > 0 ? { onShowMissing: () => void tabs.showMissing() } : {})}
      onDismiss={() => tabs.notice.dismiss()}
    />
  );
}

const NO_NOTICE = { getState: () => undefined, subscribe: () => () => undefined } as const;

export const NO_TABS = {
  getState: () => ({ tabs: [], active: undefined }),
  subscribe: () => () => undefined,
} as const;

/** Tabs moved here, files opened again, and background tabs across an engine restart. */
function useTabEvents(
  shell: DesktopShell | undefined,
  tabs: WindowTabs | undefined,
  reader: ReaderController,
  toasts: ToastQueue,
): void {
  useEffect(() => (tabs === undefined ? undefined : tabs.attach(reader)), [tabs, reader]);
  useEffect(() => {
    if (shell === undefined || tabs === undefined) return;
    const listening = async () => {
      // One move at a time, in order, as open requests are.
      let turn = Promise.resolve();
      const off = await Promise.all([
        shell.listen<Arrived>(ARRIVED_EVENT, (arrived) => {
          turn = turn.then(() => tabs.arrived(arrived)).catch((error: unknown) => console.warn("a moved tab", error));
        }),
        shell.listen<number>(ACTIVATE_EVENT, (doc) => {
          const tab = tabs.tabOf(doc);
          if (tab !== undefined) void tabs.activate(tab.id);
        }),
        shell.listen<boolean>(DRAG_OVER_EVENT, (over) => {
          if (over) document.documentElement.dataset.tabDrop = "";
          else delete document.documentElement.dataset.tabDrop;
        }),
        // After an unexpected quit, the tabs the user chose to reopen (S08), once they wait.
        shell.listen<null>(REOPEN_EVENT, () => {
          void shell.invoke<RestoredWindow | null>("take_session_tabs").then(
            (restored) => {
              if (restored !== null) {
                turn = turn
                  .then(() => tabs.restore(restored))
                  .catch((error: unknown) => console.warn("reopening", error));
              }
            },
            (error: unknown) => console.warn("taking the reopened tabs", error),
          );
        }),
        shell.listen<RestartedPayload>(RESTARTED_EVENT, (payload) => {
          for (const name of tabs.restarted(payload)) {
            toasts.add({ message: message("tabs.lost", { name }) });
          }
        }),
      ]);
      await shell.invoke("take_pending_tabs");
      // The last session's tabs for this window, once, when the app started with them (D4d).
      const restored = await shell.invoke<RestoredWindow | null>("take_session_tabs").catch(() => null);
      if (restored === null) tabs.noRestore();
      else turn = turn.then(() => tabs.restore(restored)).catch((error: unknown) => console.warn("restoring", error));
      return () => {
        for (const stop of off) stop();
      };
    };
    // Whatever fails first, the OS's opens waiting on the restore go ahead.
    const started = listening().catch((error: unknown) => {
      tabs.noRestore();
      throw error;
    });
    return stopper(started, "listening for tabs moved to this window");
  }, [shell, tabs, toasts]);
}

/** The other windows, read again whenever one opens, closes or is retitled, or this one's tabs change. */
function useOtherWindows(shell: DesktopShell | undefined, tabCount: number): readonly OtherWindow[] {
  const [others, setOthers] = useState<readonly OtherWindow[]>([]);
  useEffect(() => {
    if (shell === undefined) return;
    const read = () => {
      void shell.invoke<OtherWindow[]>("window_list").then(setOthers, () => undefined);
    };
    read();
    return stopper(shell.listen(WINDOWS_CHANGED_EVENT, read), "listening for other windows");
  }, [shell, tabCount]);
  return others;
}

/**
 * The window's title: the shown tab's document, with a leading "• " while it has unsaved edits
 * (Paper O01, as the web reader's tab), or the app's name alone.
 */
function useWindowTitle(
  shell: DesktopShell | undefined,
  tabs: WindowTabs | undefined,
  active: string | undefined,
  unsaved: boolean,
): void {
  // The shown file's name, read each render: Save as renames the file without changing the tab.
  const shown = active === undefined ? undefined : tabs?.shown;
  const file = shown === undefined ? null : (documentOf(shown.file)?.name ?? shown.file.name);
  useEffect(() => {
    if (shell === undefined || tabs === undefined) return;
    const name = file === null ? null : `${unsaved ? "• " : ""}${file}`;
    // The page's title too, as the window's (src-tauri/src/windows.rs `title`).
    document.title = name === null ? "Lamina PDF" : `${name} — Lamina PDF`;
    void shell.invoke("set_window_title", { name }).catch(() => undefined);
  }, [shell, tabs, file, unsaved]);
}
