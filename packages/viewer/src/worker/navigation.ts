// Moving through the open document: its outline, named destinations, each page's links, where it
// asks to open and its article threads, read
// by Lamina PDF in the worker (the extract module, which the text layer loads too), each target a
// page and a view in page space. The outline is read when a reader first asks for it, since it can
// hold thousands of items. The last pages' links are kept, since a page scrolled back into view
// asks again.

import type {
  AddLink,
  ArticleThread,
  ExplicitDestination,
  InitialView,
  NamedDestination,
  OutlineNode,
  OutlineTarget,
  PageLink,
  Target,
  TargetView,
} from "@laminapdf/sdk";
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

/** How the document asks to be shown when it opens: page and zoom, panel, layout, window. */
export async function initialView(signal: AbortSignal): Promise<InitialView> {
  const model = await textModel(signal);
  return model.ensure(() => model.initialView(), { signal });
}

/** The document's article threads, each with its parts in reading order; empty without any. */
export async function threads(signal: AbortSignal): Promise<ArticleThread[]> {
  const model = await textModel(signal);
  return model.ensure(() => model.threads(), { signal });
}

/** Every named destination, sorted by name. */
export async function namedDestinations(signal: AbortSignal): Promise<NamedDestination[]> {
  const model = await textModel(signal);
  return model.ensure(() => model.namedDestinations(), { signal });
}

/** Page `page` shown as `view` (page space), as the destination the outline ops take. */
export async function destinationAt(page: number, view: TargetView, signal: AbortSignal): Promise<ExplicitDestination> {
  const model = await textModel(signal);
  return model.ensure(() => model.destinationAt(page, view), { signal });
}

/** The edit that adds a link on `area` of page `page` (page space), going to `target`. */
export async function linkOnArea(
  page: number,
  area: [number, number, number, number],
  target: OutlineTarget,
  signal: AbortSignal,
): Promise<AddLink> {
  const model = await textModel(signal);
  return model.ensure(() => model.linkOnArea(page, area, target), { signal });
}

/** The edit that adds a link on text of page `page`: `lines`, four numbers per line in page space. */
export async function linkOnText(
  page: number,
  lines: readonly number[],
  target: OutlineTarget,
  signal: AbortSignal,
): Promise<AddLink> {
  const model = await textModel(signal);
  return model.ensure(() => model.linkOnText(page, lines, target), { signal });
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
