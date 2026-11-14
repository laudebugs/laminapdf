// This window's tabs: one per open document, in order, and the one shown. The window
// has one viewer and one reader controller; a tab that isn't shown keeps its document open in the
// window's engine and remembers its view, so showing it again is the reader opening that file,
// which the opener does at once (the document is open) and then puts the view back.
//
// The reader's own paths stay as they are: the Open button, the recent list and the OS's requests
// call `reader.open`, and the opener (`engine/open.ts`) tells the tabs through `showing`, which
// adds the tab or finds it. The document the window stops showing is set aside, never closed; it
// closes only with its tab. A reader that closes its open document closes that tab, and the next
// tab shows, or the start screen when it was the last. One that goes back to its start screen from
// anywhere else (an open that failed, Escape at a password prompt) closes nothing: the tab shown
// before shows again.
import type { ExternalStore, PlatformFile, ReaderController, ReaderState } from "@laminapdf/ui";
import type { Viewer, ViewPosition } from "@laminapdf/viewer";
import type {
  DesktopDocument,
  DesktopEngineClient,
  DesktopShell,
  RestartedPayload,
} from "../engine/desktop-engine-client.ts";
import { desktopFile, documentOf, rebind, restoredFile, restoredToken } from "../engine/open.ts";

/** One tab: its id, the file the reader opens for it, and the view it had when it was last shown. */
export interface Tab {
  readonly id: string;
  readonly file: PlatformFile;
  view: ViewPosition | undefined;
}

/** The tabs as the strip draws them. */
export interface TabsState {
  readonly tabs: readonly Tab[];
  /** The shown tab; none on the start screen or an error page. */
  readonly active: string | undefined;
}

/** Where a moved tab goes, as `move_tab` takes it. */
export type MoveTarget = { readonly to: "newWindow" } | { readonly to: "window"; readonly label: string };

/** What `tab://arrived` carries: a tab moved here, its document open in this window's engine. */
export interface Arrived {
  readonly document: DesktopDocument;
  readonly view: ViewPosition | null;
}

/** A tab of the last session, as `take_session_tabs` gives it: a token, never a path. */
export interface RestoredTab {
  readonly token: string;
  readonly name: string;
  readonly view: ViewPosition | null;
}

/** This window's tabs in the last session, and the shown one's index. */
export interface RestoredWindow {
  readonly tabs: readonly RestoredTab[];
  readonly active: number | null;
}

/** How long the tabs and the view stay still before the main process hears of them. */
const REPORT_AFTER_MS = 500;

/** The main process's tab events, the same names as src-tauri/src/windows.rs's. */
export const ARRIVED_EVENT = "tab://arrived";
export const ACTIVATE_EVENT = "tab://activate";
export const WINDOWS_CHANGED_EVENT = "windows://changed";
/** Whether another window's dragged tab is over this window (src-tauri/src/tab_drag.rs). */
export const DRAG_OVER_EVENT = "tab://drag-over";

/** The window's tabs. */
export class WindowTabs implements ExternalStore<TabsState> {
  readonly #shell: DesktopShell;
  readonly #client: DesktopEngineClient;
  readonly #viewer: Viewer;
  #reader: ReaderController | undefined;
  #state: TabsState = { tabs: [], active: undefined };
  readonly #listeners = new Set<() => void>();
  #next = 1;
  /** Set while the tabs drive the reader themselves (showing a tab, moving one away): its going
      back to the start screen then closes nothing. */
  #driving = false;
  #lastPhase: ReaderState["phase"] = "empty";
  /** The tab the reader last let go of: closed if the reader closed it, else shown again. */
  #aside: string | undefined;
  /** The document whose password the reader asks for: it has no tab until it unlocks. */
  #locked: number | undefined;
  /** The pending report of the tabs to the main process (the session it brings back). */
  #report: ReturnType<typeof setTimeout> | undefined;
  /** Settles once the last session's tabs are here: the OS's opens wait for it, to come after them. */
  readonly #restored: Promise<void>;
  #restoredNow: () => void = () => undefined;
  /** The last drag's `tab_drag_start`: its end waits for it, so Rust never ends a drag it hasn't begun. */
  #dragStart: Promise<void> = Promise.resolve();

  constructor(shell: DesktopShell, client: DesktopEngineClient, viewer: Viewer) {
    this.#shell = shell;
    this.#client = client;
    this.#viewer = viewer;
    this.#restored = new Promise((resolve) => {
      this.#restoredNow = resolve;
    });
  }

  /** Resolves once this window has taken its tabs of the last session, or found it had none. */
  whenRestored(): Promise<void> {
    return this.#restored;
  }

  /** No restored tabs are coming: the window had none in the last session, or asking failed. */
  noRestore(): void {
    this.#restoredNow();
  }

  /** Tells the main process the tabs and views a little after they stop changing. */
  #scheduleReport(): void {
    clearTimeout(this.#report);
    this.#report = setTimeout(() => this.#sendReport(), REPORT_AFTER_MS);
  }

  #sendReport(): void {
    const { tabs, active } = this.#state;
    const reported = tabs.map((tab) => {
      const doc = documentOf(tab.file)?.doc;
      const live = tab.id === active && doc !== undefined && this.#client.current?.doc === doc;
      const view = live ? this.#viewer.viewPosition() : tab.view;
      return { doc: doc ?? null, token: restoredToken(tab.file) ?? null, view: view ?? null };
    });
    const index = tabs.findIndex((tab) => tab.id === active);
    void this.#shell
      .invoke("session_tabs", { tabs: reported, active: index < 0 ? null : index })
      .catch(() => undefined);
  }

  /** Follows `reader`: going back to the start screen closes the shown tab. Returns the unsubscribe. */
  attach(reader: ReaderController): () => void {
    this.#reader = reader;
    this.#lastPhase = reader.getState().phase;
    // The view moving (a page turned, a zoom) is part of the session too.
    const offViewer = this.#viewer.subscribe(() => this.#scheduleReport());
    this.#scheduleReport();
    const offReader = this.#followReader(reader);
    return () => {
      offViewer();
      offReader();
      clearTimeout(this.#report);
    };
  }

  #followReader(reader: ReaderController): () => void {
    return reader.subscribe(() => {
      const state = reader.getState();
      const { phase } = state;
      const was = this.#lastPhase;
      this.#lastPhase = phase;
      this.#leaveLocked(state);
      if (phase !== "empty" || was === "empty" || this.#driving) return;
      if (was === "open") void this.#closedByReader();
      else if (this.#aside !== undefined) void this.activate(this.#aside);
    });
  }

  /**
   * Follows the document whose password the reader asks for. The main process holds it open (a
   * second open of its file would show its tab), so once the reader leaves it without a tab
   * (Escape, another file) it closes.
   */
  #leaveLocked(state: ReaderState): void {
    const asked = state.phase === "needs-password" ? documentOf(state.file)?.doc : undefined;
    const left = this.#locked;
    this.#locked = asked ?? (state.phase === "opening" ? left : undefined);
    if (left === undefined || left === this.#locked || this.tabOf(left) !== undefined) return;
    if (this.#client.current?.doc === left) return;
    void this.#shell.invoke("close_document", { doc: left }).catch(() => undefined);
  }

  getState(): TabsState {
    return this.#state;
  }

  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  #set(state: Partial<TabsState>): void {
    this.#state = { ...this.#state, ...state };
    for (const listener of this.#listeners) listener();
    this.#scheduleReport();
  }

  /**
   * The last session's tabs for this window: they come back unopened, each opening when it first
   * shows, and the one shown then shows now.
   */
  async restore(restored: RestoredWindow): Promise<void> {
    try {
      const tabs = restored.tabs.map(
        (t): Tab => ({ id: String(this.#next++), file: restoredFile(t.token, t.name), view: t.view ?? undefined }),
      );
      if (tabs.length === 0) return;
      this.#set({ tabs: [...this.#state.tabs, ...tabs] });
      const shown = tabs[restored.active ?? 0] ?? tabs[0];
      if (shown !== undefined) await this.activate(shown.id);
    } finally {
      this.#restoredNow();
    }
  }

  /** Whether `file` stands for a tab of the last session that hasn't opened (yet, or at all). */
  isUnopened(file: PlatformFile): boolean {
    return restoredToken(file) !== undefined;
  }

  /** The missing file of tab `id` was found at `document`'s file: the tab shows it from now on. */
  async located(id: string, document: DesktopDocument): Promise<void> {
    const tab = this.#state.tabs.find((t) => t.id === id);
    if (tab === undefined) return;
    rebind(tab.file, document);
    tab.view = undefined;
    await this.#drive(() => this.#reader?.open(tab.file));
  }

  /** The tab whose document is `doc`. */
  tabOf(doc: number): Tab | undefined {
    return this.#state.tabs.find((tab) => documentOf(tab.file)?.doc === doc);
  }

  /** The shown tab. */
  get shown(): Tab | undefined {
    return this.#state.tabs.find((tab) => tab.id === this.#state.active);
  }

  /**
   * The opener is about to show `file`: the shown tab keeps its view, and `file`'s tab (a new one
   * for a new document) becomes the shown one. Returns the view to put back once it shows.
   */
  showing(file: PlatformFile): ViewPosition | undefined {
    this.#keepView();
    const doc = documentOf(file)?.doc;
    // The same file (a restored tab, before its document opens), else the same document.
    const existing =
      this.#state.tabs.find((tab) => tab.file === file) ?? (doc === undefined ? undefined : this.tabOf(doc));
    if (existing !== undefined) {
      this.#set({ active: existing.id });
      return existing.view;
    }
    const tab: Tab = { id: String(this.#next++), file, view: undefined };
    this.#set({ tabs: [...this.#state.tabs, tab], active: tab.id });
    return undefined;
  }

  /**
   * Whether the reader still waits for `file`, which the engine has just opened or unlocked. When
   * it gave up on it meanwhile (Escape, another open), nothing will show it: its document closes
   * unless a tab holds it.
   */
  stillWanted(file: PlatformFile): boolean {
    const state = this.#reader?.getState();
    if (state === undefined || (state.phase === "opening" && state.file === file)) return true;
    const doc = documentOf(file)?.doc;
    if (doc !== undefined && this.tabOf(doc) === undefined) {
      void this.#shell.invoke("close_document", { doc }).catch(() => undefined);
    }
    return false;
  }

  /** The reader lets go of the shown document: its tab keeps the view, for when it shows again. */
  setAside(): void {
    this.#keepView();
    this.#aside = this.#state.active;
    // Nothing shows until the reader shows a document again (or an error, or a password prompt).
    if (this.#state.active !== undefined) this.#set({ active: undefined });
  }

  /** Puts the viewer's view into the shown tab, before the viewer shows another document. */
  #keepView(): void {
    const shown = this.shown;
    if (shown !== undefined && this.#client.current?.doc === documentOf(shown.file)?.doc) {
      shown.view = this.#viewer.viewPosition();
    }
  }

  /** Shows tab `id`. */
  async activate(id: string): Promise<void> {
    const tab = this.#state.tabs.find((t) => t.id === id);
    const reader = this.#reader;
    if (tab === undefined || reader === undefined) return;
    if (id === this.#state.active && reader.getState().phase === "open") return;
    await this.#drive(() => reader.open(tab.file));
  }

  /** Shows the tab after the shown one (`by` 1) or before it (-1), wrapping round. */
  async step(by: 1 | -1): Promise<void> {
    const { tabs, active } = this.#state;
    if (tabs.length < 2) return;
    const index = tabs.findIndex((t) => t.id === active);
    const next = tabs[(index + by + tabs.length) % tabs.length];
    if (next !== undefined) await this.activate(next.id);
  }

  /** Puts the tabs in `ids`' order. */
  reorder(ids: readonly string[]): void {
    const byId = new Map(this.#state.tabs.map((tab) => [tab.id, tab]));
    const tabs = ids.flatMap((id) => byId.get(id) ?? []);
    if (tabs.length === this.#state.tabs.length) this.#set({ tabs });
  }

  /** Closes tab `id` and its document. The shown tab closes through the reader, like its Close. */
  async close(id: string): Promise<void> {
    const unopened = this.#state.tabs.find((t) => t.id === id && this.isUnopened(t.file));
    if (unopened !== undefined) {
      // A tab of the last session whose file never opened: no document, nothing for the reader.
      const next = id === this.#state.active ? this.#neighbour(id) : undefined;
      const wasShown = id === this.#state.active;
      this.#remove(id);
      if (next !== undefined) await this.activate(next);
      else if (wasShown) await this.#drive(() => this.#reader?.close());
      return;
    }
    if (id === this.#state.active) {
      this.#reader?.close();
      return;
    }
    const tab = this.#state.tabs.find((t) => t.id === id);
    if (tab === undefined) return;
    this.#remove(tab.id);
    const doc = documentOf(tab.file)?.doc;
    if (doc !== undefined) await this.#shell.invoke("close_document", { doc });
  }

  /** The reader closed its open document: that tab and its document close. */
  async #closedByReader(): Promise<void> {
    const shown = this.#state.tabs.find((tab) => tab.id === this.#aside);
    this.#aside = undefined;
    if (shown === undefined) return;
    const doc = documentOf(shown.file)?.doc;
    const next = this.#neighbour(shown.id);
    this.#remove(shown.id);
    if (doc !== undefined) {
      await this.#shell.invoke("close_document", { doc }).catch((error: unknown) => {
        console.warn("closing the document failed", error);
      });
    }
    if (next !== undefined) await this.activate(next);
  }

  /** The tab to show when `id` goes: the one after it, else the one before. */
  #neighbour(id: string): string | undefined {
    const { tabs } = this.#state;
    const index = tabs.findIndex((t) => t.id === id);
    return (tabs[index + 1] ?? tabs[index - 1])?.id;
  }

  #remove(id: string): void {
    const tabs = this.#state.tabs.filter((t) => t.id !== id);
    this.#set({ tabs, active: this.#state.active === id ? undefined : this.#state.active });
  }

  /**
   * Moves the shown tab to a new window or another one, with its view. The document keeps its
   * kept password, so it never asks again there. On success the tab leaves this window, and the
   * next tab shows (the start screen after the last); on failure it stays and the error rejects.
   */
  async moveShown(to: MoveTarget): Promise<void> {
    const shown = this.shown;
    if (shown !== undefined)
      await this.#move(shown.id, (doc, view) => this.#shell.invoke<string>("move_tab", { doc, to, view }));
  }

  /** Tab `id` began to be dragged: Rust follows the cursor until the drag ends. */
  dragStarted(id: string): void {
    const doc = this.#tab(id)?.doc;
    if (doc === undefined) return;
    this.#dragStart = this.#shell.invoke<void>("tab_drag_start", { doc }).catch((error: unknown) => {
      // The drag still ends where Rust finds the cursor; only the drop border is missing.
      console.warn("following the dragged tab failed", error);
    });
  }

  /**
   * Tab `id`'s drag ended without a drop on this window's strip (`outside`: the pointer had left
   * this window, as far as the page can tell). Rust moves it to the window under the cursor, or
   * to a new one when the cursor is outside every window, or leaves it here. A move goes as
   * {@link moveShown}'s does; on failure the tab stays and the error rejects.
   */
  async draggedOut(id: string, outside: boolean): Promise<void> {
    await this.#dragStart;
    await this.#move(id, (doc, view) => this.#shell.invoke<string | null>("tab_drag_end", { doc, view, outside }));
  }

  /** Tab `id` and its document, while it has one. */
  #tab(id: string): { tab: Tab; doc: number } | undefined {
    const tab = this.#state.tabs.find((t) => t.id === id);
    const doc = tab === undefined ? undefined : documentOf(tab.file)?.doc;
    return tab === undefined || doc === undefined ? undefined : { tab, doc };
  }

  /**
   * Hands tab `id`'s document and view to `send`, which moves it and answers where it went (`null`:
   * it stayed). A moved tab leaves; when it was the shown one, the next shows, or the start screen.
   */
  async #move(id: string, send: (doc: number, view: ViewPosition | null) => Promise<string | null>): Promise<void> {
    const found = this.#tab(id);
    const reader = this.#reader;
    if (found === undefined || reader === undefined) return;
    const { tab, doc } = found;
    const shown = id === this.#state.active;
    const view = (shown ? this.#viewer.viewPosition() : tab.view) ?? null;
    if ((await send(doc, view)) === null) return;
    const next = this.#neighbour(id);
    this.#remove(id);
    if (!shown) return;
    // The document is the other window's now: the viewer lets go of it without closing it.
    this.#client.forget();
    if (next !== undefined) await this.activate(next);
    else await this.#drive(() => reader.close());
  }

  /** A tab moved here from another window: it shows, at the view it had there. */
  async arrived({ document, view }: Arrived): Promise<void> {
    const reader = this.#reader;
    if (reader === undefined) return;
    const file = desktopFile(document);
    const tab: Tab = { id: String(this.#next++), file, view: view ?? undefined };
    this.#keepView();
    this.#set({ tabs: [...this.#state.tabs, tab] });
    await this.#drive(() => reader.open(file));
  }

  /** The tab of `doc` reopened from its changed file as `document`, while not shown. */
  reloaded(previous: number, document: DesktopDocument): boolean {
    const tab = this.tabOf(previous);
    if (tab === undefined) return false;
    rebind(tab.file, document);
    return true;
  }

  /**
   * The window's engine restarted: a tab not shown whose document didn't reopen goes (its name is
   * returned, for the window to say so), and one that came back locked asks for its password when
   * it next shows. The shown tab is the engine client's to report.
   */
  restarted(payload: RestartedPayload): string[] {
    const lost: string[] = [];
    for (const { doc, state } of payload.docs) {
      const tab = this.tabOf(doc);
      const opened = tab === undefined ? undefined : documentOf(tab.file);
      if (tab === undefined || opened === undefined || tab.id === this.#state.active) continue;
      if (state === "locked") rebind(tab.file, { ...opened, needsPassword: true });
      else if (state !== "open") {
        lost.push(opened.name);
        this.#remove(tab.id);
      }
    }
    return lost;
  }

  /** Runs `act` on the reader without its going back to the start screen closing a tab. */
  async #drive(act: () => void | Promise<void>): Promise<void> {
    this.#driving = true;
    try {
      await act();
    } finally {
      this.#driving = false;
    }
  }
}
