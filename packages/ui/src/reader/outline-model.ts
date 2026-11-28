// The outline tab's items from the document's outline: each outline item becomes a tree item with
// the bookmark's ID ("o12.0", which the bookmark edits take), its page's label, its style, the name
// or address it goes to, and its place for the current-section rule (a fraction down the page, from
// the destination's top when it gives one). The targets stay here, by id, for going to an item.
// The web reader and the desktop both build it from what their engines read, so the types are shapes
// their items fit (the SDK's `OutlineNode` and `Target`), not either's own.

import type { BookmarkItem } from "../edit/bookmarks.js";
import type { OutlinePlace } from "./outline-tree.js";

/** The point a page jump puts just under the viewport's top, past the gap left above the page, in CSS px. */
export const TOP_PROBE = 24;

/** A page's facts the outline needs: its label (when the file labels its pages) and its height. */
export interface OutlinePage {
  readonly label?: string;
  readonly height: number;
}

/** A view's turn, clockwise, in degrees. */
export type OutlineRotation = 0 | 90 | 180 | 270;

/** Where an outline item goes: a 0-based page and how it is shown there (the SDK's `Target`). */
export interface OutlineTarget {
  readonly page: number;
  readonly view: { readonly kind: string; readonly y?: number; readonly y0?: number };
}

/** An outline item as the engine reads it (the SDK's `OutlineNode`), with the engine's own target type `T`. */
export interface OutlineNodeData<T extends OutlineTarget = OutlineTarget> {
  readonly id: string;
  readonly title: string;
  readonly open?: boolean;
  readonly bold?: boolean;
  readonly italic?: boolean;
  readonly color?: string;
  readonly destinationName?: string;
  readonly uri?: string;
  readonly target?: T;
  readonly children: readonly OutlineNodeData<T>[];
}

/** The tree's items and each item's target, by id. */
export interface OutlineModel<T extends OutlineTarget = OutlineTarget> {
  readonly items: readonly BookmarkItem[];
  readonly targets: ReadonlyMap<string, T>;
}

/** Page `index`'s label, or its number (1-based) when it has none. */
export const pageLabel = (pages: readonly { readonly label?: string }[], index: number): string =>
  pages[index]?.label ?? String(index + 1);

/** The top of the viewport a target puts its point at, as a fraction down its page. */
export function placeOf(target: OutlineTarget, pages: readonly OutlinePage[]): OutlinePlace {
  const { view } = target;
  const top = view.kind === "at" || view.kind === "fit-width" ? view.y : view.kind === "fit-rect" ? view.y0 : undefined;
  const height = pages[target.page]?.height ?? 0;
  const y = top === undefined || height <= 0 ? 0 : Math.min(1, Math.max(0, top / height));
  return { page: target.page, y };
}

/** `nodes` as the outline tab shows them, with `pages`' labels. */
export function outlineModel<T extends OutlineTarget>(
  nodes: readonly OutlineNodeData<T>[],
  pages: readonly OutlinePage[],
): OutlineModel<T> {
  const targets = new Map<string, T>();
  const convert = (list: readonly OutlineNodeData<T>[]): BookmarkItem[] =>
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
export function downThePage(x: number, y: number, rotation: OutlineRotation): number {
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
