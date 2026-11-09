// Find in document (web-app §4, R05): the viewer's find controller. A query searches the whole
// document from the current page forward and wraps round to the start. The document Web Worker
// searches a few pages per call at `background` priority, and the next call goes out only when
// one returns, so renders run in between and scrolling never waits. Results stream in by page: the
// count grows, the first match after the start shows as soon as it is found, and the highlights
// paint on the pages mounted now and on each page as it mounts. A new query, new options or
// `close()` aborts the search under way, and its late results are dropped by the search's id.

import type { FindPageResult } from "../protocol.js";
import type { FindOptions } from "./fold.js";
import type { FindSnippet } from "./page.js";

/** Pages searched per call: small enough that a render queued meanwhile waits little. */
const PAGES_PER_CALL = 4;

/** Find's state, for the find bar (`useSyncExternalStore`). */
export interface FindState {
  /** The query as typed; empty when find is idle. */
  readonly query: string;
  readonly options: FindOptions;
  /** Matches found so far, in the whole document. */
  readonly total: number;
  /** The current match's position among them, 0-based, in document order; undefined before the first. */
  readonly current: number | undefined;
  /** Pages searched so far, and the document's page count. */
  readonly searched: number;
  readonly pageCount: number;
  /** Every page searched. */
  readonly done: boolean;
  /** Pages with no text to search (scans without an OCR layer). */
  readonly pagesWithoutText: number;
  /** Which search the state belongs to, rising with each new query. */
  readonly search: number;
}

/** One match, for a list of results. */
export interface FindResult {
  /** Its position among every match found so far, 0-based, in document order (as `FindState.current`). */
  readonly ordinal: number;
  /** Its page, 0-based. */
  readonly page: number;
  /** The match in its line. */
  readonly snippet: FindSnippet;
}

/** What the controller needs of the viewer and its client. */
export interface FindHost {
  find(
    query: string,
    options: FindOptions,
    pages: readonly number[],
    signal: AbortSignal,
  ): Promise<readonly FindPageResult[]>;
  pageCount(): number;
  currentPage(): number;
  /** Scrolls match `quads` of `page` into view (centred when it is out of view). */
  reveal(page: number, quads: readonly number[]): void;
  /** These pages' highlights changed. */
  redraw(pages: Iterable<number>): void;
}

/** A match's place: its page and its index among the page's matches. */
interface Place {
  readonly page: number;
  readonly index: number;
}

const IDLE: FindState = {
  query: "",
  options: {},
  total: 0,
  current: undefined,
  searched: 0,
  pageCount: 0,
  done: true,
  pagesWithoutText: 0,
  search: 0,
};

/** Find in one viewer's document. Reach it as `viewer.find`. */
export class FindController {
  readonly #host: FindHost;
  readonly #listeners = new Set<() => void>();
  #state: FindState = IDLE;
  #byPage = new Map<number, readonly (readonly number[])[]>();
  #snippets = new Map<number, readonly FindSnippet[]>();
  /** The last list `results()` made, and the state it was made for. */
  #results: { readonly state: FindState; readonly list: readonly FindResult[] } | undefined;
  #current: Place | undefined;
  #abort: AbortController | undefined;
  #start = 0;

  constructor(host: FindHost) {
    this.#host = host;
  }

  /** Find's state now (a new object after each change). */
  getState = (): FindState => this.#state;

  /** Calls `listener` after every change of state (for `useSyncExternalStore`); returns a function that stops it. */
  subscribe = (listener: () => void): (() => void) => {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  };

  /** Searches for `query` with `options`, replacing any search under way. An empty query clears. */
  find(query: string, options: FindOptions = {}): void {
    this.#stop();
    const search = this.#state.search + 1;
    const cleared = [...this.#byPage.keys()];
    this.#byPage = new Map();
    this.#snippets = new Map();
    this.#current = undefined;
    const pageCount = this.#host.pageCount();
    this.#start = Math.min(Math.max(0, this.#host.currentPage()), Math.max(0, pageCount - 1));
    const searching = query.trim() !== "" && pageCount > 0;
    this.#set({ ...IDLE, query, options, pageCount, done: !searching, search });
    this.#host.redraw(cleared);
    if (searching) void this.#run(query, options, search);
  }

  /** The next match, wrapping to the first; scrolls it into view. */
  next(): void {
    this.#move(1);
  }

  /** The previous match, wrapping to the last; scrolls it into view. */
  previous(): void {
    this.#move(-1);
  }

  /** Makes the match at `ordinal` (as `FindResult.ordinal`) current and scrolls it into view. */
  goTo(ordinal: number): void {
    if (Number.isInteger(ordinal) && ordinal >= 0 && ordinal < this.#state.total) this.#select(ordinal);
  }

  /**
   * Every match found so far, in document order. While a search runs the list grows as pages are
   * searched; the same array comes back until find's state next changes.
   */
  results(): readonly FindResult[] {
    if (this.#results?.state === this.#state) return this.#results.list;
    const list: FindResult[] = [];
    for (const page of [...this.#snippets.keys()].sort((a, b) => a - b)) {
      for (const snippet of this.#snippets.get(page) ?? []) list.push({ ordinal: list.length, page, snippet });
    }
    this.#results = { state: this.#state, list };
    return list;
  }

  /** Ends find: the search stops and every highlight goes. */
  close(): void {
    this.find("");
  }

  /** Page `page`'s matches, and which is current (for the highlight layer). */
  matchesOn(page: number): { matches: readonly (readonly number[])[]; current: number | undefined } {
    return {
      matches: this.#byPage.get(page) ?? [],
      current: this.#current?.page === page ? this.#current.index : undefined,
    };
  }

  /** Stops the search and drops the results: a new document opened. */
  reset(): void {
    this.#stop();
    this.#byPage = new Map();
    this.#snippets = new Map();
    this.#current = undefined;
    this.#set({ ...IDLE, search: this.#state.search + 1 });
  }

  // --- internals ---

  /** Pages from the start page to the end, then from the first back to the start. */
  #order(pageCount: number): number[] {
    return Array.from({ length: pageCount }, (_, k) => (this.#start + k) % pageCount);
  }

  async #run(query: string, options: FindOptions, search: number): Promise<void> {
    const abort = new AbortController();
    this.#abort = abort;
    const order = this.#order(this.#state.pageCount);
    for (let at = 0; at < order.length; at += PAGES_PER_CALL) {
      const pages = order.slice(at, at + PAGES_PER_CALL);
      let results: readonly FindPageResult[];
      try {
        results = await this.#host.find(query, options, pages, abort.signal);
      } catch (error) {
        // A newer search, or close(): this one's results never show.
        if (abort.signal.aborted || this.#state.search !== search) return;
        // The search stops where it failed; what it found stays.
        this.#set({ done: true });
        reportError(error);
        return;
      }
      if (abort.signal.aborted || this.#state.search !== search) return;
      this.#take(results, search);
    }
    if (this.#state.search === search) this.#set({ done: true });
  }

  /** Adds one call's results: the count, the first match to show, the pages to paint. */
  #take(results: readonly FindPageResult[], search: number): void {
    let total = this.#state.total;
    let noText = this.#state.pagesWithoutText;
    const painted: number[] = [];
    for (const { page, matches, snippets, noText: empty } of results) {
      if (empty === true) noText += 1;
      if (matches.length === 0) continue;
      this.#byPage.set(page, matches);
      this.#snippets.set(page, snippets);
      total += matches.length;
      painted.push(page);
    }
    // The first match from the start page on is the current one, shown as soon as it is found.
    let first = false;
    if (this.#current === undefined) {
      const page = results.find((r) => r.matches.length > 0)?.page;
      if (page !== undefined) {
        this.#current = { page, index: 0 };
        first = true;
      }
    }
    this.#set({
      total,
      pagesWithoutText: noText,
      searched: this.#state.searched + results.length,
      current: this.#current === undefined ? undefined : this.#ordinal(this.#current),
      search,
    });
    this.#host.redraw(painted);
    if (first) this.#reveal();
  }

  /** `place`'s position among every match found so far, in document order. */
  #ordinal(place: Place): number {
    let before = 0;
    for (const [page, matches] of this.#byPage) if (page < place.page) before += matches.length;
    return before + place.index;
  }

  /** The match at position `ordinal` in document order. */
  #placeAt(ordinal: number): Place | undefined {
    let left = ordinal;
    for (const page of [...this.#byPage.keys()].sort((a, b) => a - b)) {
      const count = this.#byPage.get(page)?.length ?? 0;
      if (left < count) return { page, index: left };
      left -= count;
    }
    return undefined;
  }

  #move(step: 1 | -1): void {
    const total = this.#state.total;
    if (total === 0) return;
    const now = this.#state.current;
    // A search makes its first match current as soon as it finds one, so `now` is set whenever
    // there is a match; the fallback only guards the types.
    this.#select(now === undefined ? 0 : (now + step + total) % total);
  }

  /** Makes match `next` current, repaints the pages that changed and scrolls it into view. */
  #select(next: number): void {
    const place = this.#placeAt(next);
    if (place === undefined) return;
    const before = this.#current?.page;
    this.#current = place;
    this.#set({ current: next });
    this.#host.redraw(before === undefined || before === place.page ? [place.page] : [before, place.page]);
    this.#reveal();
  }

  #reveal(): void {
    const place = this.#current;
    const quads = place === undefined ? undefined : this.#byPage.get(place.page)?.[place.index];
    if (place !== undefined && quads !== undefined) this.#host.reveal(place.page, quads);
  }

  #stop(): void {
    this.#abort?.abort();
    this.#abort = undefined;
  }

  #set(next: Partial<FindState>): void {
    this.#state = { ...this.#state, ...next };
    for (const listener of this.#listeners) listener();
  }
}
