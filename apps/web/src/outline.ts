// The outline tab's items from the document's outline: each SDK item becomes a tree item with the
// bookmark's ID ("o12.0", which the bookmark edits take), its page's label, its style, the name or
// address it goes to, and its place for the current-section rule (a fraction down the page, from
// the destination's top when it gives one). The targets stay here, by id, for going to an item.

import type { OutlinePlace } from "@laminapdf/ui";
import type { BookmarkItem } from "@laminapdf/ui/editing";
import type { OpenedDocument, Rotation, Viewer, ViewerClient } from "@laminapdf/viewer";

/** The point a page jump puts just under the viewport's top, past the gap left above the page, in CSS px. */
export const TOP_PROBE = 24;

/** A page's facts (size and label), as the viewer opened them. */
export type PageInfo = OpenedDocument["pages"][number];

/** Page `index`'s label, or its number (1-based) when it has none. */
export const pageLabel = (pages: readonly PageInfo[], index: number): string =>
  pages[index]?.label ?? String(index + 1);
/** An outline item, as the document worker reads it. */
type OutlineNode = Awaited<ReturnType<ViewerClient["outline"]>>[number];
/** Where an outline item goes. */
type Target = Parameters<Viewer["goToDestination"]>[0];

/** The tree's items and each item's target, by id. */
export interface OutlineModel {
  readonly items: readonly BookmarkItem[];
  readonly targets: ReadonlyMap<string, Target>;
}

/** The top of the viewport a target puts its point at, as a fraction down its page. */
export function placeOf(target: Target, pages: readonly PageInfo[]): OutlinePlace {
  const { view } = target;
  const top = view.kind === "at" || view.kind === "fit-width" ? view.y : view.kind === "fit-rect" ? view.y0 : undefined;
  const height = pages[target.page]?.height ?? 0;
  const y = top === undefined || height <= 0 ? 0 : Math.min(1, Math.max(0, top / height));
  return { page: target.page, y };
}

/** `nodes` as the outline tab shows them, with `pages`' labels. */
export function outlineModel(nodes: readonly OutlineNode[], pages: readonly PageInfo[]): OutlineModel {
  const targets = new Map<string, Target>();
  const convert = (list: readonly OutlineNode[]): BookmarkItem[] =>
    list.map((node) => {
      const { id, target } = node;
      if (target !== undefined) targets.set(id, target);
      // The node's own fields (title, open state, style, the name or address it goes to), then the tree's.
      return {
        ...node,
        ...(target === undefined ? {} : { page: pageLabel(pages, target.page), at: placeOf(target, pages) }),
        children: convert(node.children),
      };
    });
  return { items: convert(nodes), targets };
}

/**
 * How far down its unturned page a point of the page box on screen is (0 to 1), for a view
 * turned clockwise by `rotation`: outline places are measured on the page as the file draws it.
 */
export function downThePage(x: number, y: number, rotation: Rotation): number {
  switch (rotation) {
    case 90:
      return 1 - x;
    case 180:
      return 1 - y;
    case 270:
      return x;
    default:
      return y;
  }
}
