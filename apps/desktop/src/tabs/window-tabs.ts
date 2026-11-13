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
import { desktopFile, documentOf, rebind } from "../engine/open.ts";

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

/** The main process's tab events, the same names as src-tauri/src/windows.rs's. */
export const ARRIVED_EVENT = "tab://arrived";
export const ACTIVATE_EVENT = "tab://activate";
export const WINDOWS_CHANGED_EVENT = "windows://changed";

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

  constructor(shell: DesktopShell, client: DesktopEngineClient, viewer: Viewer) {
    this.#shell = shell;
    this.#client = client;
    this.#viewer = viewer;
  }

  /** Follows `reader`: going back to the start screen closes the shown tab. Returns the unsubscribe. */
  attach(reader: ReaderController): () => void {
    this.#reader = reader;
    this.#lastPhase = reader.getState().phase;
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
    const existing = doc === undefined ? undefined : this.tabOf(doc);
    this.#aside = undefined;
    if (existing !== undefined) {
      this.#set({ active: existing.id });
      return existing.view;
    }
    const tab: Tab = { id: String(this.#next++), file, view: undefined };
    this.#set({ tabs: [...this.#state.tabs, tab], active: tab.id });
    return undefined;
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
    const doc = shown === undefined ? undefined : documentOf(shown.file)?.doc;
    const reader = this.#reader;
    if (shown === undefined || doc === undefined || reader === undefined) return;
    const view = this.#viewer.viewPosition() ?? null;
    await this.#shell.invoke<string>("move_tab", { doc, to, view });
    const next = this.#neighbour(shown.id);
    this.#remove(shown.id);
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
