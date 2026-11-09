// Back and forward through the places a reader jumped from and to (web-app §6 C6, R07): links,
// the outline, thumbnails, go to page and find each record a jump; scrolling alone never does.
// A jump first notes where the view is now (so Back returns to where the reader had scrolled,
// not only to where they last landed), then adds the place it lands on, cutting off whatever
// Forward held. A jump that lands where the view already is adds nothing. Each entry has an id
// that outlives the oldest entries' removal, so an app can tie its own history (the browser's,
// through the address fragment) to these entries and come back to one by id.

import type { DocumentPoint, Zoom } from "../zoom/zoom.js";

/** A place in the document: the point at the viewport's top-left, and the zoom. */
export interface HistoryPlace {
  readonly point: DocumentPoint;
  readonly zoom: Zoom;
}

/** One entry: a place and the id it keeps while it is in the history. */
export interface HistoryEntry extends HistoryPlace {
  readonly id: number;
}

/** What the history shows: whether Back and Forward go anywhere, and the pages they go to. */
export interface HistoryState {
  /** The entry the view is at. */
  readonly current: HistoryEntry | undefined;
  /** Where Back goes, when it goes anywhere. */
  readonly back: HistoryEntry | undefined;
  /** Where Forward goes, when it goes anywhere. */
  readonly forward: HistoryEntry | undefined;
}

/** What the history reads and moves of its viewer. */
export interface HistoryHost {
  /** Where the view is now; `undefined` without a document. */
  place(): HistoryPlace | undefined;
  /** Puts the view at `place`. */
  show(place: HistoryPlace): void;
}

/** The history's options. */
export interface NavigationHistoryOptions {
  /** The most entries kept; the oldest go first (default 100). */
  readonly max?: number;
}

/** Whether `a` and `b` show the same view: one page, a point within a pixel's worth, one zoom. */
function samePlace(a: HistoryPlace, b: HistoryPlace): boolean {
  const near = (x: number, y: number) => Math.abs(x - y) < 1e-3;
  return a.point.page === b.point.page && near(a.point.x, b.point.x) && near(a.point.y, b.point.y) && a.zoom === b.zoom;
}

/** One viewer's back and forward history. */
export class NavigationHistory {
  readonly #host: HistoryHost;
  readonly #max: number;
  #entries: HistoryEntry[] = [];
  #index = -1;
  #nextId = 1;
  #state: HistoryState = { current: undefined, back: undefined, forward: undefined };
  readonly #listeners = new Set<() => void>();
  readonly #pushes = new Set<(entry: HistoryEntry, replaced: boolean) => void>();

  constructor(host: HistoryHost, options: NavigationHistoryOptions = {}) {
    this.#host = host;
    this.#max = Math.max(2, Math.trunc(options.max ?? 100));
  }

  /** What Back and Forward do now (a new object after each change). */
  getState(): HistoryState {
    return this.#state;
  }

  /** Calls `listener` after every change (for `useSyncExternalStore`). */
  subscribe(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  /**
   * Calls `listener` when an entry is added: by a jump (`replaced` false), or as the first entry
   * of a document (`replaced` true, as `history.replaceState` would). Back, Forward and `goTo`
   * add nothing, so they never call it.
   */
  onEntry(listener: (entry: HistoryEntry, replaced: boolean) => void): () => void {
    this.#pushes.add(listener);
    return () => this.#pushes.delete(listener);
  }

  /** Forgets every entry: a new document starts a history of its own. */
  reset(): void {
    this.#entries = [];
    this.#index = -1;
    this.#changed();
  }

  /**
   * Runs `go` (which moves the view) as a jump: where the view was is kept for Back, and where it
   * lands is the new current entry, with nothing left for Forward. Landing where it was adds
   * nothing.
   */
  jump(go: () => void): void {
    const before = this.#host.place();
    if (before !== undefined) this.#note(before);
    go();
    const after = this.#host.place();
    if (after === undefined) return;
    const current = this.#entries[this.#index];
    if (current !== undefined && samePlace(current, after)) return;
    this.#entries.splice(this.#index + 1);
    const entry = this.#add(after);
    for (const listener of this.#pushes) listener(entry, false);
    this.#changed();
  }

  /** Goes back one entry; `false` when there is none. */
  back(): boolean {
    const entry = this.#entries[this.#index - 1];
    return entry === undefined ? false : this.goTo(entry.id);
  }

  /** Goes forward one entry; `false` when there is none. */
  forward(): boolean {
    const entry = this.#entries[this.#index + 1];
    return entry === undefined ? false : this.goTo(entry.id);
  }

  /**
   * Goes to the entry `id` (Back or Forward, by the browser's history or the app's own): the
   * view moves there and the entries stay. `false` when that entry is gone (pushed out, or from
   * another document).
   */
  goTo(id: number): boolean {
    const index = this.#entries.findIndex((e) => e.id === id);
    const entry = this.#entries[index];
    if (entry === undefined) return false;
    const here = this.#host.place();
    // Where the reader scrolled to since landing is where coming back returns.
    if (here !== undefined) this.#update(here);
    this.#index = index;
    this.#host.show(entry);
    this.#changed();
    return true;
  }

  /** The current entry takes `place`, or the first entry is made from it. */
  #note(place: HistoryPlace): void {
    if (this.#index < 0) {
      const entry = this.#add(place);
      for (const listener of this.#pushes) listener(entry, true);
      return;
    }
    this.#update(place);
  }

  #update(place: HistoryPlace): void {
    const current = this.#entries[this.#index];
    if (current !== undefined) this.#entries[this.#index] = { ...place, id: current.id };
  }

  #add(place: HistoryPlace): HistoryEntry {
    const entry = { ...place, id: this.#nextId++ };
    this.#entries.push(entry);
    if (this.#entries.length > this.#max) this.#entries.splice(0, this.#entries.length - this.#max);
    this.#index = this.#entries.length - 1;
    return entry;
  }

  #changed(): void {
    this.#state = {
      current: this.#entries[this.#index],
      back: this.#entries[this.#index - 1],
      forward: this.#entries[this.#index + 1],
    };
    for (const listener of this.#listeners) listener();
  }
}
