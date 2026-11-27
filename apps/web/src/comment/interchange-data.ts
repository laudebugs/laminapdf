// What the interchange dialogs count and send, from the document's threads (Paper C06, C07): the
// comments each scope names, the options an export is made with, the flatten op for some
// comments, and the summary's request. Pure, so the dialogs and their tests agree.

import type { ExportCommentsOptions, Op } from "@laminapdf/sdk";
import type {
  CommentExportChoice,
  CommentExportPreview,
  FlattenCounts,
  ScopeCounts,
  SummaryChoice,
  SummaryComments,
  SummaryCount,
} from "@laminapdf/ui/editing";
import { paperSize, summaryWords } from "@laminapdf/ui/editing";
import type { CommentSummaryRequest } from "@laminapdf/viewer";
import type { ThreadView } from "./threads.ts";

/** What the dialogs know of the document's comments when they open. */
export interface CommentScopes {
  /** Every thread, in document order. */
  readonly all: readonly ThreadView[];
  /** The threads the Comments panel shows: all of them when the panel isn't open. */
  readonly shown: readonly ThreadView[];
  /** The threads on the page in view. */
  readonly page: readonly ThreadView[];
  /** The threads ticked in the panel, or the one selected on the page. */
  readonly selected: readonly ThreadView[];
}

/** The pages (0-based, ascending, once each) `threads` are on. */
export function pagesOf(threads: readonly ThreadView[]): number[] {
  return [...new Set(threads.map((t) => t.page))].sort((a, b) => a - b);
}

/** The threads Markdown's "Highlights and notes" names: text markup and notes. */
function notesOf(threads: readonly ThreadView[]): ThreadView[] {
  return threads.filter((t) => t.kind === "markup" || t.kind === "notes");
}

/** The threads an export's Comments choice names. */
function exportThreads(scopes: CommentScopes, choice: CommentExportChoice): readonly ThreadView[] {
  if (choice.scope === "highlightsAndNotes") return notesOf(scopes.all);
  return choice.scope === "shown" ? scopes.shown : scopes.all;
}

/** How many comments each choice of Export's Comments names. */
export function scopeCounts(scopes: CommentScopes): ScopeCounts {
  return { notes: notesOf(scopes.all).length, all: scopes.all.length, shown: scopes.shown.length };
}

/** The options an export is made with: "Shown in the panel" is the pages those comments are on. */
export function exportOptions(scopes: CommentScopes, choice: CommentExportChoice, file: string): ExportCommentsOptions {
  return {
    format: choice.format,
    comments: choice.scope === "highlightsAndNotes" ? "highlightsAndNotes" : "all",
    ...(choice.scope === "shown" ? { pages: pagesOf(scopes.shown) } : {}),
    file,
    markdown: choice.include,
  };
}

/** What the Markdown preview says of its comments: how many items, on how many pages. */
export function previewOf(text: string, scopes: CommentScopes, choice: CommentExportChoice): CommentExportPreview {
  const threads = exportThreads(scopes, choice);
  // "Shown" and "All" in Markdown still hold only highlights and notes, as the format does.
  const items = choice.scope === "highlightsAndNotes" ? threads : notesOf(threads);
  return { text, items: items.length, pages: pagesOf(items).length };
}

/** How many comments each Flatten scope names. */
export function flattenCounts(scopes: CommentScopes): FlattenCounts {
  return {
    all: scopes.all.length,
    page: scopes.page.length,
    shown: scopes.shown.length,
    selected: scopes.selected.length,
  };
}

/** The kinds, among `threads`, that flattening leaves alone: a sound has no picture to draw. */
export function staysAmong(threads: readonly ThreadView[]): string[] {
  return [...new Set(threads.filter((t) => t.subtype === "Sound").map((t) => t.subtype))];
}

/** The op that draws `threads`' comments into their pages and removes them, as the view shows them. */
export function flattenOp(threads: readonly ThreadView[]): Op {
  return { op: "flattenAnnots", args: { annots: threads.map((t) => t.id), intent: "view" } } as Op;
}

/** How many comments, on how many pages, each choice of the summary's Comments lists. */
export function summaryCounts(scopes: CommentScopes): Record<SummaryComments, SummaryCount> {
  const count = (threads: readonly ThreadView[]) => ({ comments: threads.length, pages: pagesOf(threads).length });
  return { all: count(scopes.all), shown: count(scopes.shown) };
}

/** What the worker makes the summary from: the fonts' bytes, and `choice` as the SDK takes it. */
export function summaryRequest(
  scopes: CommentScopes,
  choice: SummaryChoice,
  fonts: readonly Uint8Array[],
  title: string | undefined,
  language: string,
): CommentSummaryRequest {
  return {
    fonts,
    layout: choice.layout,
    ...(choice.comments === "shown" ? { comments: scopes.shown.map((t) => t.id) } : {}),
    replies: choice.replies,
    status: choice.status,
    emptyPages: choice.emptyPages,
    paper: paperSize(choice.paper),
    ...(title === undefined ? {} : { title }),
    language,
    words: summaryWords(),
  };
}
