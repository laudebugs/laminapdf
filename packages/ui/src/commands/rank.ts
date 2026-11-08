// The palette's ranking: how well a query matches a command's title or keywords, and the parts of
// the title it matched (drawn in 600). A title that starts with the query beats one with a word
// that does, which beats the query anywhere inside, which beats its letters in order. Recent
// commands come first among equals. Small and dependency-free, so the palette view can change
// without the ranking changing (U1b's risks).

/** How a query matched one text: higher is better, and the matched ranges [start, end). */
export interface TextMatch {
  readonly score: number;
  readonly ranges: readonly (readonly [number, number])[];
}

const PREFIX = 100;
const WORD_START = 80;
const INSIDE = 60;
const LETTERS = 20;

/** Whether `index` in `text` starts a word. */
const startsWord = (text: string, index: number) => index === 0 || !/[\p{L}\p{N}]/u.test(text.charAt(index - 1));

/**
 * The query's letters in `text`, in order, spaces skipped: a weaker match, which contiguous runs
 * and word starts make stronger. Both are lower-cased already.
 */
function matchLetters(q: string, t: string): TextMatch | undefined {
  const ranges: [number, number][] = [];
  let from = 0;
  let bonus = 0;
  for (const ch of q.replace(/\s+/g, "")) {
    const found = t.indexOf(ch, from);
    if (found < 0) return undefined;
    const last = ranges.at(-1);
    if (last !== undefined && last[1] === found) {
      last[1] = found + 1;
      bonus += 1;
    } else {
      ranges.push([found, found + 1]);
      if (startsWord(t, found)) bonus += 2;
    }
    from = found + 1;
  }
  return { score: LETTERS + Math.min(bonus, LETTERS - 1), ranges };
}

/** How `query` matches `text`, without regard to case, or undefined when it doesn't. */
export function matchText(query: string, text: string): TextMatch | undefined {
  const q = query.trim().toLocaleLowerCase();
  const t = text.toLocaleLowerCase();
  if (q === "") return { score: 0, ranges: [] };
  const at = t.indexOf(q);
  if (at >= 0) {
    const score = at === 0 ? PREFIX : startsWord(t, at) ? WORD_START : INSIDE;
    return { score, ranges: [[at, at + q.length]] };
  }
  return matchLetters(q, t);
}

/** What the ranking reads of a command. */
export interface Rankable {
  readonly id: string;
  readonly title: string;
  readonly keywords?: readonly string[];
}

/** A ranked command, with the title ranges the query matched. */
export interface Ranked<Item extends Rankable> {
  readonly item: Item;
  readonly score: number;
  readonly ranges: readonly (readonly [number, number])[];
}

/**
 * `items` that `query` matches, best first: by score, then most recent (`recent`, newest first),
 * then in their given order. A keyword match counts a little less than the same title match and
 * highlights nothing.
 */
export function rankCommands<Item extends Rankable>(
  query: string,
  items: readonly Item[],
  recent: readonly string[] = [],
): Ranked<Item>[] {
  const ranked: (Ranked<Item> & { order: number })[] = [];
  items.forEach((item, order) => {
    const title = matchText(query, item.title);
    const keyword = Math.max(-1, ...(item.keywords ?? []).map((k) => matchText(query, k)?.score ?? -1));
    const keywordScore = keyword < 0 ? -1 : keyword - 5;
    if (title === undefined && keywordScore < 0) return;
    const best = title !== undefined && title.score >= keywordScore;
    ranked.push({ item, score: best ? title.score : keywordScore, ranges: best ? title.ranges : [], order });
  });
  const recency = (id: string) => {
    const index = recent.indexOf(id);
    return index < 0 ? recent.length : index;
  };
  return ranked
    .sort((a, b) => b.score - a.score || recency(a.item.id) - recency(b.item.id) || a.order - b.order)
    .map(({ order: _order, ...rest }) => rest);
}

/** The storage the recent list lives in: `localStorage`, or nothing where reading it throws. */
function defaultStorage(): Pick<Storage, "getItem" | "setItem"> | undefined {
  try {
    return globalThis.localStorage ?? undefined;
  } catch {
    return undefined;
  }
}

/**
 * The commands run from the palette, newest first, kept on this device only. Every read and write
 * is guarded: a private window, blocked storage or a full quota leaves the list in memory.
 */
export class RecentCommands {
  readonly #storage: Pick<Storage, "getItem" | "setItem"> | undefined;
  readonly #key: string;
  readonly #max: number;
  #ids: string[];

  constructor(options: { storage?: Pick<Storage, "getItem" | "setItem">; key?: string; max?: number } = {}) {
    this.#storage = options.storage ?? defaultStorage();
    this.#key = options.key ?? "lamina.palette.recent";
    this.#max = options.max ?? 5;
    this.#ids = this.#read();
  }

  /** The recent command ids, newest first. */
  list(): readonly string[] {
    return this.#ids;
  }

  /** Puts `id` first. */
  add(id: string): void {
    this.#ids = [id, ...this.#ids.filter((other) => other !== id)].slice(0, this.#max);
    try {
      this.#storage?.setItem(this.#key, JSON.stringify(this.#ids));
    } catch {
      // Kept in memory for this session.
    }
  }

  #read(): string[] {
    try {
      const stored: unknown = JSON.parse(this.#storage?.getItem(this.#key) ?? "[]");
      return Array.isArray(stored)
        ? stored.filter((id): id is string => typeof id === "string").slice(0, this.#max)
        : [];
    } catch {
      return [];
    }
  }
}
