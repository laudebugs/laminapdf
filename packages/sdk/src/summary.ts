// The comment summary: a new PDF/UA-1 document listing a review's comments, laid out by the text
// module, which has the fonts.

import type { WasmDocument } from "@laminapdf/wasm/base";
import type { WasmDocument as TextWasmDocument } from "@laminapdf/wasm/text";
import { fontFaces } from "./contents.js";
import { packageError } from "./errors.js";
import type { Font } from "./font.js";

/**
 * What {@link Document.commentSummary} shows and how. Only `font` is needed; the rest have
 * defaults.
 *
 * @since 0.6.0
 */
export interface CommentSummaryOptions {
  /**
   * The font its text is set in, embedded and subset. A standard 14 font is refused
   * (`font-not-embedded`), since PDF/UA-1 needs embedded fonts.
   */
  readonly font: Font;
  /** Fonts for the characters `font` lacks, tried in order. */
  readonly fallbacks?: readonly Font[];
  /**
   * `"besidePage"` (the default): each page drawn small at the left, with its comments' numbers
   * marked on it and the comments beside it. `"list"`: each page's comments under its heading,
   * without the page.
   */
  readonly layout?: "besidePage" | "list";
  /** The IDs of the comments to list, such as those a comments panel shows; every comment when left out. */
  readonly comments?: readonly string[];
  /** Whether each comment's replies are listed under it (yes by default). */
  readonly replies?: boolean;
  /** Whether each comment's latest status and check mark are listed (yes by default). */
  readonly status?: boolean;
  /** Whether pages without comments are listed too, each saying it has none (no by default). */
  readonly emptyPages?: boolean;
  /** `"document"`, the size of the first page (the default), or `[width, height]` in points. */
  readonly paper?: "document" | readonly [number, number];
  /**
   * Its title: "Comments on" and the document's title by default, or "Comment summary" when the
   * document has no title.
   */
  readonly title?: string;
  /**
   * The summary's language, a BCP 47 tag (`"en"` by default): give the language most comments
   * are in, so screen readers read them in it. Give the summary's own words in it too (`words`).
   */
  readonly language?: string;
  /**
   * The words the summary writes itself, such as "Page", in the summary's `language`. Any you
   * leave out stay in English.
   *
   * @since 0.6.0
   */
  readonly words?: CommentSummaryWords;
}

/**
 * The words a comment summary writes itself. Each word is a template: the summary fills in the
 * names in braces, such as `{label}`, and a template may leave one out. Any word you leave out
 * stays in English. The comments' own text, their authors and the dates are never translated.
 *
 * @example
 * ```ts
 * const words: CommentSummaryWords = {
 *   page: "Seite {label}",
 *   kinds: { Text: "Notiz", Highlight: "Hervorhebung" },
 * };
 * ```
 *
 * @since 0.6.0
 */
export interface CommentSummaryWords {
  /** The title of a document that has one: "Comments on {title}". */
  readonly titled?: string;
  /** The title of a document that has none: "Comment summary". */
  readonly untitled?: string;
  /** The line under the title when the document has no comments. */
  readonly noComments?: string;
  /** The line under the title when none of the chosen comments are in the document. */
  readonly noneChosen?: string;
  /**
   * The line counting the comments and their pages: "{comments} on {pages}.", built from
   * `commentsOne` or `commentsOther` and `pagesOne` or `pagesOther`.
   */
  readonly count?: string;
  /** One comment: "1 comment". */
  readonly commentsOne?: string;
  /**
   * Several comments: "{count} comments". A language with more plural forms can phrase the count
   * line to avoid them, such as "Comments: {count}".
   */
  readonly commentsOther?: string;
  /** One page: "1 page". */
  readonly pagesOne?: string;
  /** Several pages: "{count} pages". */
  readonly pagesOther?: string;
  /**
   * Each page's heading: "Page {label}". It is also the alternative text of a page drawn beside
   * its comments when it marks none.
   */
  readonly page?: string;
  /** The line under a page without comments, when those pages are listed. */
  readonly pageEmpty?: string;
  /** The alternative text of a page drawn beside one comment: "Page {label}, with comment {first} marked". */
  readonly figureOne?: string;
  /** The alternative text of a page drawn beside several: "Page {label}, with comments {first} to {last} marked". */
  readonly figureMany?: string;
  /** The status line: "Status: {status}". */
  readonly status?: string;
  /** The check mark, after the status or alone: "Marked". */
  readonly marked?: string;
  /** The text a comment marks, quoted under it: "“{text}”". */
  readonly quote?: string;
  /** Annotation kinds by their PDF subtype (`Text`, `Ink`…), in place of the English names: "Note", "Drawing"…. */
  readonly kinds?: Readonly<Record<string, string>>;
  /** Review states by their PDF name (`Accepted`, `Rejected`, `Cancelled`, `Completed`). */
  readonly states?: Readonly<Record<string, string>>;
}

/** @internal The summary of `doc` as `options` asks, when `doc` lives in the text module. */
export function commentSummaryOf(doc: WasmDocument, options: CommentSummaryOptions): WasmDocument {
  const text = doc as Partial<TextWasmDocument>;
  if (typeof text.commentSummary !== "function") {
    throw packageError(
      "text-module-not-loaded",
      "doc.commentSummary() needs a document opened while the text module was loaded: init({ text: true })",
    );
  }
  const { font, fallbacks = [], comments, paper, ...rest } = options;
  const init = {
    ...rest,
    ...(comments === undefined ? {} : { comments: [...comments] }),
    ...(paper === undefined ? {} : { paper: typeof paper === "string" ? paper : ([...paper] as [number, number]) }),
  };
  const summary = text.commentSummary(init, fontFaces([font, ...fallbacks]));
  return summary as unknown as WasmDocument;
}
