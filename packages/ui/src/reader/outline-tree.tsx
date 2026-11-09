// The outline tab (R06, UI/UX plan §5.1): the document's outline (bookmarks) as a WAI-ARIA tree,
// on React Aria's Tree: the arrow keys move and open and close items, Home and End go to the ends,
// typing finds an item by its title, Enter goes to it. Each row shows its page label at the far
// end, in the outline row's look (§7.7). The section that holds the reading position is marked
// `aria-current` and tinted, and its ancestors open so it shows. A long outline is virtualised.

import { Icon } from "@laminapdf/icons/react";
import { type ReactNode, useEffect, useMemo, useState } from "react";
import {
  Button as AriaButton,
  Collection,
  type Key,
  ListLayout,
  Tree,
  TreeItem,
  TreeItemContent,
  Virtualizer,
} from "react-aria-components";
import { message } from "../messages.js";

/** Where in the document a place is: a 0-based page and how far down it, 0 (top) to 1. */
export interface OutlinePlace {
  readonly page: number;
  readonly y: number;
}

/** One outline item, as the reader gives it. */
export interface OutlineTreeItem {
  /** Unique in the outline (its path, say "2.0"). */
  readonly id: string;
  readonly title: string;
  /** Its page's label (or number), shown at the far end; absent for an item that goes nowhere. */
  readonly page?: string;
  /** Where it goes, for finding the current section. */
  readonly at?: OutlinePlace;
  /** Whether it opens with its children shown (the file's `/Count`). */
  readonly open?: boolean;
  readonly children: readonly OutlineTreeItem[];
}

/** Props of `OutlineTree`. */
export interface OutlineTreeProps {
  readonly items: readonly OutlineTreeItem[];
  /** The id of the section that holds the reading position (see {@link currentOutlineItem}). */
  readonly current?: string;
  /** An item was chosen (click, tap or Enter). */
  readonly onNavigate: (id: string) => void;
}

/** Above this many items the tree draws only the rows in view. */
const VIRTUAL_FROM = 500;
/** A row's height for the virtualiser, in CSS px: the outline row's target size. */
const ROW_HEIGHT = 36;

/** Every item depth-first (each before its children), with its ancestors' ids. */
function* walk(
  items: readonly OutlineTreeItem[],
  parents: readonly string[] = [],
): Generator<[OutlineTreeItem, readonly string[]]> {
  for (const item of items) {
    yield [item, parents];
    yield* walk(item.children, [...parents, item.id]);
  }
}

/** Whether place `a` comes after place `b` in the document. */
function after(a: OutlinePlace, b: OutlinePlace): boolean {
  return a.page > b.page || (a.page === b.page && a.y > b.y);
}

/**
 * The section that holds `top` (the place at the top of the viewport): the item whose place is
 * the latest at or before it, and of items at the same place the deepest (the last in document
 * order). An item that points back before the chapter being read never wins over it. `undefined`
 * before the first.
 */
export function currentOutlineItem(items: readonly OutlineTreeItem[], top: OutlinePlace): string | undefined {
  let found: { id: string; at: OutlinePlace } | undefined;
  for (const [item] of walk(items)) {
    const at = item.at;
    if (at === undefined || after(at, top)) continue;
    if (found === undefined || !after(found.at, at)) found = { id: item.id, at };
  }
  return found?.id;
}

/** The ids of the items the file opens with their children shown. */
function openIds(all: readonly (readonly [OutlineTreeItem, readonly string[]])[]): Set<Key> {
  return new Set(all.filter(([i]) => i.open).map(([i]) => i.id));
}

/** One row, then its children. */
function Item({ item, current }: { item: OutlineTreeItem; current: string | undefined }): ReactNode {
  const isCurrent = item.id === current;
  return (
    <TreeItem id={item.id} textValue={item.title} className="paper-outline-item">
      <TreeItemContent>
        {({ hasChildItems, isExpanded, level }) => (
          <div
            className="paper-outline-row"
            data-level={Math.min(level, 2)}
            data-current={isCurrent || undefined}
            // Levels past two step in further, by the nested indent each time.
            {...(level > 2 ? { style: { marginInlineStart: `calc(${level - 2} * var(--paper-space-22))` } } : {})}
          >
            {hasChildItems ? (
              <AriaButton slot="chevron" className="paper-outline-toggle" excludeFromTabOrder>
                <Icon name={isExpanded ? "chevron-down" : "chevron-right"} />
              </AriaButton>
            ) : (
              <span className="paper-outline-toggle" aria-hidden="true" />
            )}
            <span className="paper-outline-link" {...(isCurrent ? { "aria-current": "location" } : {})}>
              <span className="paper-outline-title">{item.title}</span>
              {item.page === undefined ? null : <span className="paper-outline-page">{item.page}</span>}
            </span>
          </div>
        )}
      </TreeItemContent>
      <Collection items={item.children} dependencies={[current]}>
        {(child) => <Item item={child} current={current} />}
      </Collection>
    </TreeItem>
  );
}

/** The document outline as a tree, inside the rail's Outline tab. */
export function OutlineTree({ items, current, onNavigate }: OutlineTreeProps): ReactNode {
  const all = useMemo(() => [...walk(items)], [items]);
  const [expanded, setExpanded] = useState<Set<Key>>(() => openIds(all));
  // A new outline starts as the file says.
  useEffect(() => setExpanded(openIds(all)), [all]);
  // The current section shows: its ancestors open.
  useEffect(() => {
    const parents = all.find(([i]) => i.id === current)?.[1] ?? [];
    setExpanded((open) => (parents.every((p) => open.has(p)) ? open : new Set([...open, ...parents])));
  }, [all, current]);
  const tree = (
    <Tree
      aria-label={message("reader.outline")}
      className="paper-outline-tree"
      items={items}
      expandedKeys={expanded}
      onExpandedChange={setExpanded}
      onAction={(key) => onNavigate(String(key))}
      // The rows are cached by item: a new current section must draw them again.
      dependencies={[current]}
    >
      {(item) => <Item item={item} current={current} />}
    </Tree>
  );
  return all.length > VIRTUAL_FROM ? (
    <Virtualizer layout={ListLayout} layoutOptions={{ rowHeight: ROW_HEIGHT }}>
      {tree}
    </Virtualizer>
  ) : (
    tree
  );
}
