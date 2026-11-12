// Reader mode's output: what `doc.reflow()` returns, typed from the JSON the extract and text
// modules write (`laminapdf_extract::Reflow`, serialised in camelCase).

/**
 * Why reader mode doesn't use a document's structure tree (it reads the page layout instead), or,
 * as `"nothing"`, why it shows nothing.
 *
 * @since 0.2.0
 */
export type ReflowRefusal = "untagged" | "empty" | "generic" | "coverage" | "order" | "nothing";

/**
 * Where the reading order comes from: the structure tree, or the page layout (approximate).
 *
 * @since 0.2.0
 */
export type ReflowSource = "tags" | "approximate";

/**
 * Why the approximate reading view shows a page as its image.
 *
 * @since 0.2.0
 */
export type PageImageReason = "scanned" | "uncertain";

/**
 * A block's kind and what goes with it.
 *
 * @since 0.2.0
 */
export type ReflowKind =
  | { readonly kind: "heading"; readonly level: number }
  | { readonly kind: "paragraph" }
  | { readonly kind: "list-item"; readonly marker: string; readonly depth: number }
  | { readonly kind: "quote" }
  | { readonly kind: "caption" }
  | { readonly kind: "note" }
  | { readonly kind: "figure" }
  | { readonly kind: "formula" }
  | { readonly kind: "table" }
  | { readonly kind: "page-image"; readonly why: PageImageReason }
  | { readonly kind: "other" };

/**
 * One block of the reading view.
 *
 * @since 0.2.0
 */
export type ReflowBlock = ReflowKind & {
  /**
   * The text; empty for a figure, formula or table, which are shown as crops of their page, but
   * for an approximate table or page image, which keeps the text read there for searching.
   */
  readonly text: string;
  /** The page it is on (0-based; a crop's first page). */
  readonly page: number;
  /**
   * Its box on that page in page space, `[x, y, width, height]` in points from the top left:
   * where "back to the page" goes, and a crop's clip. `null` for a crop that paints nothing
   * reader mode could find.
   */
  readonly bbox: readonly [number, number, number, number] | null;
  /** A crop's alternate text (`/Alt`), when the file gives one. */
  readonly alt?: string;
};

/**
 * The measures behind reader mode's verdict.
 *
 * @since 0.2.0
 */
export interface ReflowChecks {
  /** Marked-content sequences in the structure tree. */
  readonly content: number;
  /** The share of them in no paragraph, heading, list or other block-level element. */
  readonly genericShare: number;
  /** Pages whose tree gave their order. */
  readonly pagesUsed: number;
  /** Pages whose tree the per-page checks ignored for its coverage. */
  readonly pagesCoverage: number;
  /** Pages whose tree the per-page checks ignored for its order. */
  readonly pagesOrder: number;
  /** Kendall's τ between the tree's order and the layout's (−1 to 1), or `null` with too few blocks. */
  readonly agreement: number | null;
  /** Pages the approximate view shows as their image; 0 with tags. */
  readonly pageImages: number;
}

/**
 * What `doc.reflow()` makes of a document.
 *
 * @since 0.2.0
 */
export interface Reflow {
  /** Whether reader mode can show it: false only when no page has text or a picture. */
  readonly available: boolean;
  /** Where the order comes from. */
  readonly source: ReflowSource;
  /** Why the structure tree wasn't used (or `"nothing"` when unavailable); absent with tags. */
  readonly reason?: ReflowRefusal;
  /** The document's language (the catalog's `/Lang`), for the reading view's `lang`. */
  readonly lang?: string;
  /** The blocks, in reading order. */
  readonly blocks: readonly ReflowBlock[];
  /** The measures the checks read. */
  readonly checks: ReflowChecks;
}
