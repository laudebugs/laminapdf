// Moving through the open document: its outline, named destinations and each page's links, read
// by Lamina PDF in the worker (the extract module, which the text layer loads too), each target a
// page and a view in page space. The outline is read when a reader first asks for it, since it can
// hold thousands of items. The last pages' links are kept, since a page scrolled back into view
// asks again.

import type { OutlineNode, PageLink, Target } from "@laminapdf/sdk";
import { textModel } from "./document.js";
import { CACHED_PAGES, RecentPages } from "./recent-pages.js";

/** The last pages' links. */
const links = new RecentPages<readonly PageLink[]>(CACHED_PAGES);

/** The outline's top-level items, each with its children; empty without one. */
export async function outline(signal: AbortSignal): Promise<OutlineNode[]> {
  const model = await textModel(signal);
  return model.ensure(() => model.outline(), { signal });
}

/** Where the named destination `name` goes, or `undefined` when the document has no such name. */
export async function namedDestination(name: string, signal: AbortSignal): Promise<Target | undefined> {
  const model = await textModel(signal);
  return model.ensure(() => model.namedDestination(name), { signal });
}

/** Page `index`'s links, in annotation order (cached). */
export async function pageLinks(index: number, signal: AbortSignal): Promise<readonly PageLink[]> {
  const cached = links.get(index);
  if (cached !== undefined) return cached;
  const model = await textModel(signal);
  const read = await model.ensure(() => model.pageLinks(index), { signal });
  links.set(index, read);
  return read;
}

/** Forgets `pages`' links (an edit changed them), or every page's (the document closed). */
export function forgetLinks(pages?: Iterable<number>): void {
  links.forget(pages);
}
