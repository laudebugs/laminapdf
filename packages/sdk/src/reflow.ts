// Reader mode's output: what `doc.reflow()` returns, typed from the JSON the extract and text
// modules write (`laminapdf_extract::Reflow`, serialised in camelCase).

/** Why reader mode refuses a document. */
export type ReflowRefusal = "untagged" | "empty" | "generic" | "coverage" | "order";

/** A block's kind and what goes with it. */
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
  | { readonly kind: "other" };

/** One block of the reading view. */
export type ReflowBlock = ReflowKind & {
  /** The text; empty for a figure, formula or table, which are shown as crops of their page. */
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

/** The measures behind reader mode's verdict. */
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
}

/** What `doc.reflow()` makes of a document. */
export interface Reflow {
  /** Whether reader mode can show it; when it can't, `reason` says why and `blocks` is empty. */
  readonly available: boolean;
  /** Why it can't; absent when it can. */
  readonly reason?: ReflowRefusal;
  /** The document's language (the catalog's `/Lang`), for the reading view's `lang`. */
  readonly lang?: string;
  /** The blocks, in the structure tree's reading order. */
  readonly blocks: readonly ReflowBlock[];
  /** The measures the checks read. */
  readonly checks: ReflowChecks;
}
