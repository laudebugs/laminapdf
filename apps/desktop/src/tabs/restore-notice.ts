// Whether a window shows its restore notice (desktop shell S02): what the last session brought
// back into it, until the user dismisses it or first navigates. Restoring a tab puts its view
// back, possibly after the restore itself has finished, so a page change counts as the user's
// only once the viewer has reached the page the restored tab was at, and only within one document:
// another document coming on screen (a located file) settles on its own first page.
import type { ExternalStore } from "@laminapdf/ui";

/** What a window's restore brought back. */
export interface RestoredSummary {
  /** The tabs it restored. */
  readonly tabs: number;
  /** How many of their files couldn't be found. */
  readonly missing: number;
}

/** The viewer's state, as far as the notice reads it. */
interface PageStore extends ExternalStore<{ readonly currentPage: number }> {}

/** The notice's state: the summary while it shows, else undefined. */
export class RestoreNoticeState implements ExternalStore<RestoredSummary | undefined> {
  readonly #pages: PageStore;
  readonly #documentShown: () => number | undefined;
  #summary: RestoredSummary | undefined;
  /** The page the restored tab opens at, until the viewer reaches it; undefined once it has. */
  #arriving: number | undefined;
  /** The page the viewer settled on, and in which document: leaving it is the first navigation. */
  #settled: { page: number; document: number | undefined } | undefined;
  #stopWatching: () => void = () => undefined;
  readonly #listeners = new Set<() => void>();

  /** `documentShown` says which document the viewer shows now, if any. */
  constructor(pages: PageStore, documentShown: () => number | undefined) {
    this.#pages = pages;
    this.#documentShown = documentShown;
  }

  getState(): RestoredSummary | undefined {
    return this.#summary;
  }

  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  /**
   * Shows `summary`. `arriving` is the 0-based page the shown tab is coming back at, which the
   * viewer may still be moving to; undefined when no document shows (its file is missing).
   */
  show(summary: RestoredSummary, arriving: number | undefined): void {
    if (summary.tabs === 0) return;
    this.#summary = summary;
    this.#arriving = arriving;
    this.#settled = undefined;
    this.#stopWatching();
    this.#stopWatching = this.#pages.subscribe(() => this.#pageChanged(this.#pages.getState().currentPage));
    this.#pageChanged(this.#pages.getState().currentPage);
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

  /** One fewer file is missing: it was located, or its tab closed. */
  found(): void {
    const summary = this.#summary;
    if (summary === undefined || summary.missing === 0) return;
    this.#summary = { ...summary, missing: summary.missing - 1 };
    this.#notify();
  }

  #pageChanged(page: number): void {
    if (this.#arriving !== undefined) {
      if (page !== this.#arriving) return;
      this.#arriving = undefined;
    }
    const document = this.#documentShown();
    if (this.#settled === undefined || this.#settled.document !== document) this.#settled = { page, document };
    else if (page !== this.#settled.page) this.dismiss();
  }

  #notify(): void {
    for (const listener of this.#listeners) listener();
  }
}
