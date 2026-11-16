// The print dialog's page range (R09): page numbers or page labels, `1–5, 11, A-1`. A label may
// hold a hyphen (`A-1`), so a part that is a whole label is that page; otherwise a range splits at
// an en dash, or at a hyphen between two numbers or labels.

import { countMessage, message } from "../messages.js";

/** The pages a range names (0-based, in the order given), or why it names none. */
export type PrintPagesResult =
  | { readonly ok: true; readonly pages: readonly number[] }
  | { readonly ok: false; readonly error: string };

/** The 0-based page `part` names: a label first, then a number from 1 to `total`. */
function pageOf(part: string, total: number, labels: readonly string[] | undefined): number | undefined {
  const byLabel = labels?.indexOf(part) ?? -1;
  if (byLabel >= 0) return byLabel;
  if (!/^\d+$/.test(part)) return undefined;
  const n = Number(part);
  return n >= 1 && n <= total ? n - 1 : undefined;
}

/** `pages (1–21, A-1–A-3)` for the error: the count, and the labels' first and last when labelled. */
function described(total: number, labels: readonly string[] | undefined): string {
  if (labels === undefined || labels.length === 0) {
    return countMessage(total, { one: "print.range.pagesOne", many: "print.range.pages" });
  }
  return message("print.range.pagesLabelled", { total, first: labels[0] ?? "", last: labels.at(-1) ?? "" });
}

/** Reads `text` against a document of `total` pages, labelled by `labels` when it labels them. */
export function parsePrintPages(text: string, total: number, labels?: readonly string[]): PrintPagesResult {
  const parts = text
    .split(",")
    .map((p) => p.trim())
    .filter((p) => p !== "");
  if (parts.length === 0) return { ok: false, error: message("print.range.empty") };
  const pages: number[] = [];
  for (const part of parts) {
    const whole = pageOf(part, total, labels);
    if (whole !== undefined) {
      pages.push(whole);
      continue;
    }
    const range = splitRange(part, total, labels);
    if (range === undefined) {
      const missing = /^\d+$/.test(part)
        ? part
        : part.split(/[–-]/).find((p) => pageOf(p.trim(), total, labels) === undefined);
      return {
        ok: false,
        error: message("print.range.noPage", { page: (missing ?? part).trim(), pages: described(total, labels) }),
      };
    }
    const [from, to] = range;
    if (to < from) return { ok: false, error: message("print.range.backwards", { range: part }) };
    for (let page = from; page <= to; page++) pages.push(page);
  }
  return { ok: true, pages };
}

/** `part` as a range of two pages, at an en dash or at the one hyphen whose sides both name a page. */
function splitRange(
  part: string,
  total: number,
  labels: readonly string[] | undefined,
): readonly [number, number] | undefined {
  const separators = [...part.matchAll(/[–-]/g)].map((m) => m.index);
  for (const at of separators) {
    const from = pageOf(part.slice(0, at).trim(), total, labels);
    const to = pageOf(part.slice(at + 1).trim(), total, labels);
    if (from !== undefined && to !== undefined) return [from, to];
  }
  return undefined;
}

/** Which pages a dialog's Pages group names: every page, the one in view, or a typed range. */
export type PagesMode = "all" | "current" | "range";

/** What a dialog with a Pages group knows about the document. */
export interface PagesContext {
  readonly pageCount: number;
  /** The page in view, 0-based. */
  readonly currentPage: number;
  readonly labels?: readonly string[] | undefined;
}

/** The 0-based pages `mode` names, or the range's error. */
export function chosenPages(
  mode: PagesMode,
  range: string,
  context: PagesContext,
): { pages: readonly number[] } | { error: string } {
  if (mode === "all") return { pages: Array.from({ length: context.pageCount }, (_, i) => i) };
  if (mode === "current") return { pages: [context.currentPage] };
  const result = parsePrintPages(range, context.pageCount, context.labels);
  return result.ok ? { pages: result.pages } : { error: result.error };
}

/** The page in view as the document names it: its label, else its number. */
export function currentLabel(context: Pick<PagesContext, "currentPage" | "labels">): string {
  return context.labels?.[context.currentPage] ?? String(context.currentPage + 1);
}
