// The bitmap cache: rendered ImageBitmaps by page, scale and tile, within a byte budget
// (width × height × 4 each). Past the budget it evicts the bitmaps farthest from the viewport
// first, and closes every bitmap it evicts or replaces, so GPU memory is given back at once.

/** One cached bitmap: the page it belongs to and its decoded size in bytes. */
export interface CachedBitmap {
  readonly page: number;
  readonly bitmap: ImageBitmap;
  readonly bytes: number;
}

/** Default budget: 256 MB of decoded pixels. */
export const DEFAULT_CACHE_BYTES = 256 * 1024 * 1024;

/** Rendered bitmaps by key, within a byte budget, evicting the farthest from the current page. */
export class BitmapCache {
  readonly budget: number;
  readonly #entries = new Map<string, CachedBitmap>();
  #bytes = 0;

  constructor(budget = DEFAULT_CACHE_BYTES) {
    this.budget = budget;
  }

  get bytes(): number {
    return this.#bytes;
  }

  get size(): number {
    return this.#entries.size;
  }

  /** The bitmap under `key`, if cached (it becomes the most recently used). */
  get(key: string): ImageBitmap | undefined {
    const entry = this.#entries.get(key);
    if (entry === undefined) return undefined;
    // Most recently used last (the tie-breaker among pages at the same distance).
    this.#entries.delete(key);
    this.#entries.set(key, entry);
    return entry.bitmap;
  }

  /**
   * Stores `bitmap` for `page` under `key` (replacing and closing an older one), then evicts by
   * distance from `focus` (the current page) until the cache fits its budget again.
   *
   * @returns whether the cache kept it. A bitmap larger than the whole budget isn't kept, nor
   *   closed: it stays the caller's, to draw and close.
   */
  set(key: string, page: number, bitmap: ImageBitmap, focus: number): boolean {
    this.delete(key);
    const bytes = bitmap.width * bitmap.height * 4;
    if (bytes > this.budget) return false;
    this.#entries.set(key, { page, bitmap, bytes });
    this.#bytes += bytes;
    this.#evict(focus, key);
    return true;
  }

  /** Closes and forgets the bitmap under `key`, if any. */
  delete(key: string): void {
    const entry = this.#entries.get(key);
    if (entry === undefined) return;
    this.#entries.delete(key);
    this.#bytes -= entry.bytes;
    entry.bitmap.close();
  }

  /** Closes every bitmap of `page` whose key `keep` doesn't accept (an old zoom's tiles). */
  prune(page: number, keep: (key: string) => boolean): void {
    for (const [key, entry] of [...this.#entries]) if (entry.page === page && !keep(key)) this.delete(key);
  }

  #evict(focus: number, protect: string): void {
    while (this.#bytes > this.budget) {
      let worst: string | undefined;
      let distance = -1;
      // Insertion order is least recently used first, so among equals the oldest goes.
      for (const [key, entry] of this.#entries) {
        if (key === protect) continue;
        const d = Math.abs(entry.page - focus);
        if (d > distance) {
          distance = d;
          worst = key;
        }
      }
      if (worst === undefined) return;
      this.delete(worst);
    }
  }

  /** Closes everything. */
  clear(): void {
    for (const key of [...this.#entries.keys()]) this.delete(key);
  }
}
