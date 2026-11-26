// What the interchange dialogs decide, apart from how they look (Paper C06, C07): export's
// choices and the name it saves under, what an import's summary says in lines, which scopes
// Flatten offers, what its signature notes and its confirm button are, and the summary's options.
// The dialogs are pure views over these; the shell does the work.

/** Something an import says, as the SDK's `ImportReportWarning`: `interchange-skipped`, `interchange-document-mismatch` or `annot-name-duplicate`. */
export interface ImportReportWarning {
  readonly code: string;
  /** What is skipped: an annotation's subtype (`Sound`) or `field`. */
  readonly kind?: string | undefined;
  /** Why: `unsupported`, `form-field`, `missing-page`, `missing-parent` or `incomplete`. */
  readonly reason?: string | undefined;
  /** Its page, 0-based. */
  readonly page?: number | undefined;
  /** Its line in the file, from 1. */
  readonly line?: number | undefined;
  /** For a mismatch, the PDF the file names. */
  readonly file?: string | undefined;
  /** For a duplicate, the ID two comments share. */
  readonly name?: string | undefined;
}

/** What an import does or did, as the SDK's `ImportReport` reports it. */
export interface ImportReport {
  readonly added: number;
  readonly matched: number;
  readonly changed: number;
  readonly deleted: number;
  /** The pages (0-based) the added comments are on. */
  readonly pages: readonly number[];
  readonly warnings: readonly ImportReportWarning[];
}

/** The formats Export writes, in the order its radios list them. */
export const EXPORT_FORMATS = ["xfdf", "fdf", "markdown"] as const;

/** One of them. */
export type CommentExportFormat = (typeof EXPORT_FORMATS)[number];

/** Which comments Export writes, in the order its select lists them. */
export const EXPORT_SCOPES = ["highlightsAndNotes", "all", "shown"] as const;

/** One of them. */
export type CommentExportScope = (typeof EXPORT_SCOPES)[number];

/** What Markdown includes, each part on or off. */
export interface MarkdownInclude {
  readonly headings: boolean;
  readonly authors: boolean;
  readonly replies: boolean;
}

/** Export's choices. */
export interface CommentExportChoice {
  readonly format: CommentExportFormat;
  readonly scope: CommentExportScope;
  readonly include: MarkdownInclude;
}

/** How many comments each scope names: the notes in the file, all of them, and those the panel shows. */
export interface ScopeCounts {
  readonly notes: number;
  readonly all: number;
  readonly shown: number;
}

/** The scope a format starts on: Markdown is for highlights and notes, the others carry everything. */
function startScope(format: CommentExportFormat): CommentExportScope {
  return format === "markdown" ? "highlightsAndNotes" : "all";
}

/** The choices when Export opens on `format`. */
export function startChoice(format: CommentExportFormat): CommentExportChoice {
  return { format, scope: startScope(format), include: { headings: true, authors: true, replies: true } };
}

/** `choice` moved to `format`: a scope still at the old format's start follows it. */
export function withFormat(choice: CommentExportChoice, format: CommentExportFormat): CommentExportChoice {
  const follows = choice.scope === startScope(choice.format);
  return { ...choice, format, scope: follows ? startScope(format) : choice.scope };
}

/** The name Export saves under, from the document's name without `.pdf`. */
export function exportName(stem: string, format: CommentExportFormat): string {
  return format === "markdown" ? `${stem} – highlights.md` : `${stem}.${format}`;
}

/** The media type of an export's file. */
export function exportType(format: CommentExportFormat): string {
  if (format === "xfdf") return "application/vnd.adobe.xfdf";
  if (format === "fdf") return "application/vnd.fdf";
  return "text/markdown";
}

/** How many comments `scope` names, `counts` being what the document has. */
export function scopeCount(counts: ScopeCounts, scope: CommentExportScope): number {
  return scope === "highlightsAndNotes" ? counts.notes : scope === "all" ? counts.all : counts.shown;
}

/** The pages (1-based, ascending, once each) of `indices` as a list: "1, 3 and 16" (`and` is the word). */
export function pageList(indices: readonly number[], and: string): string {
  const pages = [...new Set(indices)].sort((a, b) => a - b).map((i) => String(i + 1));
  const last = pages.pop();
  if (last === undefined) return "";
  return pages.length === 0 ? last : `${pages.join(", ")} ${and} ${last}`;
}

/** What an import does with the comments already in the file, in the order its radios list them. */
export const IMPORT_MODES = ["merge", "replace", "add"] as const;

/** One of them. */
export type ImportMode = (typeof IMPORT_MODES)[number];

/** What the import dialog shows of reading its file. */
export type ImportRead =
  | { readonly state: "reading" }
  | { readonly state: "ready"; readonly summary: ImportReport }
  | { readonly state: "failed"; readonly format: string | undefined; readonly line: number | undefined };

/** What reading failed on: the error's `format` and `line` when it is an interchange error. */
export function importFailure(error: unknown): Extract<ImportRead, { state: "failed" }> {
  const data =
    typeof error === "object" && error !== null && "data" in error
      ? (error as { data?: Readonly<Record<string, unknown>> }).data
      : undefined;
  const format = data?.format;
  const line = data?.line;
  return {
    state: "failed",
    format: typeof format === "string" ? format : undefined,
    line: typeof line === "number" ? line : undefined,
  };
}

/** The import's warnings that skip something, in the order they came. */
export function skipped(summary: ImportReport): ImportReportWarning[] {
  return summary.warnings.filter((w) => w.code === "interchange-skipped");
}

/** The PDF the file says it was made for, when that isn't this one. */
export function mismatch(summary: ImportReport): string | undefined {
  const found = summary.warnings.find((w) => w.code === "interchange-document-mismatch");
  return found === undefined ? undefined : (found.file ?? "");
}

/** How many comments in the file share an ID with another in it. */
export function duplicates(summary: ImportReport): number {
  return summary.warnings.filter((w) => w.code === "annot-name-duplicate").length;
}

/** The comments the import changes: those it adds (`merge`, `add`) or replaces. */
export function importCount(summary: ImportReport): number {
  return summary.added;
}

/** Flatten's scopes, in the order its radios list them. */
const FLATTEN_SCOPES = ["all", "page", "shown", "selected"] as const;

/** One of them. */
export type FlattenChoice = (typeof FLATTEN_SCOPES)[number];

/** How many comments each Flatten scope names; `selected` is 0 with nothing selected. */
export type FlattenCounts = Readonly<Record<FlattenChoice, number>>;

/** The scopes Flatten offers: Selected only with a selection. */
export function flattenScopes(counts: FlattenCounts): FlattenChoice[] {
  return FLATTEN_SCOPES.filter((scope) => scope !== "selected" || counts.selected > 0);
}

/** The scope Flatten starts on: the selection when there is one, else all. */
export function startFlatten(counts: FlattenCounts): FlattenChoice {
  return counts.selected > 0 ? "selected" : "all";
}

/** How the file's signatures meet Flatten (E5 plan §5). */
export interface FlattenSignatures {
  /** Approval signatures, which flattening keeps valid while their details show the change. */
  readonly approval: number;
  /** A certification or usage rights that flattening breaks: only a copy keeps the original signed. */
  readonly certified: boolean;
}

/** Flatten's confirmation, from what the person ticked. */
export interface FlattenAnswer {
  readonly scope: FlattenChoice;
  /** Also remove the comments from the file's history: the save becomes a full rewrite. */
  readonly history: boolean;
  /** "I understand the signature will no longer be valid". */
  readonly understood: boolean;
}

/** What Flatten's confirm button does. */
export type FlattenAction = "flatten" | "rewrite" | "copy";

/**
 * What the confirm button does and whether it can: a certified file offers a copy, until the
 * override is ticked; the history option is a rewrite, which a signed file can't survive and so
 * needs the override.
 */
export function flattenAction(
  signatures: FlattenSignatures,
  answer: FlattenAnswer,
  count: number,
): { readonly action: FlattenAction; readonly enabled: boolean } {
  const signed = signatures.approval > 0 || signatures.certified;
  const overridden = answer.understood;
  if (signatures.certified && !overridden) return { action: "copy", enabled: count > 0 };
  if (answer.history) {
    return { action: "rewrite", enabled: count > 0 && (!signed || overridden) };
  }
  return { action: "flatten", enabled: count > 0 };
}

/** Whether the override checkbox shows: a change in place that breaks a signature. */
export function needsOverride(signatures: FlattenSignatures, history: boolean): boolean {
  return signatures.certified || (history && signatures.approval > 0);
}

/** The kinds a summary lays out, in the order its Layout radios list them. */
export const SUMMARY_LAYOUTS = ["besidePage", "list"] as const;

/** One of them. */
type SummaryLayout = (typeof SUMMARY_LAYOUTS)[number];

/** What a summary lists. */
export type SummaryComments = "all" | "shown";

/** Paper for the summary: the document's, or one size. */
export type SummaryPaper = "document" | "a4" | "letter";

/** Every choice of the summary dialog. */
export interface SummaryChoice {
  readonly layout: SummaryLayout;
  readonly comments: SummaryComments;
  readonly replies: boolean;
  readonly status: boolean;
  readonly emptyPages: boolean;
  readonly paper: SummaryPaper;
}

/** The summary dialog's starting choices (C07). */
export const START_SUMMARY: SummaryChoice = {
  layout: "besidePage",
  comments: "all",
  replies: true,
  status: true,
  emptyPages: false,
  paper: "document",
};

/** A4 and US Letter, in points. */
const PAPER_SIZES = { a4: [595.28, 841.89], letter: [612, 792] } as const;

/** The SDK's `paper` option for `paper`. */
export function paperSize(paper: SummaryPaper): "document" | readonly [number, number] {
  return paper === "document" ? "document" : PAPER_SIZES[paper];
}

/** One page the summary listed without drawing it, and why. */
export interface NotDrawn {
  /** The page, 0-based. */
  readonly page: number;
  readonly reason: "fonts-not-embedded" | "content-unreadable" | string;
}

/** The `summary-page-not-drawn` warnings among `warnings`, as pages and reasons. */
export function notDrawn(
  warnings: readonly { readonly code: string; readonly data?: Readonly<Record<string, unknown>> | undefined }[],
): NotDrawn[] {
  return warnings.flatMap((w) => {
    const page = w.data?.page;
    const reason = w.data?.reason;
    return w.code === "summary-page-not-drawn" && typeof page === "number"
      ? [{ page, reason: typeof reason === "string" ? reason : "" }]
      : [];
  });
}
