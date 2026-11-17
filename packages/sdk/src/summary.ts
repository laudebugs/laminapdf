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
  /** The font its text is set in, embedded and subset. A standard 14 font is refused under PDF/UA-1. */
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
  /** Its title: "Comments on" and the document's title by default, or "Comment summary". */
  readonly title?: string;
  /**
   * The language most comments are in, a BCP 47 tag (`"en"` by default), which screen readers
   * read them in. The summary's own words, such as "Page", are English.
   */
  readonly language?: string;
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
