// The page labels editor's model (Paper O10): its rows (one per range, as the dialog edits them),
// the labels they give each page, numbered as Lamina PDF numbers them (laminapdf-model's
// labels.rs), so the preview shows what readers show, the rules a row breaks, and the ranges the
// labels op takes (ISO 32000-2 §12.4.2).

/** A numbering style, as the labels op names it, or `none` for labels that are the prefix alone. */
export type LabelStyleName = "decimal" | "lowerRoman" | "upperRoman" | "lowerLetters" | "upperLetters" | "none";

/** The styles, in the order the dialog lists them. */
export const LABEL_STYLES: readonly LabelStyleName[] = [
  "decimal",
  "lowerRoman",
  "upperRoman",
  "lowerLetters",
  "upperLetters",
  "none",
];

/** One range as the labels op takes and gives it: its first page by ID, its style, prefix and first number. */
export interface LabelRangeValue {
  readonly page: string;
  readonly style?: Exclude<LabelStyleName, "none">;
  readonly prefix?: string;
  readonly first?: number;
}

/** One row of the dialog: the range's first page and first number as typed. */
export interface LabelRow {
  /** Its own key, kept while rows are added and removed. */
  readonly key: number;
  /** The first page's number, 1-based, as typed. */
  readonly from: string;
  readonly style: LabelStyleName;
  readonly prefix: string;
  /** The first page's number in the range, as typed. */
  readonly start: string;
}

/** Above this, roman numerals and letters are written in decimal, as Lamina PDF writes them. */
const MAX_SPELLED = 10_000;

const ROMAN: readonly (readonly [number, string])[] = [
  [1000, "M"],
  [900, "CM"],
  [500, "D"],
  [400, "CD"],
  [100, "C"],
  [90, "XC"],
  [50, "L"],
  [40, "XL"],
  [10, "X"],
  [9, "IX"],
  [5, "V"],
  [4, "IV"],
  [1, "I"],
];

function roman(value: number): string {
  let rest = value;
  let out = "";
  for (const [n, s] of ROMAN) {
    while (rest >= n) {
      out += s;
      rest -= n;
    }
  }
  return out;
}

/** A to Z for 1–26, AA to ZZ for 27–52, and so on (§12.4.2). */
function letters(value: number, base: string): string {
  const zeroBased = Math.max(0, value - 1);
  return String.fromCharCode(base.charCodeAt(0) + (zeroBased % 26)).repeat(Math.floor(zeroBased / 26) + 1);
}

/** The number part of a label for `value` (1 or more) in `style`. */
export function numberIn(style: LabelStyleName, value: number): string {
  if (style === "none") return "";
  if (style === "decimal" || value > MAX_SPELLED) return String(value);
  switch (style) {
    case "upperRoman":
      return roman(value);
    case "lowerRoman":
      return roman(value).toLowerCase();
    case "upperLetters":
      return letters(value, "A");
    case "lowerLetters":
      return letters(value, "a");
  }
}

/** A whole number from 1 up, as typed, or undefined. */
function counting(text: string): number | undefined {
  const value = text.trim();
  if (!/^\d+$/.test(value)) return undefined;
  const n = Number(value);
  return Number.isSafeInteger(n) && n >= 1 ? n : undefined;
}

let nextKey = 1;

/** A new row. */
export function row(from: number, style: LabelStyleName = "decimal", prefix = "", start = 1): LabelRow {
  return { key: nextKey++, from: String(from), style, prefix, start: String(start) };
}

/**
 * The dialog's rows for a document's ranges (`pages` its page IDs in order): one per range, or
 * one "1, 2, 3 from page 1" row for a document without labels. A range on a page the document
 * lacks is left out. A file whose first range starts after page 1 (readers number those pages
 * 1, 2, 3) gets a "1, 2, 3 from page 1" row first, since the labels op needs a range there.
 */
export function rowsOf(ranges: readonly LabelRangeValue[], pages: readonly string[]): LabelRow[] {
  const rows: LabelRow[] = [];
  for (const range of ranges) {
    const at = pages.indexOf(range.page);
    if (at < 0) continue;
    rows.push(row(at + 1, range.style ?? "none", range.prefix ?? "", range.first ?? 1));
  }
  return rows[0]?.from === "1" ? rows : [row(1), ...rows];
}

/** What is wrong with the rows: the rows whose first page clashes or is out of range, and those whose first number isn't one. */
export interface LabelProblems {
  /** Rows whose From field is out of 1 to the page count, by key. */
  readonly outside: ReadonlySet<number>;
  /** Rows that start on the same page as another, by key, and the first page two rows share (1-based). */
  readonly clash: { readonly keys: ReadonlySet<number>; readonly page: number } | undefined;
  /** Rows whose Start at field isn't a whole number from 1, by key. */
  readonly badStart: ReadonlySet<number>;
}

/** The rules the rows break, for `count` pages. */
export function problemsOf(rows: readonly LabelRow[], count: number): LabelProblems {
  const outside = new Set<number>();
  const badStart = new Set<number>();
  const byPage = new Map<number, number[]>();
  for (const r of rows) {
    const from = counting(r.from);
    if (from === undefined || from > count) outside.add(r.key);
    else byPage.set(from, [...(byPage.get(from) ?? []), r.key]);
    if (counting(r.start) === undefined) badStart.add(r.key);
  }
  let clash: LabelProblems["clash"];
  for (const [page, keys] of [...byPage].sort(([a], [b]) => a - b)) {
    if (keys.length > 1) {
      clash = { keys: new Set(keys), page };
      break;
    }
  }
  return { outside, clash, badStart };
}

/** Whether the rows can be applied. */
export function isValid(problems: LabelProblems): boolean {
  return problems.outside.size === 0 && problems.clash === undefined && problems.badStart.size === 0;
}

/** The rows in page order, with their first page (0-based) and first number read; rows with problems are left out. */
function readRows(rows: readonly LabelRow[], count: number) {
  return rows
    .map((r) => ({ row: r, at: (counting(r.from) ?? 0) - 1, first: counting(r.start) }))
    .filter(
      (r): r is { row: LabelRow; at: number; first: number } => r.at >= 0 && r.at < count && r.first !== undefined,
    )
    .sort((a, b) => a.at - b.at);
}

/** Each page's label under the rows, and the key of the row that labels it (undefined before the first range). */
export function labelsOf(rows: readonly LabelRow[], count: number): { label: string; key: number | undefined }[] {
  const read = readRows(rows, count);
  const out: { label: string; key: number | undefined }[] = [];
  let at = 0;
  for (let page = 0; page < count; page++) {
    while (at + 1 < read.length && (read[at + 1]?.at ?? count) <= page) at++;
    const range = read[at];
    if (range === undefined || range.at > page) {
      out.push({ label: String(page + 1), key: undefined });
      continue;
    }
    out.push({
      label: range.row.prefix + numberIn(range.row.style, range.first + page - range.at),
      key: range.row.key,
    });
  }
  return out;
}

/**
 * Each page's label under `ranges` (`pages` the page IDs in order), or undefined for every page
 * when the document labels none, for an app whose engine reports the ranges but not the labels.
 */
export function pageLabels(ranges: readonly LabelRangeValue[], pages: readonly string[]): (string | undefined)[] {
  if (ranges.length === 0) return pages.map(() => undefined);
  return labelsOf(rowsOf(ranges, pages), pages.length).map(({ label }) => label);
}

/** The ranges the labels op takes for the rows (`pages` the page IDs in order). */
export function rangesOf(rows: readonly LabelRow[], pages: readonly string[]): LabelRangeValue[] {
  return readRows(rows, pages.length).map(({ row: r, at, first }) => ({
    page: pages[at] ?? "",
    ...(r.style === "none" ? {} : { style: r.style }),
    ...(r.prefix === "" ? {} : { prefix: r.prefix }),
    ...(first === 1 ? {} : { first }),
  }));
}

/** The first two pages (1-based) that share a label, with the label, or undefined when all differ. */
export function sharedLabel(labels: readonly string[]): { first: number; second: number; label: string } | undefined {
  const seen = new Map<string, number>();
  for (const [i, label] of labels.entries()) {
    const earlier = seen.get(label);
    if (earlier !== undefined) return { first: earlier + 1, second: i + 1, label };
    seen.set(label, i);
  }
  return undefined;
}

/** One entry of the preview: a page and its label, or a run of pages left out. */
export type PreviewEntry =
  | { readonly kind: "page"; readonly page: number; readonly label: string; readonly key: number | undefined }
  | {
      readonly kind: "gap";
      readonly from: number;
      readonly to: number;
      readonly firstLabel: string;
      readonly lastLabel: string;
    };

/**
 * The preview: the first three pages, the pages around each range's start, and the last two, in
 * order, with each run between them as a gap. Pages are 1-based.
 */
export function previewOf(
  labels: readonly { label: string; key: number | undefined }[],
  starts: readonly number[],
): PreviewEntry[] {
  const count = labels.length;
  const shown = new Set<number>();
  const add = (i: number) => {
    if (i >= 0 && i < count) shown.add(i);
  };
  for (let i = 0; i < 3; i++) add(i);
  for (const start of starts) for (let i = start - 1; i <= start + 1; i++) add(i);
  add(count - 2);
  add(count - 1);
  const pages = [...shown].sort((a, b) => a - b);
  const out: PreviewEntry[] = [];
  let last = -1;
  for (const i of pages) {
    if (i > last + 1) {
      out.push({
        kind: "gap",
        from: last + 2,
        to: i,
        firstLabel: labels[last + 1]?.label ?? "",
        lastLabel: labels[i - 1]?.label ?? "",
      });
    }
    const entry = labels[i];
    out.push({ kind: "page", page: i + 1, label: entry?.label ?? "", key: entry?.key });
    last = i;
  }
  return out;
}

/** The 0-based first pages of the rows that can be read. */
export function startsOf(rows: readonly LabelRow[], count: number): number[] {
  return readRows(rows, count).map((r) => r.at);
}
