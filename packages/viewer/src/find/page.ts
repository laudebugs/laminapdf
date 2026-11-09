// A page's text as find searches it: its runs joined as copy joins them (a new line at a new
// baseline, a space at a gap), with, for every character, the glyphs it came from. A run's text
// is in logical order and its glyphs in drawing order, so a character maps to its glyph when the
// glyphs spell the run forwards, or backwards (right-to-left text), and to the whole run when
// they spell neither (an `/ActualText` replacement, or reordered marks). A match then becomes one
// quad per run it touches, from its first glyph to its last.

import { layoutSpans, type Piece, separator } from "../layers/spans.js";
import type { TextRunData } from "../protocol.js";
import { type FindOptions, type Folded, fold, foldQuery, matches } from "./fold.js";

/** A page's runs and glyphs, as the document Web Worker keeps them. */
export interface PageGlyphs {
  readonly runs: readonly TextRunData[];
  /** 8 numbers per glyph: bottom-left, bottom-right, top-right, top-left, each `x, y`, in page space. */
  readonly quads: Float32Array;
  /** Glyph `i`'s text is `glyphText.slice(glyphOffsets[i], glyphOffsets[i + 1])`. */
  readonly glyphText: string;
  readonly glyphOffsets: Uint32Array;
}

/** A page's text for searching, with the glyphs behind each UTF-16 unit. */
export interface SearchText {
  readonly text: string;
  /** For unit `i`, the glyphs `[lo[i], hi[i])`; an empty range for a separator. */
  readonly lo: Int32Array;
  readonly hi: Int32Array;
  /** The run each glyph belongs to. */
  readonly runOf: Int32Array;
}

/** One match: a quad (8 numbers, as above) per run it touches, in order. */
export type MatchQuads = readonly number[];

/** Glyph `g`'s text. */
function glyphString(page: PageGlyphs, g: number): string {
  return page.glyphText.slice(page.glyphOffsets[g] ?? 0, page.glyphOffsets[g + 1] ?? 0);
}

/** For each UTF-16 unit of `run.text`, the glyph range it came from. */
function mapRun(page: PageGlyphs, run: TextRunData): [number, number][] {
  const [start, end] = run.glyphRange;
  const units: [number, number][] = [];
  const walk = (glyphs: number[]): boolean => {
    units.length = 0;
    for (const g of glyphs) for (let k = 0; k < glyphString(page, g).length; k++) units.push([g, g + 1]);
    return units.length === run.text.length && glyphs.map((g) => glyphString(page, g)).join("") === run.text;
  };
  const forwards = Array.from({ length: end - start }, (_, k) => start + k);
  if (walk(forwards) || walk([...forwards].reverse())) return units;
  return Array.from({ length: run.text.length }, () => [start, end] as [number, number]);
}

/** `page`'s text in reading order, mapped to its glyphs. */
export function searchText(page: PageGlyphs): SearchText {
  const spans = layoutSpans(page.runs, page.quads);
  let text = "";
  const lo: number[] = [];
  const hi: number[] = [];
  const glyphs = Math.max(0, page.glyphOffsets.length - 1);
  const runOf = new Int32Array(glyphs).fill(-1);
  let before: Piece | undefined;
  for (const span of spans) {
    const run = page.runs[span.run];
    if (run === undefined) continue;
    const piece: Piece = { page: 0, text: run.text, span };
    if (before !== undefined) {
      const gap = separator(before, piece);
      for (let k = 0; k < gap.length; k++) {
        lo.push(0);
        hi.push(0);
      }
      text += gap;
    }
    for (const [a, b] of mapRun(page, run)) {
      lo.push(a);
      hi.push(b);
    }
    for (let g = run.glyphRange[0]; g < run.glyphRange[1]; g++) runOf[g] = span.run;
    text += run.text;
    before = piece;
  }
  return { text, lo: Int32Array.from(lo), hi: Int32Array.from(hi), runOf };
}

/** The glyphs `[first, last]` that source units `[start, end)` cover, per run. */
function glyphsByRun(search: SearchText, start: number, end: number): Map<number, [number, number]> {
  const byRun = new Map<number, [number, number]>();
  for (let u = start; u < end; u++) {
    const a = search.lo[u] ?? 0;
    const b = search.hi[u] ?? 0;
    if (b <= a) continue;
    const run = search.runOf[a] ?? -1;
    const seen = byRun.get(run);
    byRun.set(run, seen === undefined ? [a, b - 1] : [Math.min(seen[0], a), Math.max(seen[1], b - 1)]);
  }
  return byRun;
}

/** The quads of source units `[start, end)`: one per run, from its first glyph to its last. */
function quadsOf(page: PageGlyphs, search: SearchText, start: number, end: number): MatchQuads {
  const q = page.quads;
  const at = (i: number) => q[i] ?? 0;
  return [...glyphsByRun(search, start, end).values()].flatMap(([first, last]) => {
    const f = first * 8;
    const l = last * 8;
    // Bottom-left and top-left of the first glyph, bottom-right and top-right of the last.
    return [at(f), at(f + 1), at(l + 2), at(l + 3), at(l + 4), at(l + 5), at(f + 6), at(f + 7)];
  });
}

/** Every match of `query` in `search`'s text, as `[start, end)` of the folded text, with that text. */
export function matchRanges(
  search: SearchText,
  query: string,
  options: FindOptions,
): { readonly folded: Folded; readonly ranges: [number, number][] } {
  const folded = fold(search.text, options);
  return { folded, ranges: matches(folded.text, foldQuery(query, options), options.wholeWords === true) };
}

/** The quads of matches `ranges` of `folded`, the folded text of `search`. */
export function quadsOfMatches(
  page: PageGlyphs,
  search: SearchText,
  folded: Folded,
  ranges: readonly (readonly [number, number])[],
): MatchQuads[] {
  return ranges.map(([a, b]) => {
    const start = folded.from[a] ?? 0;
    const end = (folded.from[b - 1] ?? start) + 1;
    return quadsOf(page, search, start, end);
  });
}

/** Every match of `query` on `page`, each as its quads. */
export function findOnPage(page: PageGlyphs, search: SearchText, query: string, options: FindOptions): MatchQuads[] {
  const { folded, ranges } = matchRanges(search, query, options);
  return quadsOfMatches(page, search, folded, ranges);
}

/** A match in its line: the text before it, the match, and the text after, as the page has them. */
export interface FindSnippet {
  /** Up to about 40 characters, starting at a word, "…" first where cut. */
  readonly before: string;
  readonly match: string;
  /** Up to about 40 characters, ending at a word, "…" last where cut. */
  readonly after: string;
}

/** About how many characters of context a snippet keeps on each side of its match. */
const SNIPPET_CONTEXT = 40;

const SPACES = /\s+/gu;
const isLowSurrogate = (code: number) => code >= 0xdc00 && code <= 0xdfff;

/** `text[from, to)` with white space collapsed to single spaces. */
const collapsed = (text: string, from: number, to: number) => text.slice(from, to).replace(SPACES, " ");

/** Where context starting `SNIPPET_CONTEXT` units before `start` begins: at the next word, or a whole character. */
function contextStart(text: string, start: number): number {
  let at = Math.max(0, start - SNIPPET_CONTEXT);
  if (at === 0) return 0;
  const space = text.slice(at, start).search(/\s/u);
  if (space >= 0) return at + space + 1;
  if (isLowSurrogate(text.charCodeAt(at))) at += 1;
  return at;
}

/** Where context ending `SNIPPET_CONTEXT` units after `end` stops: after the last whole word, or a whole character. */
function contextEnd(text: string, end: number): number {
  let at = Math.min(text.length, end + SNIPPET_CONTEXT);
  if (at === text.length) return at;
  const space = text.slice(end, at).search(/\s\S*$/u);
  if (space >= 0) return end + space;
  if (isLowSurrogate(text.charCodeAt(at))) at -= 1;
  return at;
}

/** The snippet of the match `range` of `folded`, the folded text of `search`. */
export function snippetOf(search: SearchText, folded: Folded, range: readonly [number, number]): FindSnippet {
  const text = search.text;
  const [a, b] = range;
  const start = folded.from[a] ?? 0;
  const end = (folded.from[b - 1] ?? start) + 1;
  const from = contextStart(text, start);
  const to = contextEnd(text, end);
  const before = collapsed(text, from, start).trimStart();
  const after = collapsed(text, end, to).trimEnd();
  return {
    before: from > 0 && before !== "" ? `…${before}` : before,
    match: collapsed(text, start, end),
    after: to < text.length && after !== "" ? `${after}…` : after,
  };
}
