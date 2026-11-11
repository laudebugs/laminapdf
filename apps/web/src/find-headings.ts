// The headings of find's Results tab (R05): "Page {label} · {section}", the page's label (or its
// number) and the outline section in effect at the top of that page, the one the Outline tab marks
// when the page is scrolled to the top. Without an outline, or above its first item, "Page {label}".

import { message, type OutlineTreeItem, outlineItemAt } from "@laminapdf/ui";
import { type PageInfo, pageLabel } from "./outline.ts";

/** The heading of page `page`'s results (0-based), from the document's pages and outline. */
export function resultHeadings(
  pages: readonly PageInfo[],
  outline: readonly OutlineTreeItem[],
): (page: number) => string {
  return (page) => {
    const label = pageLabel(pages, page);
    const section = outlineItemAt(outline, { page, y: 0 })?.title;
    return section === undefined || section.trim() === ""
      ? message("find.results.page", { page: label })
      : message("find.results.pageSection", { page: label, section });
  };
}
