// What the document Web Worker keeps of the pages it read last: a page's text, its searchable
// text, its links. Entries are keyed by document and page, so another document never reads one's
// pages; reading one makes it the newest, and the oldest go first once the entries weigh more than
// the budget (a count of pages, unless `weigh` gives each its size).

import { current } from "./document.js";

/** A cache of the pages read last, of the open document and those before it. */
export class RecentPages<T> {
  readonly #entries = new Map<string, T>();
  readonly #budget: number;
  readonly #weigh: (value: T) => number;
  #weight = 0;

  constructor(budget: number, weigh: (value: T) => number = () => 1) {
    this.#budget = budget;
    this.#weigh = weigh;
  }

  /** Page `index` of the open document, now the newest; `undefined` when it isn't kept. */
  get(index: number): T | undefined {
    const key = this.#key(index);
    const value = this.#entries.get(key);
    if (value !== undefined) {
      this.#entries.delete(key);
      this.#entries.set(key, value);
    }
    return value;
  }

  /** Keeps `value` for page `index` of the open document, letting the oldest go over the budget. */
  set(index: number, value: T): void {
    const key = this.#key(index);
    const old = this.#entries.get(key);
    if (old !== undefined) this.#weight -= this.#weigh(old);
    this.#entries.delete(key);
    this.#entries.set(key, value);
    this.#weight += this.#weigh(value);
    for (const [oldest, kept] of this.#entries) {
      if (this.#weight <= this.#budget) break;
      this.#entries.delete(oldest);
      this.#weight -= this.#weigh(kept);
    }
  }

  /** Forgets every page: the document closed. */
  clear(): void {
    this.#entries.clear();
    this.#weight = 0;
  }

  #key(index: number): string {
    return `${current().id}:${index}`;
  }
}
