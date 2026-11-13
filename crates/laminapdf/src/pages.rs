//! Pages across documents (E2 plan §2.1): source documents for
//! [`InsertPagesFrom`](crate::InsertPagesFrom), [`ReplacePages`](crate::ReplacePages) and
//! [`MergeDocuments`](crate::MergeDocuments), and new documents made from pages (extracted,
//! exported, split or merged), in one call or as a job.

use std::collections::VecDeque;

use laminapdf_ops::{MergeDocuments, MergeInput, MergeOptions, MergeSource, Origin, SourceId, SplitBy};

use crate::PageId;
use crate::document::Document;
use crate::error::Result;
use crate::job::{Budget, CancelToken, JobCore, Phase, Step, check};

impl Document {
    /// Adds another PDF as a source document for the ops that copy pages from one, and returns
    /// its ID: the SHA-256 of its bytes, so the same file gets the same ID on every device and
    /// is added once. An encrypted file opens with `password`; the password is used here only,
    /// never in an op. Adding a source changes nothing to save or undo.
    ///
    /// # Errors
    /// `wrong-password`, `password-required`, and the errors of a file that doesn't open.
    ///
    /// Added in 0.3.0.
    pub fn add_source(&mut self, bytes: Vec<u8>, password: Option<&str>) -> Result<SourceId> {
        Ok(self.inner.add_source(bytes, password)?)
    }

    /// The page IDs of the source document `source`, in order: what
    /// [`InsertPagesFrom`](crate::InsertPagesFrom) and [`ReplacePages`](crate::ReplacePages)
    /// name its pages by.
    ///
    /// # Errors
    /// `unknown-source` for a source this document wasn't given; reading errors.
    ///
    /// Added in 0.3.0.
    pub fn source_pages(&self, source: &SourceId) -> Result<Vec<PageId>> {
        let doc = self.inner.source_document(source)?;
        Ok(doc.pages()?.iter().map(|p| p.id().clone()).collect())
    }

    /// A new document of `pages` of this one (each once, in the order given), as it stands
    /// now, edits included: the pages with their annotations, the links between them, the
    /// named destinations those links use, their form fields (signature fields unsigned) and
    /// their page labels, and their tags when this document is tagged. Links to pages left
    /// behind are removed. Saving the new document writes a whole file holding only what its pages reach,
    /// with no password.
    ///
    /// # Errors
    /// `unknown-page`; reading errors; limits passed by the copy.
    ///
    /// Added in 0.3.0.
    pub fn extract_pages(&self, pages: &[PageId]) -> Result<Self> {
        Ok(Self::from(self.inner.extract_pages(pages)?))
    }

    /// One new document per range of `ranges` (each a list of pages, as
    /// [`Document::extract_pages`] takes), in order.
    ///
    /// # Errors
    /// As [`Document::extract_pages`], for the first range that fails.
    ///
    /// Added in 0.3.0.
    pub fn export_ranges(&self, ranges: &[Vec<PageId>]) -> Result<Vec<Self>> {
        self.export_ranges_with(ranges, &CancelToken::new())
    }

    /// [`Document::export_ranges`], stopped between two ranges once `cancel` is cancelled.
    ///
    /// # Errors
    /// `cancelled`; as [`Document::export_ranges`].
    ///
    /// Added in 0.3.0.
    pub fn export_ranges_with(&self, ranges: &[Vec<PageId>], cancel: &CancelToken) -> Result<Vec<Self>> {
        let mut out = Vec::with_capacity(ranges.len());
        for pages in ranges {
            check(cancel)?;
            out.push(self.extract_pages(pages)?);
        }
        Ok(out)
    }

    /// This document cut into new documents as `by` says: parts of a number of pages, lists of
    /// pages, or a part from each bookmark at a depth to the next (the pages before the first
    /// making a part of their own). Each part is made as [`Document::extract_pages`] makes a
    /// new document, with the bookmarks that go to its pages, so a part cut at a chapter's
    /// bookmark has that chapter's bookmarks.
    ///
    /// ```
    /// use std::num::NonZeroUsize;
    /// use laminapdf::{Document, SplitBy};
    ///
    /// # let bytes = include_bytes!("../../../fixtures/pages/chapters.pdf").to_vec();
    /// let doc = Document::open_bytes(bytes, Default::default())?;
    /// let chapters = doc.split(&SplitBy::Bookmarks { level: NonZeroUsize::MIN })?;
    /// assert_eq!(chapters.len(), 3);
    /// # Ok::<(), laminapdf::Error>(())
    /// ```
    ///
    /// # Errors
    /// As [`Document::extract_pages`], for the first part that fails.
    ///
    /// Added in 0.3.0.
    pub fn split(&self, by: &SplitBy) -> Result<Vec<Self>> {
        self.split_with(by, &CancelToken::new())
    }

    /// [`Document::split`], stopped between two parts once `cancel` is cancelled.
    ///
    /// # Errors
    /// `cancelled`; as [`Document::split`].
    ///
    /// Added in 0.3.0.
    pub fn split_with(&self, by: &SplitBy, cancel: &CancelToken) -> Result<Vec<Self>> {
        self.export_ranges_with(&self.inner.split_ranges(by)?, cancel)
    }

    /// [`Document::split`] as a job ([`ExportJob`]), stepped one part per unit.
    ///
    /// # Errors
    /// Errors reading the page tree or the outline, which say where the parts begin.
    ///
    /// Added in 0.3.0.
    pub fn split_job(&self, by: &SplitBy) -> Result<ExportJob> {
        Ok(self.export_job(self.inner.split_ranges(by)?))
    }

    /// A new document of `inputs`' pages, in order, with what belongs to each file: its
    /// bookmarks (per file, under a bookmark named after its title, unless `options` says
    /// otherwise), its form fields (renamed `<partial>#<n>` when a name is already used, or
    /// linked on request), its layers and page labels. Signature fields are left off, since a
    /// signature covers the file it was made in. The result is tagged when every file is,
    /// each file's tags under a `Part` element; otherwise it comes without tags.
    ///
    /// # Errors
    /// `no-pages-left` for no inputs; `wrong-password`, `password-required` and the errors of a
    /// file that doesn't open; limits passed by the copy.
    ///
    /// Added in 0.3.0.
    pub fn merge(inputs: Vec<MergeInput>, options: MergeOptions) -> Result<Self> {
        Self::merge_with(inputs, options, &CancelToken::new())
    }

    /// [`Document::merge`], stopped between two files once `cancel` is cancelled.
    ///
    /// # Errors
    /// `cancelled`; as [`Document::merge`].
    ///
    /// Added in 0.3.0.
    pub fn merge_with(inputs: Vec<MergeInput>, options: MergeOptions, cancel: &CancelToken) -> Result<Self> {
        let mut job = Self::merge_job(inputs, options);
        job.cancel = cancel.clone();
        loop {
            if let Step::Done(doc) = job.step(Budget::units(u32::MAX))? {
                return Ok(doc);
            }
        }
    }

    /// [`Document::merge`] as a job ([`MergeJob`]): one unit per file opened, and one for the
    /// merge.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub fn merge_job(inputs: Vec<MergeInput>, options: MergeOptions) -> MergeJob {
        MergeJob {
            cancel: CancelToken::new(),
            total: u32::try_from(inputs.len()).unwrap_or(u32::MAX).saturating_add(1),
            inputs: inputs.into(),
            sources: Vec::new(),
            options,
            doc: None,
            finished: false,
        }
    }

    /// [`Document::export_ranges`] as a job ([`ExportJob`]), stepped one range per unit.
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub fn export_job(&self, ranges: Vec<Vec<PageId>>) -> ExportJob {
        ExportJob {
            core: JobCore::new(self, CancelToken::new()),
            ranges,
            done: Vec::new(),
        }
    }
}

/// Exporting ranges of pages as new documents, as a resumable job: created by
/// [`Document::export_job`], stepped with the document it was created from.
///
/// **Units:** each range is 1 unit ([`Phase::Write`]): its pages are copied in one call, so
/// that links between them keep pointing at the copies, and a step overshoots its budget by
/// at most one range. Every step first checks that the job hasn't finished
/// (`job-finished`), its token (`cancelled`), and that the document is the same instance at
/// the same revision (`document-changed`).
///
/// ```
/// use laminapdf::{Budget, Document, Step};
///
/// # let bytes = include_bytes!("../../../fixtures/pages/references.pdf").to_vec();
/// let doc = Document::open_bytes(bytes, Default::default())?;
/// let ids: Vec<_> = (0..doc.page_count()?).map(|i| doc.page_info(i).map(|p| p.id)).collect::<Result<_, _>>()?;
/// let mut job = doc.export_job(vec![ids[..2].to_vec(), ids[2..].to_vec()]);
/// let parts = loop {
///     if let Step::Done(parts) = job.step(&doc, Budget::units(1))? {
///         break parts;
///     }
/// };
/// assert_eq!(parts.len(), 2);
/// # Ok::<(), laminapdf::Error>(())
/// ```
///
/// Added in 0.3.0.
#[derive(Debug)]
pub struct ExportJob {
    core: JobCore,
    ranges: Vec<Vec<PageId>>,
    done: Vec<Document>,
}

impl ExportJob {
    /// The job's cancel token (cancel it from anywhere that holds a clone).
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub fn cancel_token(&self) -> CancelToken {
        self.core.cancel().clone()
    }

    /// Exports at most `budget` ranges (at least one), then returns progress or the documents.
    ///
    /// # Errors
    /// `job-finished`, `cancelled` or `document-changed`, and the errors of
    /// [`Document::extract_pages`]; each finishes the job.
    ///
    /// Added in 0.3.0.
    pub fn step(&mut self, doc: &Document, budget: Budget) -> Result<Step<Vec<Document>>> {
        self.core.begin(doc)?;
        let outcome = self.run(doc, budget.get().max(1));
        if matches!(outcome, Ok(Step::Done(_))) {
            self.core.finish();
        }
        self.core.ended(outcome)
    }

    fn run(&mut self, doc: &Document, mut units: u32) -> Result<Step<Vec<Document>>> {
        while let Some(pages) = self.ranges.get(self.done.len()) {
            check(self.core.cancel())?;
            if units == 0 {
                return Ok(Step::Progress {
                    done: u32::try_from(self.done.len()).unwrap_or(u32::MAX),
                    total: Some(u32::try_from(self.ranges.len()).unwrap_or(u32::MAX)),
                    phase: Phase::Write,
                });
            }
            units -= 1;
            let part = doc.extract_pages(pages)?;
            self.done.push(part);
        }
        Ok(Step::Done(std::mem::take(&mut self.done)))
    }
}

/// Merging files into a new document, as a resumable job: created by [`Document::merge_job`].
/// It owns the document it builds, so no document is passed to [`MergeJob::step`].
///
/// **Units:** each file is 1 unit ([`Phase::Parse`]: it is opened and kept as a source), and the
/// merge itself 1 more ([`Phase::Write`]: every file's pages are copied in one edit, so a
/// step overshoots its budget by at most one merge). Every step first checks that the job hasn't
/// finished (`job-finished`) and its token (`cancelled`).
///
/// ```
/// use laminapdf::{Budget, Document, MergeInput, MergeOptions, Step};
///
/// # let chapters = include_bytes!("../../../fixtures/pages/chapters.pdf").to_vec();
/// # let labels = include_bytes!("../../../fixtures/pages/labels.pdf").to_vec();
/// let inputs = vec![MergeInput::new(chapters), MergeInput::new(labels)];
/// let mut job = Document::merge_job(inputs, MergeOptions::default());
/// let merged = loop {
///     if let Step::Done(doc) = job.step(Budget::units(1))? {
///         break doc;
///     }
/// };
/// assert_eq!(merged.page_count()?, 22);
/// # Ok::<(), laminapdf::Error>(())
/// ```
///
/// Added in 0.3.0.
#[derive(Debug)]
pub struct MergeJob {
    cancel: CancelToken,
    total: u32,
    inputs: VecDeque<MergeInput>,
    sources: Vec<MergeSource>,
    options: MergeOptions,
    doc: Option<laminapdf_ops::Document>,
    finished: bool,
}

impl MergeJob {
    /// The job's cancel token (cancel it from anywhere that holds a clone).
    ///
    /// Added in 0.3.0.
    #[must_use]
    pub fn cancel_token(&self) -> CancelToken {
        self.cancel.clone()
    }

    /// Opens at most `budget` files (at least one), or merges them once all are open, then
    /// returns progress or the new document.
    ///
    /// # Errors
    /// `job-finished` or `cancelled`, and the errors of [`Document::merge`]; each finishes the
    /// job.
    ///
    /// Added in 0.3.0.
    pub fn step(&mut self, budget: Budget) -> Result<Step<Document>> {
        if self.finished {
            return Err(crate::Error::JobFinished);
        }
        let outcome = self.run(budget.get().max(1));
        if !matches!(outcome, Ok(Step::Progress { .. })) {
            self.finished = true;
        }
        outcome
    }

    fn run(&mut self, mut units: u32) -> Result<Step<Document>> {
        check(&self.cancel)?;
        let doc = match &mut self.doc {
            Some(doc) => doc,
            slot => slot.insert(laminapdf_ops::Document::empty()?),
        };
        while let Some(input) = self.inputs.front() {
            if units == 0 {
                return Ok(Step::Progress {
                    done: u32::try_from(self.sources.len()).unwrap_or(u32::MAX),
                    total: Some(self.total),
                    phase: Phase::Parse,
                });
            }
            check(&self.cancel)?;
            units -= 1;
            let id = doc.add_source(std::sync::Arc::clone(&input.bytes), input.password.as_deref())?;
            let mut source = MergeSource::new(id);
            if let Some(title) = &input.title {
                source = source.with_title(title.clone());
            }
            self.sources.push(source);
            self.inputs.pop_front();
        }
        if self.sources.is_empty() {
            return Err(laminapdf_ops::Error::NoPagesLeft.into());
        }
        if units == 0 {
            return Ok(Step::Progress {
                done: self.total.saturating_sub(1),
                total: Some(self.total),
                phase: Phase::Write,
            });
        }
        check(&self.cancel)?;
        let op = MergeDocuments::new(std::mem::take(&mut self.sources), laminapdf_model::PagePosition::End)
            .with_options(self.options);
        doc.apply_all(Origin::Local, [op.into()])?;
        let done = self.doc.take().map(Document::from);
        done.map(Step::Done).ok_or(crate::Error::JobFinished)
    }
}
