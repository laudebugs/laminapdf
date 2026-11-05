//! Text extraction for retrieval pipelines and LLMs ([ADR 0030]): a page's text as blocks in
//! reading order, each with a box in top-left points ([ADR 0027]), with hidden text left out
//! and recorded.
//!
//! Built on `laminapdf-text`'s runs: runs on one baseline make lines, split at column gutters;
//! lines at a line's pitch make blocks; XY-Cut++ orders the blocks (or the content stream, on
//! request). Words hyphenated across lines are joined and Latin ligatures spelled out. Text
//! drawn invisibly, off the page or below 1 pt is never in a block's text; each such run is a
//! [`Dropped`] entry that counts its characters. Once every page is read, running headers,
//! footers, page numbers and artifact-marked text move to `dropped` too, and headings (from
//! font sizes and bold) and list items (from bullets and numbers) get their kinds.
//!
//! ```
//! # fn sample() -> Vec<u8> { std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/golden/text-std14.pdf")).unwrap() }
//! use laminapdf_extract::{ExtractOptions, extract};
//! use laminapdf_model::{Document, OpenOptions};
//! use laminapdf_syntax::CancelToken;
//!
//! let doc = Document::open(sample(), OpenOptions::default())?;
//! let out = extract(&doc, &ExtractOptions::new(), &CancelToken::new())?;
//! assert!(out.blocks[0].text.starts_with("Courier: Hello, world!"));
//! assert_eq!(out.pages[0].number, 1);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! What the rules can't do: they don't read structure trees yet, flag clipped or low-contrast
//! text, or untangle magazines, sidebars and floating figures (the crate README says more).
//!
//! [ADR 0027]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
//! [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md

#![forbid(unsafe_code)]

mod blocks;
mod error;
mod furniture;
mod headings;
mod job;
mod lines;
mod lists;
mod model;
mod options;
mod output;
mod page;
mod tables;
mod text_fix;
mod visibility;
mod xycut;

pub use error::ExtractError;
pub use job::{ExtractJob, extract, extract_job};
pub use model::{
    Block, BlockKind, Cell, Coordinates, DropReason, Dropped, ExtractedPage, ExtractedSource, Extraction, Flags,
    SCHEMA, Span, StructRef, StructSource, TableDetector,
};
pub use options::{DropOptions, ExtractLimits, ExtractOptions, ReadingOrder};
#[cfg(feature = "schema")]
pub use output::json_schema;
pub use output::{Chunk, ChunkOptions, MarkdownOptions, Region, Rendered, SplitOn, TextRange, tokens_approx};
#[cfg(feature = "serde")]
pub use output::{JsonOptions, chunks_jsonl, to_json};
pub use tables::TableMode;
