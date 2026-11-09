// A page's text for the text layer, read by Lamina PDF in the worker (ADR 0004's two-parsers rule:
// text geometry comes from Lamina PDF, never from the renderer). Runs cross as small records and
// every glyph's quad in one Float32Array, which each caller gets its own copy of to transfer, so a
// page of thousands of glyphs is three objects, not thousands. The last pages read are kept, since
// a page scrolled back into view asks again.

import type { PageGlyphs } from "../find/page.js";
import type { PageTextData, StructureData, TextRunData } from "../protocol.js";
import { textModel } from "./document.js";
import { CACHED_PAGES, RecentPages } from "./recent-pages.js";

/** The last pages' text. Each keeps its glyphs' text too, for find; only the runs and quads reach the page. */
const cache = new RecentPages<PageGlyphs>(CACHED_PAGES);

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
  const cached = cache.get(index);
  if (cached !== undefined) return cached;
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
  cache.set(index, data);
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

/** Forgets every page's text: the document closed. */
export function forgetText(): void {
  cache.clear();
}
