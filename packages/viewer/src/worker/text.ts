// A page's text for the text layer, read by Lamina PDF in the worker (ADR 0004's two-parsers rule:
// text geometry comes from Lamina PDF, never from the renderer). Runs cross as small records and
// every glyph's quad in one Float32Array, which each caller gets its own copy of to transfer, so a
// page of thousands of glyphs is three objects, not thousands. The last pages read are kept, since
// a page scrolled back into view asks again.

import type { PageGlyphs } from "../find/page.js";
import type { PageTextData, ReadingBlock, StructureData, TextRunData } from "../protocol.js";
import { current, textModel } from "./document.js";

/** How many pages' text stay cached (a screenful of pages and the margin, at most zoom). */
const CACHED_PAGES = 24;

/**
 * The cache, by document ID and page: another document never reads one's pages, and the oldest
 * go. Each page keeps its glyphs' text too, for find; only the runs and quads reach the page.
 */
const cache = new Map<string, PageGlyphs>();

/** Page `index`'s text runs and glyph quads (cached; `signal` stops a read of pending data). */
export async function pageText(index: number, signal: AbortSignal): Promise<PageTextData> {
  const { runs, quads } = await pageGlyphs(index, signal);
  return { runs, quads };
}

/**
 * Page `index`'s runs, quads and glyph text, from the cache or read now. `keep: false` (find)
 * reads a page without caching it, so a search through every page never pushes out the text of
 * the pages in view.
 */
export async function pageGlyphs(index: number, signal: AbortSignal, keep = true): Promise<PageGlyphs> {
  const { id } = current();
  const key = `${id}:${index}`;
  const cached = cache.get(key);
  if (cached !== undefined) {
    // Most recently used last, so the oldest goes first.
    cache.delete(key);
    cache.set(key, cached);
    return cached;
  }
  signal.throwIfAborted();
  const model = await textModel(signal);
  const text = await model.ensure(() => model.page(index).text(), { signal });
  const runs: TextRunData[] = text.runs.map((run) => ({
    text: run.text,
    size: run.size,
    vertical: run.vertical,
    actualText: run.actualText,
    invisible: run.invisible,
    glyphRange: run.glyphRange,
    ...(run.mcid === undefined ? {} : { mcid: run.mcid }),
  }));
  // The arrays are this worker's own (the SDK copied them out of WebAssembly memory).
  const { quads, glyphText, glyphOffsets } = text.buffers;
  const data: PageGlyphs = { runs, quads, glyphText, glyphOffsets };
  if (!keep) return data;
  cache.set(key, data);
  for (const old of cache.keys()) {
    if (cache.size <= CACHED_PAGES) break;
    cache.delete(old);
  }
  return data;
}

/** One caller's answer: its own copy of the quads, transferred. */
export function replyText(data: PageTextData): [PageTextData, Transferable[]] {
  const copy = { ...data, quads: data.quads.slice() };
  return [copy, [copy.quads.buffer]];
}

/** Whether the document says it is tagged, and its structure tree when it has one. */
export async function structure(signal: AbortSignal): Promise<StructureData> {
  const model = await textModel(signal);
  const tagged = await model.ensure(() => model.isTagged, { signal });
  const tree = await model.ensure(() => model.structureTree(), { signal });
  return tree === undefined ? { tagged } : { tagged, tree };
}

/** The reading-order blocks cached, by document ID and page (a few pages: a selection's). */
const blocks = new Map<string, ReadingBlock[]>();

/**
 * Page `index`'s blocks in reading order (cached). Running headers and footers stay in, as
 * blocks of their own: a selection that takes them in copies them.
 */
export async function readingOrder(index: number, signal: AbortSignal): Promise<ReadingBlock[]> {
  const key = `${current().id}:${index}`;
  const cached = blocks.get(key);
  if (cached !== undefined) return cached;
  const model = await textModel(signal);
  const extraction = await model.ensure(
    () => model.extract({ format: "json", pages: [index], drop: { furniture: false } }),
    { signal },
  );
  const found = extraction.blocks.map((b) => ({ bbox: b.bbox, type: b.type }));
  blocks.set(key, found);
  for (const old of blocks.keys()) {
    if (blocks.size <= CACHED_PAGES) break;
    blocks.delete(old);
  }
  return found;
}

/** Forgets every page's text: the document closed. */
export function forgetText(): void {
  cache.clear();
  blocks.clear();
}
