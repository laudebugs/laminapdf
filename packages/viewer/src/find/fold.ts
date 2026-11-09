// Text as find compares it, close to pdf.js's rules so match counts agree: compatibility forms
// folded (NFKC: `ﬁ` → `fi`, a no-break space → a space), soft hyphens dropped, a hyphen that ends a
// line joined to the next line's word, runs of white space as one space, and unless asked
// otherwise, case and diacritics ignored (pdf.js's defaults). Case folds with `toLowerCase`, not
// `toLocaleLowerCase`: a document's language isn't the reader's, and a Turkish locale would make
// `I` miss `i`. Diacritics are every combining mark (`\p{M}`), as pdf.js takes them: Latin
// accents, and also Hebrew points, Arabic harakat and Indic vowel signs, which the query loses
// too, so `שלום` finds the pointed `שָׁלוֹם`.
// Each folded UTF-16 unit remembers which source unit it came from, so a match maps back.

/** How find compares text. */
export interface FindOptions {
  /** Upper and lower case differ (default: they don't). */
  readonly matchCase?: boolean;
  /** Only whole words match (default: any part of a word). */
  readonly wholeWords?: boolean;
  /** Accents and other combining marks count (default: they don't). */
  readonly matchDiacritics?: boolean;
}

/** Folded text and, for each of its UTF-16 units, the source unit it came from. */
export interface Folded {
  readonly text: string;
  readonly from: Int32Array;
}

const SOFT_HYPHEN = 0xad;
const DIACRITICS = /\p{M}/gu;
const SPACE = /\s/;

/** One source character as find compares it. */
function foldOne(char: string, options: FindOptions): string {
  let out = char.normalize("NFKC");
  if (options.matchDiacritics !== true) out = out.normalize("NFD").replace(DIACRITICS, "");
  if (options.matchCase !== true) out = out.toLowerCase();
  return out;
}

/** `source` as find compares it. */
export function fold(source: string, options: FindOptions = {}): Folded {
  let text = "";
  const from: number[] = [];
  const push = (s: string, at: number) => {
    for (let k = 0; k < s.length; k++) from.push(at);
    text += s;
  };
  for (let i = 0; i < source.length; ) {
    const cp = source.codePointAt(i) ?? 0;
    const char = String.fromCodePoint(cp);
    const next = i + char.length;
    if (cp === SOFT_HYPHEN) {
      i = next;
      continue;
    }
    // A hyphen at a line's end joins the word it splits (pdf.js does the same).
    if (char === "-" && source[next] === "\n" && /\p{L}/u.test(text.at(-1) ?? "")) {
      i = next + 1;
      continue;
    }
    const folded = SPACE.test(char) ? " " : foldOne(char, options);
    if (folded === " " && (text === "" || text.endsWith(" "))) {
      i = next;
      continue;
    }
    push(folded, i);
    i = next;
  }
  return { text, from: Int32Array.from(from) };
}

/** The query as find compares it: folded, and trimmed of the spaces at its ends. */
export function foldQuery(query: string, options: FindOptions = {}): string {
  return fold(query, options).text.trim();
}

const WORD = /[\p{L}\p{N}\p{M}_]/u;

/** Whether UTF-16 unit `code` is the second half of a surrogate pair. */
export const isLowSurrogate = (code: number): boolean => code >= 0xdc00 && code <= 0xdfff;

/** The character that ends just before `at` in `text`, a surrogate pair whole. */
function charBefore(text: string, at: number): string {
  if (at <= 0) return "";
  const low = text.charCodeAt(at - 1);
  const pair = isLowSurrogate(low) && at >= 2;
  return String.fromCodePoint(text.codePointAt(pair ? at - 2 : at - 1) ?? 0);
}

/**
 * Whether `text[start, end)` stands as whole words: no letter, digit or mark runs on at either
 * end. This is pdf.js's test, by character class, rather than `Intl.Segmenter`'s word breaks,
 * so whole-word counts agree with pdf.js's.
 */
function wholeAt(text: string, start: number, end: number): boolean {
  const before = charBefore(text, start);
  const after = end < text.length ? String.fromCodePoint(text.codePointAt(end) ?? 0) : "";
  // A query that starts or ends in a non-word character may sit against a word.
  const first = text.slice(start, start + 2);
  const last = text.slice(Math.max(start, end - 2), end);
  return (!WORD.test(before) || !WORD.test(first.at(0) ?? "")) && (!WORD.test(after) || !WORD.test(last.at(-1) ?? ""));
}

/** Every `[start, end)` of `query` (already folded) in `text`, in order, never overlapping. */
export function matches(text: string, query: string, wholeWords = false): [number, number][] {
  const out: [number, number][] = [];
  if (query === "") return out;
  for (let at = text.indexOf(query); at >= 0; ) {
    const end = at + query.length;
    if (!wholeWords || wholeAt(text, at, end)) {
      out.push([at, end]);
      at = text.indexOf(query, end);
    } else {
      at = text.indexOf(query, at + 1);
    }
  }
  return out;
}
