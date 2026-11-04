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

/// An extraction in progress. Step it with the document it was made from until it returns
/// [`Step::Done`]; an error or a cancellation ends it with no partial result.
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
    let todo = options.pages.clone().unwrap_or_else(|| (0..count).collect());
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

impl ExtractJob {
    /// A token that cancels this job from another thread or callback.
    #[must_use]
    pub fn cancel_token(&self) -> CancelToken {
        self.cancel.clone()
    }

    /// Extracts up to `budget` pages (at least one), then returns the progress (pages done of
    /// the pages asked for) or, once every page is done, the extraction.
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
            let text = self.extractor.page(doc, &page)?;
            let out = extract_page(doc, &page, index, &text, &self.options, &self.cancel)?;
            self.out.pages.push(out.info);
            self.out.blocks.extend(out.blocks);
            self.out.dropped.extend(out.dropped);
            self.next += 1;
        }
        if self.next >= self.todo.len() {
            return Ok(Step::Done(std::mem::take(&mut self.out)));
        }
        Ok(Step::Progress {
            done: u32::try_from(self.next).unwrap_or(u32::MAX),
            total: Some(u32::try_from(self.todo.len()).unwrap_or(u32::MAX)),
            phase: Phase::Extract,
        })
    }
}
