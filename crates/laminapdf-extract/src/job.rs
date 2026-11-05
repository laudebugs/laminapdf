//! [`ExtractJob`]: an extraction as a resumable job ([ADR 0028] §3): one page per work unit, a
//! [`CancelToken`] checked before each page and inside the XY-Cut recursion.
//!
//! [ADR 0028]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0028-facade-rules-jobs-and-cancellation.md

use laminapdf_model::Document;
use laminapdf_syntax::{Budget, CancelToken, Phase, Step};
use laminapdf_text::TextExtractor;

use crate::error::ExtractError;
use crate::model::{ExtractedSource, Extraction, StructSource};
use crate::options::{ExtractOptions, ReadingOrder};
use crate::page::extract_page;
use crate::tables::{Collector, TableMode};
use crate::{furniture, headings, lists};

/// An extraction in progress. Step it with the document it was made from until it returns
/// [`Step::Done`]; an error or a cancellation ends it with no partial result. The core document
/// has no revision to compare, so the job trusts the caller to pass that document; the façade's
/// job checks it (`document-changed`, ADR 0028 §3).
#[derive(Debug)]
pub struct ExtractJob {
    options: ExtractOptions,
    cancel: CancelToken,
    /// The page indices to extract, in order.
    todo: Vec<usize>,
    /// How many of `todo` are done.
    next: usize,
    extractor: TextExtractor,
    out: Extraction,
    finished: bool,
}

/// Starts an extraction of `doc`: checks the pages asked for and reads whether the document is
/// tagged; the pages are read by [`ExtractJob::step`].
///
/// # Errors
/// `page-index-out-of-range` for a page past the last; reading errors (the page tree, the
/// catalog).
pub fn extract_job(doc: &Document, options: &ExtractOptions) -> Result<ExtractJob, ExtractError> {
    let count = doc.pages()?.len();
    let mut todo = options.pages.clone().unwrap_or_else(|| (0..count).collect());
    // A page asked for twice is extracted once, where it first appears, so block IDs stay unique.
    let mut seen = std::collections::BTreeSet::new();
    todo.retain(|&i| seen.insert(i));
    if let Some(&index) = todo.iter().find(|&&i| i >= count) {
        return Err(ExtractError::PageIndexOutOfRange { index, count });
    }
    let struct_source = match options.reading_order {
        ReadingOrder::ContentStream => StructSource::Content,
        ReadingOrder::Auto | ReadingOrder::Layout => StructSource::XyCut,
    };
    let source = ExtractedSource {
        sha256: None,
        page_count: count,
        tagged: doc.is_tagged()?,
        struct_source,
    };
    Ok(ExtractJob {
        options: options.clone(),
        cancel: CancelToken::new(),
        todo,
        next: 0,
        extractor: TextExtractor::new(),
        out: Extraction {
            source,
            ..Extraction::default()
        },
        finished: false,
    })
}

/// Extracts `doc` in one call, stopped by `cancel` (ADR 0028 F10).
///
/// # Errors
/// As [`extract_job`] and [`ExtractJob::step`].
pub fn extract(doc: &Document, options: &ExtractOptions, cancel: &CancelToken) -> Result<Extraction, ExtractError> {
    let mut job = extract_job(doc, options)?;
    job.cancel = cancel.clone();
    loop {
        if let Step::Done(extraction) = job.step(doc, Budget::UNLIMITED)? {
            return Ok(extraction);
        }
    }
}

/// The rules that need every page: running headers and footers leave the blocks, then
/// headings (whose body size is the document's) and list items are marked, and each page's
/// blocks are numbered again.
fn finish(out: &mut Extraction, options: &ExtractOptions) {
    if options.drop.furniture {
        furniture::remove(out, options.drop.keep_spans);
    }
    headings::assign(&mut out.blocks);
    lists::assign(&mut out.blocks);
    for page in out.blocks.chunk_by_mut(|a, b| a.page == b.page) {
        for (n, block) in page.iter_mut().enumerate() {
            block.id = format!("p{}-b{n}", block.page);
        }
    }
}

impl ExtractJob {
    /// A token that cancels this job from another thread or callback.
    #[must_use]
    pub fn cancel_token(&self) -> CancelToken {
        self.cancel.clone()
    }

    /// Extracts up to `budget` pages (at least one), then returns the progress (pages done of
    /// the pages asked for) or, once every page is done, the extraction. Each step walks the
    /// page tree once, so step a long document with budgets of many pages, not one.
    ///
    /// # Errors
    /// `cancelled` once the token is cancelled; `job-finished` after the job returned its
    /// result or an error; reading errors (`data-pending`, `strict`, …). Every error ends the
    /// job.
    pub fn step(&mut self, doc: &Document, budget: Budget) -> Result<Step<Extraction>, ExtractError> {
        if self.finished {
            return Err(ExtractError::JobFinished);
        }
        let result = self.run(doc, budget);
        if !matches!(result, Ok(Step::Progress { .. })) {
            self.finished = true;
        }
        result
    }

    fn run(&mut self, doc: &Document, budget: Budget) -> Result<Step<Extraction>, ExtractError> {
        let pages = doc.pages()?;
        let count = pages.len();
        for _ in 0..budget.get().max(1) {
            let Some(&index) = self.todo.get(self.next) else {
                break;
            };
            if self.cancel.is_cancelled() {
                return Err(ExtractError::Cancelled);
            }
            let page = pages
                .get(index)
                .ok_or(ExtractError::PageIndexOutOfRange { index, count })?;
            // The table rules are collected in the same walk of the content as the text.
            let (text, edges) = if self.options.tables == TableMode::Off {
                (self.extractor.page(doc, &page)?, None)
            } else {
                let frame = page.frame()?;
                let mut rules = Collector::new(&frame);
                let text = self.extractor.page_with(doc, &page, &mut rules)?;
                (text, Some(rules.finish()))
            };
            let out = extract_page(doc, &page, index, &text, edges.as_ref(), &self.options, &self.cancel)?;
            self.out.pages.push(out.info);
            self.out.blocks.extend(out.blocks);
            self.out.dropped.extend(out.dropped);
            self.next += 1;
        }
        if self.next >= self.todo.len() {
            let mut out = std::mem::take(&mut self.out);
            finish(&mut out, &self.options);
            return Ok(Step::Done(out));
        }
        Ok(Step::Progress {
            done: u32::try_from(self.next).unwrap_or(u32::MAX),
            total: Some(u32::try_from(self.todo.len()).unwrap_or(u32::MAX)),
            phase: Phase::Extract,
        })
    }
}
