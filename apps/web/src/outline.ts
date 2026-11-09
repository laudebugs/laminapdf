// The outline tab's items from the document's outline: each SDK item becomes a tree item with a
// path id ("2.0"), its page's label, and its place for the current-section rule (a fraction down
// the page, from the destination's top when it gives one). The targets stay here, by id, for
// going to an item.

import type { OutlinePlace, OutlineTreeItem } from "@laminapdf/ui";
import type { OpenedDocument, Viewer, ViewerClient } from "@laminapdf/viewer";

/** A page's facts (size and label), as the viewer opened them. */
export type PageInfo = OpenedDocument["pages"][number];
/** An outline item, as the document worker reads it. */
type OutlineNode = Awaited<ReturnType<ViewerClient["outline"]>>[number];
/** Where an outline item goes. */
type Target = Parameters<Viewer["goToDestination"]>[0];

/** The tree's items and each item's target, by id. */
export interface OutlineModel {
  readonly items: readonly OutlineTreeItem[];
  readonly targets: ReadonlyMap<string, Target>;
}

/** The top of the viewport a target puts its point at, as a fraction down its page. */
function placeOf(target: Target, pages: readonly PageInfo[]): OutlinePlace {
  const { view } = target;
  const top = view.kind === "at" || view.kind === "fit-width" ? view.y : view.kind === "fit-rect" ? view.y0 : undefined;
  const height = pages[target.page]?.height ?? 0;
  const y = top === undefined || height <= 0 ? 0 : Math.min(1, Math.max(0, top / height));
  return { page: target.page, y };
}

/** `nodes` as the outline tab shows them, with `pages`' labels. */
export function outlineModel(nodes: readonly OutlineNode[], pages: readonly PageInfo[]): OutlineModel {
  const targets = new Map<string, Target>();
  const convert = (list: readonly OutlineNode[], prefix: string): OutlineTreeItem[] =>
    list.map((node, i) => {
      const id = prefix === "" ? String(i) : `${prefix}.${i}`;
      const target = node.target;
      if (target !== undefined) targets.set(id, target);
      return {
        id,
        title: node.title,
        open: node.open,
        ...(target === undefined
          ? {}
          : { page: pages[target.page]?.label ?? String(target.page + 1), at: placeOf(target, pages) }),
        children: convert(node.children, id),
      };
    });
  return { items: convert(nodes, ""), targets };
}
