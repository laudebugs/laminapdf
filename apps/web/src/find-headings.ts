// The headings of find's Results tab (R05): "Page {label} · {section}", the page's label (or its
// number) and the outline section in effect at the top of that page, the one the Outline tab marks
// when the page is scrolled to the top. Without an outline, or above its first item, "Page {label}".

import { currentOutlineItem, message, type OutlineTreeItem } from "@laminapdf/ui";
import type { PageInfo } from "./outline.ts";

/** Every item's title by id, the children included. */
function titles(items: readonly OutlineTreeItem[], into = new Map<string, string>()): Map<string, string> {
  for (const item of items) {
    into.set(item.id, item.title);
    titles(item.children, into);
  }
  return into;
}

/** The heading of page `page`'s results (0-based), from the document's pages and outline. */
export function resultHeadings(
  pages: readonly PageInfo[],
  outline: readonly OutlineTreeItem[],
): (page: number) => string {
  const byId = titles(outline);
  return (page) => {
    const label = pages[page]?.label ?? String(page + 1);
    const id = currentOutlineItem(outline, { page, y: 0 });
    const section = id === undefined ? undefined : byId.get(id);
    return section === undefined || section.trim() === ""
      ? message("find.results.page", { page: label })
      : message("find.results.pageSection", { page: label, section });
  };
}
