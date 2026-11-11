// Find in the document Web Worker: a call searches a few pages and returns each page's matches as
// quads in page space. The viewer's find controller asks for the next pages only once a call
// returns, at `background` priority, so renders run between calls and a new query simply stops
// asking. Each page's searchable text stays in a cache of its own (32 MB at most), so typing a
// longer query searches pages already read without reading their text again; a page's glyphs are
// read again only where it has matches.

import type { FindOptions } from "../find/fold.js";
import { matchRanges, quadsOfMatches, type SearchText, searchText, snippetOf } from "../find/page.js";
import type { FindPageResult } from "../protocol.js";
import { RecentPages } from "./recent-pages.js";
import { pageGlyphs } from "./text.js";

/** The cache's budget, in bytes of UTF-16 text and glyph maps. */
const SEARCH_CACHE_BYTES = 32 * 1024 * 1024;

const bytesOf = (s: SearchText) => s.text.length * 2 + s.lo.byteLength + s.hi.byteLength + s.runOf.byteLength;

const cache = new RecentPages<SearchText>(SEARCH_CACHE_BYTES, bytesOf);

/** Page `index`'s searchable text, cached by document and page. */
async function searchable(index: number, signal: AbortSignal): Promise<SearchText> {
  const hit = cache.get(index);
  if (hit !== undefined) return hit;
  const search = searchText(await pageGlyphs(index, signal, false));
  cache.set(index, search);
  return search;
}

/** The matches of `query` on each of `pages`, in that order. */
export async function find(
  query: string,
  options: FindOptions,
  pages: readonly number[],
  signal: AbortSignal,
): Promise<FindPageResult[]> {
  const out: FindPageResult[] = [];
  for (const page of pages) {
    signal.throwIfAborted();
    const search = await searchable(page, signal);
    const empty = search.text.trim() === "";
    // Most pages have no match: the glyphs are read again only for a page that has one.
    const { folded, ranges } = matchRanges(search, query, options);
    const quads =
      ranges.length === 0 ? [] : quadsOfMatches(await pageGlyphs(page, signal, false), search, folded, ranges);
    out.push({
      page,
      matches: quads.map((q) => [...q]),
      snippets: ranges.map((r) => snippetOf(search, folded, r)),
      ...(empty ? { noText: true } : {}),
    });
  }
  return out;
}

/** Forgets `pages`' searchable text (an edit changed them), or every page's (the document closed). */
export function forgetSearch(pages?: Iterable<number>): void {
  cache.forget(pages);
}
