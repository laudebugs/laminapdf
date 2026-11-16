// A contents page made from the bookmarks, in three steps so a document open in any module can
// have one: the listing (every module), the layout (the text module, which has the fonts) and the
// insert (every module). Document.createContentsPage runs all three.

import { packageError } from "./errors.js";
import type { Font } from "./font.js";
import { textModule } from "./init.js";
import type { PagePosition } from "./transaction.js";

/**
 * Where a contents page goes and what it lists: everything of {@link ContentsPageOptions} but its
 * fonts.
 *
 * @since 0.5.0
 */
export interface ContentsPlacement {
  /** Where it goes: before the first page when left out. */
  readonly at?: PagePosition;
  /** The levels of bookmarks it lists: 1, or 2 (the default). */
  readonly levels?: 1 | 2;
  /** The heading at its top: "Contents" when left out. */
  readonly title?: string;
  /** Whether a bookmark for the contents page goes first in the outline (no by default). */
  readonly bookmark?: boolean;
  /** The entries' size in points, from 4 to 72: 11 when left out. The heading is half again as large. */
  readonly size?: number;
}

/**
 * The fonts a contents page is set in.
 *
 * @since 0.5.0
 */
export interface ContentsFonts {
  /**
   * Its font: Helvetica when left out, which isn't embedded, so a file that claims PDF/A or
   * PDF/UA-1 refuses it.
   */
  readonly font?: Font;
  /** Fonts for the characters `font` lacks, tried in order (used only with `font`). */
  readonly fallbacks?: readonly Font[];
}

/**
 * How {@link Document.createContentsPage} makes a contents page. Every field has a default.
 *
 * @since 0.5.0
 */
export interface ContentsPageOptions extends ContentsPlacement, ContentsFonts {}

/**
 * One bookmark a contents page lists.
 *
 * @since 0.5.0
 */
export interface ContentsEntry {
  /** 1 for a top-level bookmark, 2 for one inside it. */
  readonly level: number;
  /** The bookmark's title. */
  readonly title: string;
  /**
   * The label its page shows once the contents page is in: `labels[n - 1]` when the contents page
   * takes `n` pages (one to ten).
   */
  readonly labels: readonly string[];
}

/**
 * What a contents page lists, from {@link Document.contentsListing}. Pass it on unchanged: the
 * insert compares it with the document's own listing.
 *
 * @since 0.5.0
 */
export interface ContentsListing {
  /** The entries, in outline order. */
  readonly entries: readonly ContentsEntry[];
  /** The listing as the library wrote it. */
  readonly json: string;
}

/**
 * A contents page laid out by {@link layOutContentsPage}: its document, each entry's box as six
 * numbers (entry, page, x, y, width, height in page space) and the listing it came from.
 *
 * @since 0.5.0
 */
export interface ContentsPage {
  readonly bytes: Uint8Array;
  readonly boxes: Float64Array;
  readonly listing: string;
}

/** @internal The listing JSON as an object with its entries. */
export function parseListing(json: string): ContentsListing {
  const { entries } = JSON.parse(json) as { entries: ContentsEntry[] };
  return { entries, json };
}

/**
 * Lays out the contents page of `listing` as a document of its own, in `fonts`: the heading, then
 * each entry indented by its level with dot leaders to its label. Needs the text module loaded
 * (`init({ text: true })`); the document the listing came from can be open in any module.
 *
 * @throws {@link LaminaPdfError} `text-module-not-loaded`; `font-not-embedded` for the default
 *   font on a file that claims PDF/A or PDF/UA-1; `missing-glyph` for a title no font can draw.
 *
 * @since 0.5.0
 */
export function layOutContentsPage(listing: ContentsListing, fonts: ContentsFonts = {}): ContentsPage {
  const text = textModule();
  if (text === undefined) {
    throw packageError("text-module-not-loaded", "layOutContentsPage() needs the text module: init({ text: true })");
  }
  const { font, fallbacks = [] } = fonts;
  const faces = (font === undefined ? [] : [font, ...fallbacks]).map((f) =>
    f.standard !== undefined
      ? { standard: f.standard }
      : { bytes: f.bytes as Uint8Array, ...(f.index === undefined ? {} : { index: f.index }) },
  );
  return text.call((glue) => glue.layOutContentsPage(listing.json, faces));
}
