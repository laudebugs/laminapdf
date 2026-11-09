// A page's text for the text layer, read by Lamina PDF in the worker (ADR 0004's two-parsers rule:
// text geometry comes from Lamina PDF, never from the renderer). Runs cross as small records and
// every glyph's quad in one Float32Array, which each caller gets its own copy of to transfer, so a
// page of thousands of glyphs is three objects, not thousands. The last pages read are kept, since
// a page scrolled back into view asks again.

import type { PageTextData, StructureData, TextRunData } from "../protocol.js";
import { current } from "./document.js";

/** How many pages' text stay cached (a screenful of pages and the margin, at most zoom). */
const CACHED_PAGES = 24;

/** The cache, by document ID and page: another document never reads one's pages, and the oldest go. */
const cache = new Map<string, PageTextData>();

/** Page `index`'s text runs and glyph quads (cached; `signal` stops a read of pending data). */
export async function pageText(index: number, signal: AbortSignal): Promise<PageTextData> {
  const { id, model } = current();
  const key = `${id}:${index}`;
  const cached = cache.get(key);
  if (cached !== undefined) {
    // Most recently used last, so the oldest goes first.
    cache.delete(key);
    cache.set(key, cached);
    return cached;
  }
  signal.throwIfAborted();
  const text = await model.ensure(() => model.page(index).text(), { signal });
  const runs: TextRunData[] = text.runs.map((run) => ({
    text: run.text,
    size: run.size,
    vertical: run.vertical,
    actualText: run.actualText,
    invisible: run.invisible,
    glyphs: run.glyphRange,
    ...(run.mcid === undefined ? {} : { mcid: run.mcid }),
  }));
  const { quads } = text.buffers;
  const data: PageTextData = { runs, quads: quads.slice() };
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
  const { model } = current();
  const tagged = await model.ensure(() => model.isTagged, { signal });
  const tree = await model.ensure(() => model.structureTree(), { signal });
  return tree === undefined ? { tagged } : { tagged, tree };
}
