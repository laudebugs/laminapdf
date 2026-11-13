// Where a bookmark goes when it moves (O11): from the keyboard (⇧⌘↑ ⇧⌘↓ among its siblings, ⇧⌘←
// out to its parent's level, ⇧⌘→ in under the sibling before it; Ctrl Shift on Windows and
// Linux) and from a drop (before, after or onto a row). Both name the move as the outline ops do:
// a parent and a place among its children.

import type { OutlineTreeItem } from "../reader/outline-tree.js";

/** A place among a parent's children, as `moveOutlineItem` takes it. */
export type SiblingAt = "first" | "last" | { readonly before: string } | { readonly after: string };

/** A bookmark's move: under `parent` (`null`: the top level), at `at`. */
export interface BookmarkMove {
  readonly item: string;
  readonly parent: string | null;
  readonly at: SiblingAt;
}

/** Where an item sits: its parent and its siblings (itself among them). */
interface Place {
  readonly parent: OutlineTreeItem | undefined;
  readonly siblings: readonly OutlineTreeItem[];
  readonly index: number;
}

/** The place of item `id`, or `undefined` when the tree hasn't it. */
export function placeOf(items: readonly OutlineTreeItem[], id: string, parent?: OutlineTreeItem): Place | undefined {
  const index = items.findIndex((i) => i.id === id);
  if (index >= 0) return { parent, siblings: items, index };
  for (const item of items) {
    const found = placeOf(item.children, id, item);
    if (found !== undefined) return found;
  }
  return undefined;
}

/** Whether `id` is `ancestor` or inside it. */
function within(ancestor: OutlineTreeItem, id: string): boolean {
  return ancestor.id === id || ancestor.children.some((c) => within(c, id));
}

/** The keys that move a bookmark: an arrow with Shift and ⌘ (Ctrl off Apple). */
export type MoveKey = "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight";

/** The move a move key makes of item `id`, or `undefined` when it can't go that way. */
export function keyMove(items: readonly OutlineTreeItem[], id: string, key: MoveKey): BookmarkMove | undefined {
  const place = placeOf(items, id);
  if (place === undefined) return undefined;
  const { parent, siblings, index } = place;
  const parentId = parent?.id ?? null;
  const before = siblings[index - 1];
  const after = siblings[index + 1];
  switch (key) {
    case "ArrowUp":
      return before === undefined ? undefined : { item: id, parent: parentId, at: { before: before.id } };
    case "ArrowDown":
      return after === undefined ? undefined : { item: id, parent: parentId, at: { after: after.id } };
    case "ArrowRight":
      return before === undefined ? undefined : { item: id, parent: before.id, at: "last" };
    case "ArrowLeft": {
      if (parent === undefined) return undefined;
      const grand = placeOf(items, parent.id);
      return { item: id, parent: grand?.parent?.id ?? null, at: { after: parent.id } };
    }
  }
}

/** Where a drop puts the dragged items: relative to a row. */
export type DropPosition = "before" | "after" | "on";

/**
 * The move a drop of `dragged` makes, relative to the row `target`: beside it, or onto it as its
 * last child. `undefined` when the drop would put an item inside itself, or onto itself.
 */
export function dropMove(
  items: readonly OutlineTreeItem[],
  dragged: string,
  target: string,
  position: DropPosition,
): BookmarkMove | undefined {
  const moving = placeOf(items, dragged);
  const onto = placeOf(items, target);
  if (moving === undefined || onto === undefined) return undefined;
  const item = moving.siblings[moving.index];
  if (item === undefined || within(item, target)) return undefined;
  if (position === "on") return { item: dragged, parent: target, at: "last" };
  const parent = onto.parent?.id ?? null;
  return { item: dragged, parent, at: position === "before" ? { before: target } : { after: target } };
}
