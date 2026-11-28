// Sets of code points as the CSS `unicode-range` writes them (`U+0020-007E,U+00A0`), which
// fonts.lock.json records for each font file: what a font covers, and whether it covers a character.

/** A set of code points: sorted, non-overlapping `[first, last]` pairs. */
export type CodePoints = readonly (readonly [number, number])[];

/** The code points `range` lists; a part that isn't `U+XXXX` or `U+XXXX-YYYY` is skipped. */
export function parseRange(range: string): CodePoints {
  const pairs: [number, number][] = [];
  for (const part of range.split(",")) {
    const match = /^\s*U\+([0-9a-f]{1,6})(?:-([0-9a-f]{1,6}))?\s*$/i.exec(part);
    if (!match?.[1]) continue;
    const first = Number.parseInt(match[1], 16);
    const last = match[2] ? Number.parseInt(match[2], 16) : first;
    if (last >= first) pairs.push([first, last]);
  }
  return merge(pairs);
}

/** `pairs` sorted, with those that touch or overlap joined. */
export function merge(pairs: readonly (readonly [number, number])[]): CodePoints {
  const sorted = pairs.map(([a, b]) => [a, b] as [number, number]).sort((x, y) => x[0] - y[0]);
  const out: [number, number][] = [];
  for (const [first, last] of sorted) {
    const previous = out[out.length - 1];
    if (previous && first <= previous[1] + 1) previous[1] = Math.max(previous[1], last);
    else out.push([first, last]);
  }
  return out;
}

/** Whether `set` holds `codePoint`. */
export function covers(set: CodePoints, codePoint: number): boolean {
  let low = 0;
  let high = set.length - 1;
  while (low <= high) {
    const middle = (low + high) >> 1;
    const [first, last] = set[middle] ?? [0, -1];
    if (codePoint < first) high = middle - 1;
    else if (codePoint > last) low = middle + 1;
    else return true;
  }
  return false;
}
