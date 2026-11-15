// How an edit that inserted, deleted or moved pages moved each page (E2 plan §5): the view keeps
// its state by page index, so it follows the pages from their IDs before and after the edit.

/** Where a page that was at some index before the edit is now. */
interface PageAt {
  /** Its index now; for a deleted page, its nearest surviving page's. */
  readonly page: number;
  /** False for a deleted page, which `page` stands in for. */
  readonly kept: boolean;
}

/** How each page moved between two page lists. */
export interface PageMoves {
  /** Each page now, by index: the index it had before, or `undefined` for a new page. */
  readonly from: readonly (number | undefined)[];
  /** The pages now that are new, in order. */
  readonly added: readonly number[];
  /** An old page's index now, or `undefined` for a deleted page. */
  to(old: number): number | undefined;
  /**
   * Where an old page is now: its own index, or for a deleted one its nearest surviving page's,
   * the next one that stayed, else the one before.
   */
  at(old: number): PageAt;
}

/** How the pages moved from the IDs in `before` to those in `after`. */
export function pageMoves(before: readonly string[], after: readonly string[]): PageMoves {
  const was = new Map(before.map((id, index) => [id, index]));
  const from = after.map((id) => was.get(id));
  const now = new Array<number | undefined>(before.length).fill(undefined);
  const added: number[] = [];
  from.forEach((old, index) => {
    if (old === undefined) added.push(index);
    else now[old] = index;
  });
  // For each old page, the index now of the nearest one that stayed after it, and before it.
  const laterKept = nearest(now, -1);
  const earlierKept = nearest(now, 1);
  return {
    from,
    added,
    to: (old) => now[old],
    at: (old) => {
      const kept = now[old];
      if (kept !== undefined) return { page: kept, kept: true };
      return { page: laterKept[old] ?? earlierKept[old] ?? 0, kept: false };
    },
  };
}

/**
 * For each old page, the index now of the nearest page that stayed, scanning towards it in
 * `direction` (-1: from the end, so the next one after; 1: from the start, the one before).
 */
function nearest(now: readonly (number | undefined)[], direction: 1 | -1): (number | undefined)[] {
  const out = new Array<number | undefined>(now.length).fill(undefined);
  let seen: number | undefined;
  for (let k = 0; k < now.length; k++) {
    const old = direction === 1 ? k : now.length - 1 - k;
    out[old] = seen;
    seen = now[old] ?? seen;
  }
  return out;
}

/** `map`'s entries at their pages' indices after `moves`; a deleted page's go. */
export function movedKeys<T>(map: ReadonlyMap<number, T>, moves: PageMoves): Map<number, T> {
  const next = new Map<number, T>();
  for (const [page, value] of map) {
    const to = moves.to(page);
    if (to !== undefined) next.set(to, value);
  }
  return next;
}

/** Whether `a` and `b` list the same page IDs in the same order. */
export function samePageIds(a: readonly string[], b: readonly string[]): boolean {
  return a.length === b.length && a.every((id, i) => id === b[i]);
}

/** `page`'s index after `moves`, as a list: empty when it was deleted. */
export function keptPage(moves: PageMoves, page: number): number[] {
  const to = moves.to(page);
  return to === undefined ? [] : [to];
}
