//! Pages across documents (E2 plan §2.1): source documents for
//! [`InsertPagesFrom`](crate::InsertPagesFrom) and [`ReplacePages`](crate::ReplacePages), and new
//! documents made from pages, in one call or as a job.

use laminapdf_ops::SourceId;

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
    /// Added in 0.2.0.
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
    /// Added in 0.2.0.
    pub fn source_pages(&self, source: &SourceId) -> Result<Vec<PageId>> {
        let doc = self.inner.source_document(source)?;
        Ok(doc.pages()?.iter().map(|p| p.id().clone()).collect())
    }

    /// A new document of `pages` of this one (each once, in the order given), as it stands
    /// now, edits included: the pages with their annotations, the links between them, the
    /// named destinations those links use, their form fields (signature fields unsigned) and
    /// their page labels. Links to pages left behind are removed, and the pages come without
    /// tags. Saving the new document writes a whole file holding only what its pages reach,
    /// with no password.
    ///
    /// # Errors
    /// `unknown-page`; reading errors; limits passed by the copy.
    ///
    /// Added in 0.2.0.
    pub fn extract_pages(&self, pages: &[PageId]) -> Result<Self> {
        Ok(Self::from(self.inner.extract_pages(pages)?))
    }

    /// One new document per range of `ranges` (each a list of pages, as
    /// [`Document::extract_pages`] takes), in order.
    ///
    /// # Errors
    /// As [`Document::extract_pages`], for the first range that fails.
    ///
    /// Added in 0.2.0.
    pub fn export_ranges(&self, ranges: &[Vec<PageId>]) -> Result<Vec<Self>> {
        self.export_ranges_with(ranges, &CancelToken::new())
    }

    /// [`Document::export_ranges`], stopped between two ranges once `cancel` is cancelled.
    ///
    /// # Errors
    /// `cancelled`; as [`Document::export_ranges`].
    ///
    /// Added in 0.2.0.
    pub fn export_ranges_with(&self, ranges: &[Vec<PageId>], cancel: &CancelToken) -> Result<Vec<Self>> {
        let mut out = Vec::with_capacity(ranges.len());
        for pages in ranges {
            check(cancel)?;
            out.push(self.extract_pages(pages)?);
        }
        Ok(out)
    }

    /// [`Document::export_ranges`] as a job ([`ExportJob`]), stepped one range per unit.
    ///
    /// Added in 0.2.0.
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
/// Added in 0.2.0.
#[derive(Debug)]
pub struct ExportJob {
    core: JobCore,
    ranges: Vec<Vec<PageId>>,
    done: Vec<Document>,
}

impl ExportJob {
    /// The job's cancel token (cancel it from anywhere that holds a clone).
    ///
    /// Added in 0.2.0.
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
    /// Added in 0.2.0.
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
