// What Split and Export ranges make (Paper O08): the parts a document is cut into, every `n` pages,
// at page ranges (`1–6, 7–15`: each comma part a file, read as the print dialog reads a range), or
// where its top-level bookmarks begin; which pages two parts share; and each part's file name.

import { laterMessage } from "../later-messages.js";
import { parsePrintPages } from "../reader/print-pages.js";
import { pageRanges } from "./moves.js";

/** One file a split makes: its pages (0-based, in order) and, cut at a bookmark, that bookmark's title. */
export interface Part {
  readonly pages: readonly number[];
  readonly title?: string;
}

/** The parts a split makes, or why it makes none. */
export type PartsResult =
  | { readonly ok: true; readonly parts: readonly Part[]; readonly shared: readonly number[] }
  | { readonly ok: false; readonly error: string };

/** The most parts one split makes: a zip holds at most 65,535 files, and a list this long is a mistake. */
export const MAX_PARTS = 1000;

/** The parts of a document of `total` pages, `count` pages each, the last with what is left. */
export function partsEvery(count: number, total: number): PartsResult {
  if (!Number.isInteger(count) || count < 1 || count > total) {
    return { ok: false, error: laterMessage("documents.split.count.error", { total }) };
  }
  const parts: Part[] = [];
  for (let start = 0; start < total; start += count) {
    parts.push({ pages: Array.from({ length: Math.min(count, total - start) }, (_, i) => start + i) });
  }
  return parts.length > MAX_PARTS ? tooMany() : { ok: true, parts, shared: [] };
}

/**
 * The parts `text` names: one per comma part (`1–6, 7–15, 18`), each read as the print dialog
 * reads a range, by number or label. Pages in two parts are allowed and listed in `shared`.
 */
export function partsOfRanges(text: string, total: number, labels?: readonly string[]): PartsResult {
  const pieces = text
    .split(",")
    .map((piece) => piece.trim())
    .filter((piece) => piece !== "");
  if (pieces.length === 0) return { ok: false, error: laterMessage("documents.split.ranges.empty") };
  const parts: Part[] = [];
  for (const piece of pieces) {
    const read = parsePrintPages(piece, total, labels);
    if (!read.ok) return read;
    parts.push({ pages: [...new Set(read.pages)] });
  }
  if (parts.length > MAX_PARTS) return tooMany();
  const seen = new Map<number, number>();
  for (const part of parts) for (const page of part.pages) seen.set(page, (seen.get(page) ?? 0) + 1);
  const shared = [...seen].flatMap(([page, times]) => (times > 1 ? [page] : [])).sort((a, b) => a - b);
  return { ok: true, parts, shared };
}

/** One top-level bookmark: its title and the page (0-based) it goes to. */
export interface Chapter {
  readonly title: string;
  readonly page: number;
}

/**
 * The parts that begin at `chapters` (top-level bookmarks, in outline order) in a document of
 * `total` pages: each runs to the page before the next one that starts later. Pages before the
 * first are a part of their own, untitled; bookmarks to the same page as an earlier one start none.
 */
export function partsAtBookmarks(chapters: readonly Chapter[], total: number): PartsResult {
  const starts = chapters
    .filter((chapter) => chapter.page >= 0 && chapter.page < total)
    .sort((a, b) => a.page - b.page)
    .filter((chapter, i, all) => i === 0 || chapter.page !== all[i - 1]?.page);
  if (starts.length === 0) return { ok: false, error: laterMessage("documents.split.bookmarks.none") };
  const parts: Part[] = [];
  const first = starts[0] as Chapter;
  if (first.page > 0) parts.push({ pages: Array.from({ length: first.page }, (_, i) => i) });
  starts.forEach((chapter, i) => {
    const end = starts[i + 1]?.page ?? total;
    parts.push({ pages: Array.from({ length: end - chapter.page }, (_, k) => chapter.page + k), title: chapter.title });
  });
  return parts.length > MAX_PARTS ? tooMany() : { ok: true, parts, shared: [] };
}

function tooMany(): PartsResult {
  return { ok: false, error: laterMessage("documents.split.tooMany", { max: MAX_PARTS }) };
}

/** How Split names its files (O08): by number and bookmark, by file name and number, or by file name and pages. */
export type PartNaming = "number-title" | "name-number" | "name-pages";

/**
 * Each part's file name, without `.pdf` (the shell makes it safe to save): `1. Repairs backlog`,
 * `Q3-board-report 2` or `Q3-board-report pages 3-6`, from the open file's `stem` and the pages'
 * labels. Names that would repeat get ` (2)`, ` (3)`.
 */
export function partNames(
  parts: readonly Part[],
  naming: PartNaming,
  stem: string,
  label: (index: number) => string,
): string[] {
  const width = String(parts.length).length;
  const names = parts.map((part, i) => {
    const number = String(i + 1).padStart(width, "0");
    if (naming === "name-number") return `${stem} ${number}`;
    // A hyphen, not the en dash the dialogs show: names stay plain ASCII where the labels allow.
    if (naming === "name-pages") {
      const pages = pageRanges(part.pages, label).replaceAll("–", "-");
      return laterMessage("documents.split.name.pages", { stem, pages });
    }
    return part.title === undefined ? `${number}. ${stem}` : `${number}. ${part.title}`;
  });
  const used = new Map<string, number>();
  return names.map((name) => {
    const key = name.toLowerCase();
    const times = (used.get(key) ?? 0) + 1;
    used.set(key, times);
    return times === 1 ? name : `${name} (${times})`;
  });
}
