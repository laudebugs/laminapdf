//! Hidden text alone ([ADR 0030] §4): the page pass that flags runs, without lines, blocks,
//! tables, reading order or page furniture. `laminapdf check` uses it to count what a failed
//! redaction leaves behind; it reads every page through the same code as [`crate::extract`],
//! so the two can't drift.
//!
//! [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md

use laminapdf_model::Document;
use laminapdf_syntax::CancelToken;

use crate::error::ExtractError;
use crate::job::extract_job;
use crate::model::Dropped;
use crate::options::{ExtractOptions, ReadingOrder};
use crate::plugins::Models;
use crate::tables::TableMode;

/// The text of `doc`'s pages that a reader can't see: a `dropped` entry for each run, or
/// stretch of a run, that §4's flags leave out (invisible, off the page, tiny, clipped, low
/// contrast, covered), in page order. Artifacts go through the same flags as the rest, so a box
/// over an `Artifact`-marked line still shows as covered; they aren't hidden text themselves.
/// `options` picks the pages and keeps the spans; its reading order, tables, furniture and
/// models don't apply.
///
/// # Errors
/// As [`crate::extract`]: `cancelled` once `cancel` is set (checked before each page),
/// `page-index-out-of-range`, and the document's reading errors.
///
/// Added in 0.2.0.
pub fn hidden_text(
    doc: &Document,
    options: &ExtractOptions,
    cancel: &CancelToken,
) -> Result<Vec<Dropped>, ExtractError> {
    let mut options = options.clone();
    // The order, tables and models don't change which runs are hidden: skip the tree, the
    // table rules and the models. Artifacts stay in the pass, so their hidden stretches show.
    options.reading_order = ReadingOrder::Layout;
    options.tables = TableMode::Off;
    options.drop.furniture = false;
    options.models = Models::default();
    let mut job = extract_job(doc, &options)?;
    job.hidden(doc, cancel)
}
