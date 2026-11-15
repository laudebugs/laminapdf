// Whether a window shows its restore notice (desktop shell S02), which says what the last session
// brought back into it. It shows until the user dismisses it or first navigates. Restoring a tab
// moves the viewer to the place it was at, possibly after the restore itself has finished, and the
// viewer's page (the one in the middle of the window) needn't be the page the session recorded.
// So page changes only say where the view settles for a moment after the restore, or after
// another document (a located file) comes on screen. After that, leaving the settled page is the
// first navigation.
//
// The notice says what the restore found, once: locating a missing file doesn't change it, so a
// screen reader isn't told again.
import type { ExternalStore } from "@laminapdf/ui";

/** What a window's restore brought back. */
export interface RestoredSummary {
  /** The tabs it restored. */
  readonly tabs: number;
  /** How many of their files couldn't be found. */
  readonly missing: number;
}

/** The viewer's page, as the notice and the tab strip read it. */
export type PageStore = ExternalStore<{ readonly currentPage: number }>;

/** How long after a restore, or another document coming on screen, the view may still be settling. */
export const SETTLE_MS = 1500;

/** The notice's state: the summary while it shows, else undefined. */
export class RestoreNoticeState implements ExternalStore<RestoredSummary | undefined> {
  readonly #pages: PageStore;
  readonly #documentShown: () => number | undefined;
  readonly #now: () => number;
  #summary: RestoredSummary | undefined;
  /** Where the view settled, in which document, and until when it may still be settling. */
  #settled: { page: number; document: number | undefined; settlingUntil: number } | undefined;
  #stopWatching: () => void = () => undefined;
  readonly #listeners = new Set<() => void>();

  /**
   * `documentShown` says which document the viewer shows now, if any; `now` is the clock, in ms.
   */
  constructor(pages: PageStore, documentShown: () => number | undefined, now: () => number = () => performance.now()) {
    this.#pages = pages;
    this.#documentShown = documentShown;
    this.#now = now;
  }

  getState(): RestoredSummary | undefined {
    return this.#summary;
  }

  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  /** Shows `summary`, the view now settling where the restore puts it. */
  show(summary: RestoredSummary): void {
    if (summary.tabs === 0) return;
    this.#summary = summary;
    this.#settle(this.#pages.getState().currentPage);
    this.#stopWatching();
    this.#stopWatching = this.#pages.subscribe(() => this.#pageChanged(this.#pages.getState().currentPage));
    this.#notify();
  }

  /** Takes the notice away: Dismiss, or the user went somewhere. */
  dismiss(): void {
    if (this.#summary === undefined) return;
    this.#summary = undefined;
    this.#stopWatching();
    this.#stopWatching = () => undefined;
    this.#notify();
  }

  #settle(page: number): void {
    this.#settled = { page, document: this.#documentShown(), settlingUntil: this.#now() + SETTLE_MS };
  }

  #pageChanged(page: number): void {
    const settled = this.#settled;
    if (settled === undefined || settled.document !== this.#documentShown()) this.#settle(page);
    else if (this.#now() < settled.settlingUntil) settled.page = page;
    else if (page !== settled.page) this.dismiss();
  }

  #notify(): void {
    for (const listener of this.#listeners) listener();
  }
}
