// The page grid's moves (Paper O06): a drop, a keyboard move and Move to page… all name a gap
// between pages, and the selection goes there as one `MovePages` op. A gap is an index into the
// page list as it is before the move: 0 is before the first page, `count` after the last.

/** Where a `MovePages` op puts its pages: wire form, `"start"`, `"end"` or beside a page by ID. */
export type MoveTo = "start" | "end" | { readonly before: string } | { readonly after: string };

/** A move of the pages at `pages` (their IDs, in document order) to `to`. */
export interface PageMove {
  readonly pages: readonly string[];
  readonly to: MoveTo;
  /** The page IDs in their order after the move. */
  readonly order: readonly string[];
  /** The moved pages' indices after the move, in order. */
  readonly moved: readonly number[];
}

/**
 * The move that puts the pages at `selected` (indices into `order`) in gap `gap`, or `undefined`
 * when it would leave every page where it is.
 */
export function moveToGap(order: readonly string[], selected: Iterable<number>, gap: number): PageMove | undefined {
  const picked = [...new Set(selected)].filter((i) => i >= 0 && i < order.length).sort((a, b) => a - b);
  if (picked.length === 0) return undefined;
  const chosen = new Set(picked);
  const at = Math.max(0, Math.min(order.length, Math.trunc(gap)));
  const pages = picked.map((i) => order[i] as string);
  const rest = order.filter((_, i) => !chosen.has(i));
  const before = order.slice(0, at).filter((_, i) => !chosen.has(i)).length;
  const next = [...rest.slice(0, before), ...pages, ...rest.slice(before)];
  if (next.every((id, i) => id === order[i])) return undefined;
  const to: MoveTo = before === 0 ? "start" : { after: rest[before - 1] as string };
  return { pages, to, order: next, moved: pages.map((_, k) => before + k) };
}

/** Where Move to page… puts the pages, as a gap: after or before page `index` (0-based), or at the start or end. */
export type MovePosition = "after" | "before" | "start" | "end";

/** The gap Move to page… names in a list of `count` pages. */
export function gapOf(position: MovePosition, index: number, count: number): number {
  if (position === "start") return 0;
  if (position === "end") return count;
  return position === "after" ? index + 1 : index;
}

/**
 * The gap a keyboard move takes the selection to: `delta` positions on (−1 and 1 one page, minus
 * and plus a row's length one row), from its first page back or its last page on, within the list.
 */
export function stepGap(selected: Iterable<number>, delta: number, count: number): number | undefined {
  const picked = [...selected];
  if (picked.length === 0 || delta === 0) return undefined;
  if (delta < 0) return Math.max(0, Math.min(...picked) + delta);
  return Math.min(count, Math.max(...picked) + 1 + delta);
}

/**
 * The pages at `indices` as a person names them, by their labels: runs of neighbours as ranges
 * ("5–6", "1, 3–5"). `label` gives a page's label from its index.
 */
export function pageRanges(indices: Iterable<number>, label: (index: number) => string): string {
  const sorted = [...new Set(indices)].sort((a, b) => a - b);
  const parts: string[] = [];
  let start = sorted[0];
  for (let k = 0; k < sorted.length; k++) {
    const here = sorted[k] as number;
    const next = sorted[k + 1];
    if (next === here + 1) continue;
    parts.push(start === here ? label(here) : `${label(start as number)}–${label(here)}`);
    start = next;
  }
  return parts.join(", ");
}
