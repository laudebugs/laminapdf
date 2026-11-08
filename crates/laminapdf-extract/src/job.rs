//! [`ExtractJob`]: an extraction as a resumable job ([ADR 0028] §3): one page per work unit, a
//! [`CancelToken`] checked before each page and inside the XY-Cut recursion.
//!
//! [ADR 0028]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0028-facade-rules-jobs-and-cancellation.md

use laminapdf_model::Document;
use laminapdf_syntax::{Budget, CancelToken, Phase, Step};
use laminapdf_text::TextExtractor;

use crate::error::ExtractError;
use crate::marks::Marks;
use crate::model::{ExtractedSource, Extraction, StructSource};
use crate::options::{ExtractOptions, ReadingOrder};
use crate::page::extract_page;
use crate::page::{PageInput, TreeUse};
use crate::tables::TableMode;
use crate::tagged::TaggedIndex;
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
    /// The structure tree, when the reading order may come from it.
    tree: Option<TaggedIndex>,
    /// Pages the tree ordered, and pages whose tree failed a check.
    tree_pages: (usize, usize),
}

/// Starts an extraction of `doc`: checks the pages asked for, reads whether the document is
/// tagged and, when the reading order may come from it, its structure tree; the pages are read
/// by [`ExtractJob::step`].
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
        ReadingOrder::Auto | ReadingOrder::Tagged | ReadingOrder::Layout => StructSource::XyCut,
    };
    // The tree is read once, here; each page checks its own part of it.
    let tree = match options.reading_order {
        ReadingOrder::Tagged => TaggedIndex::read(doc)?,
        ReadingOrder::Auto if doc.is_tagged()? => TaggedIndex::read(doc)?,
        ReadingOrder::Auto | ReadingOrder::Layout | ReadingOrder::ContentStream => None,
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
        tree,
        tree_pages: (0, 0),
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
            // The table rules and what is painted under the text are collected in the same walk
            // of the content as the text.
            let frame = page.frame()?;
            let mut marks = Marks::new(&frame, self.options.tables != TableMode::Off);
            let text = self.extractor.page_with(doc, &page, &mut marks)?;
            let (edges, backdrops, drawn) = marks.finish();
            let out = extract_page(
                doc,
                &page,
                index,
                &text,
                &PageInput {
                    edges: edges.as_ref(),
                    backdrops: &backdrops,
                    drawn: &drawn,
                    tree: self.tree.as_ref(),
                },
                &self.options,
                &self.cancel,
            )?;
            match out.tree {
                TreeUse::Used => self.tree_pages.0 += 1,
                TreeUse::Ignored => self.tree_pages.1 += 1,
                TreeUse::None => {}
            }
            self.out.pages.push(out.info);
            self.out.blocks.extend(out.blocks);
            self.out.dropped.extend(out.dropped);
            self.next += 1;
        }
        if self.next >= self.todo.len() {
            let mut out = std::mem::take(&mut self.out);
            // Tagged only when the tree gave every page it covers its order.
            if let (true, (used, 0)) = (out.source.struct_source == StructSource::XyCut, self.tree_pages)
                && used > 0
            {
                out.source.struct_source = StructSource::Tagged;
            }
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
