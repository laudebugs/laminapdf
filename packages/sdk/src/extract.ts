// doc.extract(): a document's text for retrieval and LLMs. The WASM returns one string per call
// (Markdown, text, the laminapdf.extract/1 JSON or the chunks as a JSON array); this module
// names the options and parses the string into the type each `format` promises. The JSON's
// types are generated from its schema (generated/extract.ts).
import type {
  ExtractFormat,
  ReadingOrderName as ReadingOrder,
  SplitOnName as SplitOn,
  TableModeName as TableMode,
} from "@laminapdf/wasm/extract";
import type { LaminaPdfWarning } from "./errors.js";
import type { Chunk, Extraction as ExtractionJson } from "./generated/extract.js";

export type {
  Cell,
  Coordinates,
  Dropped,
  DropReason,
  ExtractedPage,
  ExtractedSource,
  Flags,
  Generator,
  JsonBlock as ExtractedBlock,
  Region,
  Span,
  StructRef,
  StructSource,
  TableDetector,
  TextRange,
} from "./generated/extract.js";

/**
 * The `laminapdf.extract/1` document: the schema's type, with the file's warnings as the
 * {@link LaminaPdfWarning}s they are (the schema leaves their shape to ADR 0026's).
 */
export type Extraction = Omit<ExtractionJson, "warnings"> & { warnings: LaminaPdfWarning[] };

// The format and option names come from the extract module's own declarations (tsify, from
// crates/laminapdf-wasm/src/extract.rs), so they can't drift from what the binding accepts.
export type { ExtractFormat, ReadingOrder, SplitOn, TableMode };

/** How the Markdown is cut into chunks. */
export interface ChunkOptions {
  /** A chunk's budget in approximate tokens (characters / 4); 512 by default. Blocks are never cut. */
  readonly maxTokens?: number;
  /** Approximate tokens of whole blocks a chunk repeats from the one before it; 0 by default. */
  readonly overlap?: number;
  /** Where chunks may start. */
  readonly splitOn?: SplitOn;
}

/** Options for {@link Document.extract}. */
export interface ExtractOptions<F extends ExtractFormat = ExtractFormat> {
  /** What to return: Markdown or plain text as a string, the JSON as an {@link Extraction}, or {@link Chunk}s. */
  readonly format: F;
  /** The 0-based page indices to extract, in this order; every page by default. */
  readonly pages?: readonly number[];
  /** Where the reading order comes from; `"auto"` by default. */
  readonly readingOrder?: ReadingOrder;
  /** `keepSpans` keeps each hidden run's text in its `dropped` entry (JSON), for auditing a file. */
  readonly drop?: { readonly keepSpans?: boolean };
  /** Which table detectors run; `"lattice"` by default. */
  readonly tables?: TableMode;
  /** Join words hyphenated across lines; on by default. */
  readonly dehyphenate?: boolean;
  /** Write `<!-- page N -->` before each page's first block (Markdown, chunks and the JSON's text ranges). */
  readonly pageMarkers?: boolean;
  /** How chunks are cut (`"chunks"`, and `"json"` to include them). */
  readonly chunk?: ChunkOptions;
}

/** The result of {@link Document.extract} for each format. */
export interface ExtractResult {
  readonly markdown: string;
  readonly text: string;
  readonly json: Extraction;
  readonly chunks: Chunk[];
}

/** The WASM's string as the type `format` promises. */
export function parseExtract<F extends ExtractFormat>(format: F, out: string): ExtractResult[F] {
  return (format === "json" || format === "chunks" ? JSON.parse(out) : out) as ExtractResult[F];
}

export type { Chunk };
