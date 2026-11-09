// Find in the document Web Worker: a call searches a few pages and returns each page's matches as
// quads in page space. The viewer's find controller asks for the next pages only once a call
// returns, at `background` priority, so renders run between calls and a new query simply stops
// asking. Each page's searchable text stays in a cache of its own (32 MB at most), so typing a
// longer query searches pages already read without reading their text again; a page's glyphs are
// read again only where it has matches.

import type { FindOptions } from "../find/fold.js";
import { fold, foldQuery, matches } from "../find/fold.js";
import { findOnPage, type SearchText, searchText } from "../find/page.js";
import type { FindPageResult } from "../protocol.js";
import { current } from "./document.js";
import { pageGlyphs } from "./text.js";

/** The cache's budget, in bytes of UTF-16 text and glyph maps. */
const SEARCH_CACHE_BYTES = 32 * 1024 * 1024;

const cache = new Map<string, SearchText>();
let cacheBytes = 0;

const bytesOf = (s: SearchText) => s.text.length * 2 + s.lo.byteLength + s.hi.byteLength + s.runOf.byteLength;

/** Page `index`'s searchable text, cached by document and page. */
async function searchable(index: number, signal: AbortSignal): Promise<SearchText> {
  const key = `${current().id}:${index}`;
  const hit = cache.get(key);
  if (hit !== undefined) {
    cache.delete(key);
    cache.set(key, hit);
    return hit;
  }
  const search = searchText(await pageGlyphs(index, signal));
  cache.set(key, search);
  cacheBytes += bytesOf(search);
  for (const [old, value] of cache) {
    if (cacheBytes <= SEARCH_CACHE_BYTES) break;
    cache.delete(old);
    cacheBytes -= bytesOf(value);
  }
  return search;
}

/** The matches of `query` on each of `pages`, in that order. */
export async function find(
  query: string,
  options: FindOptions,
  pages: readonly number[],
  signal: AbortSignal,
): Promise<FindPageResult[]> {
  const wanted = foldQuery(query, options);
  const out: FindPageResult[] = [];
  for (const page of pages) {
    signal.throwIfAborted();
    const search = await searchable(page, signal);
    const empty = search.text.trim() === "";
    // Most pages have no match: count on the folded text first, read glyphs only for matches.
    const found =
      wanted === "" ? 0 : matches(fold(search.text, options).text, wanted, options.wholeWords === true).length;
    const quads = found === 0 ? [] : findOnPage(await pageGlyphs(page, signal), search, query, options);
    out.push({ page, matches: quads.map((q) => [...q]), ...(empty ? { noText: true } : {}) });
  }
  return out;
}

/** Forgets every page's searchable text: the document closed. */
export function forgetSearch(): void {
  cache.clear();
  cacheBytes = 0;
}
