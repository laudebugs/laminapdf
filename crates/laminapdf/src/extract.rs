//! Text extraction (feature `extract`, which `text` implies): [`Document::page_text`](crate::Document::page_text)
//! returns a page's [`PageText`]: glyphs with their text, codes and boxes in page space
//! (ADR 0027), runs with their font, size, MCID and `ActualText`, and [`PageText::lines`].
//!
//! ```
//! # fn sample() -> Vec<u8> { std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/golden/text-std14.pdf")).unwrap() }
//! use laminapdf::{Document, OpenOptions, extract::TextOptions};
//!
//! let doc = Document::open_bytes(sample(), OpenOptions::new())?;
//! let text = doc.page_text(0, &TextOptions::new())?;
//! assert_eq!(text.lines()[0], "Courier: Hello, world! 0123456789");
//! let first = &text.glyphs[0];
//! assert!(first.quad[0].y > first.quad[3].y); // the baseline is below the top: y grows down
//! # Ok::<(), laminapdf::Error>(())
//! ```

pub use laminapdf_extract::{
    Block, BlockKind, Cell, Chunk, ChunkOptions, Coordinates, DropOptions, DropReason, Dropped, ExtractLimits,
    ExtractOptions, ExtractedPage, ExtractedSource, Extraction, Flags, LayoutModel, LayoutRegion, MarkdownOptions,
    ModelError, ModelInfo, ModelRequest, ModelResult, ModelSlot, Models, OcrEngine, OcrWord, PageRaster, PageRenderer,
    ReadingOrder, Region, RegionLabel, Rendered, Span, SplitOn, StructRef, StructSource, TableDetector, TableMode,
    TextLayer, TextLayerStatus, TextLayerThresholds, TextRange, chunks_jsonl, tokens_approx,
};
#[cfg(feature = "reflow")]
pub use laminapdf_extract::{Reflow, ReflowBlock, ReflowChecks, ReflowKind, ReflowRefusal};
pub use laminapdf_text::{Code, Glyph, PageText, PageTextRun, Quad, UserSpaceVector};

use crate::job::{Budget, CancelToken, JobCore, Step};
use crate::{Document, Result};

/// What [`Document::page_text`](crate::Document::page_text) returns.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct TextOptions {
    /// Leave out text drawn invisibly (rendering modes 3 and 7: OCR layers, hidden text).
    /// Default `false`: invisible text is extracted, flagged on its glyphs and runs.
    pub skip_invisible: bool,
}

impl TextOptions {
    /// The defaults: every glyph, visible or not.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Leave out invisible text ([`TextOptions::skip_invisible`]).
    #[must_use]
    pub const fn with_skip_invisible(mut self, skip: bool) -> Self {
        self.skip_invisible = skip;
        self
    }
}

/// Extraction for retrieval and LLMs ([ADR 0030]): blocks in reading order with boxes in
/// top-left points, tables with their cells, hidden text left out and listed.
///
/// ```
/// # fn sample() -> Vec<u8> { std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/golden/text-std14.pdf")).unwrap() }
/// use laminapdf::{Document, OpenOptions, extract::{ExtractOptions, MarkdownOptions}};
///
/// let doc = Document::open_bytes(sample(), OpenOptions::new())?;
/// let extraction = doc.extract(&ExtractOptions::new())?;
/// let markdown = extraction.to_markdown(&MarkdownOptions::new());
/// assert!(markdown.text.starts_with("Courier: Hello, world!"));
/// let json = doc.extract_json(&extraction, &markdown, None);
/// assert!(json.starts_with(r#"{"schema":"laminapdf.extract/1""#));
/// # Ok::<(), laminapdf::Error>(())
/// ```
///
/// [ADR 0030]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0030-extraction-output-and-model-plugins.md
impl Document {
    /// Extracts the pages `options` asks for, to the end.
    ///
    /// # Errors
    /// `page-index-out-of-range`; the reading errors (`strict`, the limits, `data-pending`).
    pub fn extract(&self, options: &ExtractOptions) -> Result<Extraction> {
        self.extract_with(options, &CancelToken::new())
    }

    /// [`Document::extract`], stopped by `cancel` from another thread (`cancelled`, and no
    /// partial result).
    ///
    /// # Errors
    /// As [`Document::extract`], and `cancelled`.
    pub fn extract_with(&self, options: &ExtractOptions, cancel: &CancelToken) -> Result<Extraction> {
        Ok(laminapdf_extract::extract(self.ops(), options, cancel)?)
    }

    /// The document for reader mode: its blocks in the structure tree's order, figures,
    /// formulas and tables as crops of their pages with their alternate text, or why its tree
    /// can't be trusted ([`Reflow::reason`]). An untagged document is refused as `untagged`.
    ///
    /// # Errors
    /// `cancelled`; reading errors (the page tree, the structure tree, a page's content).
    #[cfg(feature = "reflow")]
    pub fn reflow(&self, cancel: &CancelToken) -> Result<Reflow> {
        Ok(laminapdf_extract::reflow(self.ops(), cancel)?)
    }

    /// An extraction as a job: [`ExtractionJob::step`] does one page per work unit.
    ///
    /// # Errors
    /// `page-index-out-of-range` for a page `options` names that the document lacks.
    pub fn extract_job(&self, options: &ExtractOptions) -> Result<ExtractionJob> {
        let job = laminapdf_extract::extract_job(self.ops(), options)?;
        Ok(ExtractionJob {
            core: JobCore::new(self, job.cancel_token()),
            job,
        })
    }

    /// `extraction` as `laminapdf.extract/1` JSON with this document's warnings and, when
    /// given, `chunks`. `markdown` is the extraction's Markdown, which each block's `textRange`
    /// points into.
    #[must_use]
    pub fn extract_json(&self, extraction: &Extraction, markdown: &Rendered, chunks: Option<Vec<Chunk>>) -> String {
        let warnings = self
            .warnings()
            .next_batch(u32::MAX)
            .iter()
            .filter_map(|w| serde_json::to_value(w).ok())
            .collect();
        let mut options = laminapdf_extract::JsonOptions::new().with_warnings(warnings);
        if let Some(chunks) = chunks {
            options = options.with_chunks(chunks);
        }
        laminapdf_extract::to_json(extraction, markdown, &options)
    }
}

/// An extraction stepped a page at a time ([ADR 0028] §3): from [`Document::extract_job`].
/// It wraps `laminapdf-extract`'s job with the façade's checks: an edit, undo, redo or save of
/// the document between steps ends it with `document-changed`.
///
/// [ADR 0028]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0028-facade-rules-jobs-and-cancellation.md
#[derive(Debug)]
pub struct ExtractionJob {
    core: JobCore,
    job: laminapdf_extract::ExtractJob,
}

impl ExtractionJob {
    /// The token that cancels this job, from any thread.
    #[must_use]
    pub fn cancel_token(&self) -> CancelToken {
        self.core.cancel().clone()
    }

    /// Extracts up to `budget` pages: [`Step::Progress`] with the pages done, or
    /// [`Step::Done`] with the extraction.
    ///
    /// # Errors
    /// `document-changed`, `job-finished`, `cancelled`, and the reading errors. After an error
    /// the job is finished.
    pub fn step(&mut self, doc: &Document, budget: Budget) -> Result<Step<Extraction>> {
        self.core.begin(doc)?;
        let outcome = self.job.step(doc.ops(), budget).map_err(Into::into);
        if matches!(outcome, Ok(Step::Done(_))) {
            self.core.finish();
        }
        self.core.ended(outcome)
    }

    /// The model result the job waits for, when the host runs the model
    /// ([`ModelSlot::Host`], or a plug-in with no [`PageRenderer`]): render the page, run the
    /// model and hand the result to [`ExtractionJob::provide`]; until then a step does nothing.
    #[must_use]
    pub fn request(&self) -> Option<&ModelRequest> {
        self.job.request().filter(|_| !self.core.is_finished())
    }

    /// Hands the job the result of its open [`ExtractionJob::request`].
    ///
    /// # Errors
    /// `model-result-unexpected` when nothing was asked for or `result` is the other model's;
    /// `job-finished` after the job ended.
    pub fn provide(&mut self, result: ModelResult) -> Result<()> {
        if self.core.is_finished() {
            return Err(crate::Error::JobFinished);
        }
        Ok(self.job.provide(result)?)
    }
}
