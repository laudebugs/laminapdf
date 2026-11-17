// The keys that move through a list one row at a time (WAI-ARIA's listbox and grid patterns): the
// arrows, Home, End and the page keys, for every list that keeps its own roving focus.

/** The row the arrow, Home, End and page keys move the focused row `focus` of `total` to. */
export function keyTarget(key: string, focus: number, total: number): number | undefined {
  const keys: Record<string, number | undefined> = {
    ArrowDown: focus + 1,
    ArrowUp: focus - 1,
    Home: 0,
    End: total - 1,
    PageDown: focus + 10,
    PageUp: focus - 10,
  };
  const to = keys[key];
  return to === undefined ? undefined : Math.max(0, Math.min(total - 1, to));
}
