// The reader's back and forward history in the browser's (R07): every jump the viewer records
// (a link, the outline, a thumbnail, go to page, find) becomes a browser history entry whose
// fragment names the page it landed on, built with `buildFragment` from the fragment there now, so
// `url` and every other key stay as they were and nothing reaches the server. The entry's state
// holds the viewer's entry id, so the browser's Back and Forward (and Alt+← or ⌘[, which are the
// browser's own on the web) return to the exact place: page, scroll and zoom. The reader's own
// Back and Forward buttons step the browser's history, so both stay one history.
//
// The first jump in a document marks the entry the reader is on (replaceState), so Back from the
// first jump lands there. An id is only good for the viewer that made it: a new view (a backend
// switch, a reload) gets another token, and an entry from an older one goes by its fragment's
// page instead.

import type { Viewer } from "@laminapdf/viewer";
import type { Engine } from "../session.ts";
import { buildFragment, parseFragment } from "./url-fragment.ts";

/** What the reader keeps in a browser history entry's state. */
interface HistoryMark {
  /** The view that made the entry. */
  readonly view: string;
  /** Its entry id in that view's history. */
  readonly id: number;
}

const KEY = "laminaHistory";

/** The reader's mark in a browser history state, when it has one. */
function markOf(state: unknown): HistoryMark | undefined {
  if (typeof state !== "object" || state === null) return undefined;
  const mark = (state as Record<string, unknown>)[KEY];
  if (typeof mark !== "object" || mark === null) return undefined;
  const { view, id } = mark as Record<string, unknown>;
  return typeof view === "string" && typeof id === "number" ? { view, id } : undefined;
}

/** The address of page `page` (0-based) of the document showing: the fragment now, with `page` set. */
export function pageAddress(location: Pick<Location, "pathname" | "search" | "hash">, page: number): string {
  const fragment = buildFragment(parseFragment(location.hash), { page: String(page + 1), nameddest: undefined });
  return `${location.pathname}${location.search}#${fragment}`;
}

/** Ties a viewer's history to `win`'s; `restore` answers a `popstate` or `hashchange`. */
export interface BrowserHistory {
  /**
   * Goes to the place the browser's current entry marks, if this view made it: `true` when the
   * view is there (moved, or already), so the fragment needs no other reading.
   */
  restore(): boolean;
  /** Stops following. */
  stop(): void;
}

/** Writes each entry `viewer`'s history adds into `win`'s, marked with a token for this view. */
function mark(
  win: Window,
  viewer: Viewer,
): { readonly viewer: Viewer; readonly token: string; readonly stop: () => void } {
  const token = win.crypto.randomUUID();
  const stop = viewer.history.onEntry((entry, replaced) => {
    const marked = { [KEY]: { view: token, id: entry.id } satisfies HistoryMark };
    if (replaced) {
      const state = typeof win.history.state === "object" && win.history.state !== null ? win.history.state : {};
      win.history.replaceState({ ...state, ...marked }, "");
    } else {
      win.history.pushState(marked, "", pageAddress(win.location, entry.point.page));
    }
  });
  return { viewer, token, stop };
}

/** Follows `engine`'s view, whichever it is: its jumps become entries in `win`'s history. */
export function followHistory(win: Window, engine: Engine): BrowserHistory {
  let view: { readonly viewer: Viewer; readonly token: string; readonly stop: () => void } | undefined;
  const hook = (viewer: Viewer | undefined) => {
    if (viewer === view?.viewer) return;
    view?.stop();
    view = viewer === undefined ? undefined : mark(win, viewer);
  };
  hook(engine.getState().viewer);
  const unsubscribe = engine.subscribe(() => hook(engine.getState().viewer));
  return {
    restore() {
      const mark = markOf(win.history.state);
      if (view === undefined || mark === undefined || mark.view !== view.token) return false;
      const { history } = view.viewer;
      return history.getState().current?.id === mark.id || history.goTo(mark.id);
    },
    stop() {
      unsubscribe();
      view?.stop();
    },
  };
}
