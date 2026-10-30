//! [`Document`]: the façade's document handle (ADR 0028 F1–F12).

use std::cell::Cell;
use std::marker::PhantomData;
use std::sync::Arc;

use lamina_cos::{ByteSource, OpenOptions};
use lamina_model::PageId;
use lamina_ops::{Op, OpLog, Origin, UndoOutcome};
use lamina_save::{ReadableSink, Save as _, SaveOptions, SaveReport, SinkError, WriteSink};

use crate::error::{Error, Result};
use crate::job::{CancelToken, SaveJob};
use crate::warning::{Warning, WarningCursor};

/// The identity of one [`Document`] value: jobs hold a weak reference and compare pointers,
/// so another document (or this one dropped and a new one opened) never passes for it.
#[derive(Debug, Default)]
pub(crate) struct Instance;

/// An open PDF: read it, edit it with ops (with undo), save it ([ADR 0028] F1–F12).
///
/// Every method takes and returns owned data: pages are addressed by 0-based `index` or by
/// [`PageId`], never by a borrowed view (F1); reads take `&self` and changes `&mut self`
/// (F4); long work is a job ([`Document::save_job`]) with a run-to-completion form that takes
/// a [`CancelToken`] ([`Document::save`], F10). Rust callers who want borrowed views
/// (`lamina_model::Page`) use [`Document::ops`].
///
/// **Threads.** `Send` but not `Sync` ([ADR 0005] §7, [ADR 0028] §2): move it to another
/// thread, or share it behind the lock of the edge that needs one (`Mutex` for `UniFFI` and
/// Tauri, `Rc<RefCell<…>>` in wasm).
///
/// **Revision.** Every committed edit, undo, redo and save bumps a revision counter that
/// every job checks before each step (`document-changed`, ADR 0028 §3).
///
/// [ADR 0005]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0005-document-model.md
/// [ADR 0028]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0028-facade-rules-jobs-and-cancellation.md
#[derive(Debug)]
pub struct Document {
    inner: lamina_ops::Document,
    instance: Arc<Instance>,
    revision: u64,
    /// `!Sync` whatever the inner document becomes: the lock lives in the edge (ADR 0028 §2).
    _not_sync: PhantomData<Cell<()>>,
}

/// One page's facts, owned (F1, F5): its index, its [`PageId`], its displayed size in points
/// (the `CropBox` with `/Rotate` and `/UserUnit` applied, [ADR 0027]) and its rotation.
///
/// [ADR 0027]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PageInfo {
    /// The 0-based page index.
    pub index: u32,
    /// The page's stable ID (`p12.0`), valid across page moves.
    pub id: PageId,
    /// Displayed width in points.
    pub width: f64,
    /// Displayed height in points.
    pub height: f64,
    /// `/Rotate` in degrees: 0, 90, 180 or 270.
    pub rotation: u16,
    /// The page label (`/PageLabels`), when the document defines one. Document text.
    pub label: Option<String>,
}

/// The Info dictionary's text entries (ISO 32000-2 §14.3.3), decoded; document text.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct DocumentInfo {
    /// `/Title`.
    pub title: Option<String>,
    /// `/Author`.
    pub author: Option<String>,
    /// `/Subject`.
    pub subject: Option<String>,
    /// `/Keywords`.
    pub keywords: Option<String>,
    /// `/Creator`.
    pub creator: Option<String>,
    /// `/Producer`.
    pub producer: Option<String>,
}

/// The pages an edit touched (owned, F5).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Edited {
    /// IDs of the pages whose content or attributes changed, in ID order.
    pub touched_pages: Vec<PageId>,
}

/// A finished save: the output and what the writer did.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Saved {
    /// The file written: the whole file, or only the appended revision with
    /// `IncrementalOutput::AppendedOnly` (F7).
    pub bytes: Vec<u8>,
    /// The mode used, bytes written, renumbering and invalidated signatures.
    pub report: SaveReport,
}

impl Document {
    /// Opens a document from its bytes (F3, F7). Nothing is copied: the document parses in
    /// place.
    ///
    /// # Errors
    /// `not-pdf`, `missing-root`, `password-required`, `wrong-password`, `limit-exceeded`,
    /// `strict` and the other reading errors.
    pub fn open_bytes(bytes: Vec<u8>, options: OpenOptions) -> Result<Self> {
        Ok(Self::from(lamina_ops::Document::open(bytes, options)?))
    }

    /// Opens a document from a shared [`ByteSource`]: for edge crates that wrap a host's
    /// source (ranges over HTTP, a file) behind their own concrete constructor (F3). A source
    /// that answers `Pending` makes calls fail with `data-pending`.
    ///
    /// # Errors
    /// As [`Document::open_bytes`], and `data-pending`/`source`.
    pub fn open_source(source: Arc<dyn ByteSource>, options: OpenOptions) -> Result<Self> {
        Ok(Self::from(lamina_ops::Document::open(source, options)?))
    }

    /// The number of pages.
    ///
    /// # Errors
    /// Reading errors (the page tree).
    pub fn page_count(&self) -> Result<u32> {
        Ok(u32::try_from(self.inner.pages()?.len()).unwrap_or(u32::MAX))
    }

    /// Page `index`'s facts.
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors.
    pub fn page_info(&self, index: u32) -> Result<PageInfo> {
        let pages = self.inner.pages()?;
        let count = u32::try_from(pages.len()).unwrap_or(u32::MAX);
        let page = usize::try_from(index)
            .ok()
            .and_then(|i| pages.get(i))
            .ok_or(Error::PageIndexOutOfRange { index, count })?;
        let frame = page.frame()?;
        Ok(PageInfo {
            index,
            id: page.id().clone(),
            width: frame.width().get(),
            height: frame.height().get(),
            rotation: frame.rotate().degrees(),
            label: page.label()?,
        })
    }

    /// The facts of the page with `id` (F1: sub-objects are addressed by value).
    ///
    /// # Errors
    /// `unknown-page` when the document has no such page; reading errors.
    pub fn page_by_id(&self, id: &PageId) -> Result<PageInfo> {
        let index = self
            .inner
            .pages()?
            .index_of(id)
            .ok_or_else(|| Error::from(lamina_ops::Error::UnknownPage(id.clone())))?;
        self.page_info(u32::try_from(index).unwrap_or(u32::MAX))
    }

    /// The Info dictionary's text entries (all `None` without one).
    ///
    /// # Errors
    /// Reading errors.
    pub fn info(&self) -> Result<DocumentInfo> {
        let Some(info) = self.inner.info()? else {
            return Ok(DocumentInfo::default());
        };
        Ok(DocumentInfo {
            title: info.title()?,
            author: info.author()?,
            subject: info.subject()?,
            keywords: info.keywords()?,
            creator: info.creator()?,
            producer: info.producer()?,
        })
    }

    /// A cursor over the warnings raised so far (F8), oldest first.
    #[must_use]
    pub fn warnings(&self) -> WarningCursor {
        WarningCursor::new(self.inner.warnings().into_iter().map(Warning::from).collect())
    }

    /// Whether there are changes no save has written yet.
    #[must_use]
    pub fn has_unsaved_changes(&self) -> bool {
        self.inner.has_unsaved_changes()
    }

    /// Applies `ops` in order as one transaction (ADR 0006): all of them or none. Each op is
    /// a value ([`Op`]: `SetInfo`, `RotatePage`), so the edit is serialisable and replayable;
    /// no closure crosses the boundary (F3).
    ///
    /// # Errors
    /// The first op's error (the transaction is rolled back): `unknown-page`, `direct-page`,
    /// reading errors; `saved-not-rebased`.
    pub fn apply(&mut self, ops: Vec<Op>) -> Result<Edited> {
        let applied = self.inner.apply_all(Origin::Local, ops)?;
        self.bump();
        Ok(Edited {
            touched_pages: applied.touched_pages.into_iter().collect(),
        })
    }

    /// Applies an op log (`{ "v": 1, "ops": [ … ] }`, ADR 0006 §6) as one transaction.
    ///
    /// # Errors
    /// `invalid-op-log` (the log's text only in `data`), `unsupported-version`; as
    /// [`Document::apply`].
    pub fn apply_log(&mut self, json: &str) -> Result<Edited> {
        self.apply(OpLog::from_json(json)?.into_ops())
    }

    /// Undoes the newest transaction; `None` when there is nothing to undo.
    ///
    /// # Errors
    /// `nested-transaction`, `saved-not-rebased`; reading errors (nothing is undone).
    pub fn undo(&mut self) -> Result<Option<UndoOutcome>> {
        let outcome = self.inner.undo()?.map(|r| r.outcome);
        if outcome.is_some() {
            self.bump();
        }
        Ok(outcome)
    }

    /// Redoes the newest undone transaction; `None` when there is nothing to redo.
    ///
    /// # Errors
    /// As [`Document::undo`].
    pub fn redo(&mut self) -> Result<Option<UndoOutcome>> {
        let outcome = self.inner.redo()?.map(|r| r.outcome);
        if outcome.is_some() {
            self.bump();
        }
        Ok(outcome)
    }

    /// Saves the document and rebases it onto the output, running to completion (F10): the
    /// same work as stepping [`Document::save_job`] with [`Budget::UNLIMITED`], stopped by
    /// `cancel` at the same points.
    ///
    /// [`Budget::UNLIMITED`]: crate::Budget::UNLIMITED
    ///
    /// # Errors
    /// As [`SaveJob::step`].
    pub fn save(&mut self, options: &SaveOptions, cancel: &CancelToken) -> Result<Saved> {
        let mut job = SaveJob::new(self, *options, cancel.clone());
        loop {
            if let crate::Step::Done(saved) = job.step(self, crate::Budget::UNLIMITED)? {
                return Ok(saved);
            }
        }
    }

    /// A save as a resumable job, with its own [`CancelToken`] ([`SaveJob::cancel_token`]).
    #[must_use]
    pub fn save_job(&self, options: &SaveOptions) -> SaveJob {
        SaveJob::new(self, *options, CancelToken::new())
    }

    /// A save job that stops when `cancel` is cancelled (one token for several jobs).
    #[must_use]
    pub fn save_job_with(&self, options: &SaveOptions, cancel: &CancelToken) -> SaveJob {
        SaveJob::new(self, *options, cancel.clone())
    }

    /// The `lamina-ops` document underneath, for Rust callers who want the typed, borrowed
    /// views of `lamina-model` (pages, outline, page labels) or `lamina-content`'s
    /// interpreter. Read-only: every change goes through the façade, so the revision moves.
    #[must_use]
    pub const fn ops(&self) -> &lamina_ops::Document {
        &self.inner
    }

    /// The `lamina-ops` document, giving up the façade.
    #[must_use]
    pub fn into_ops(self) -> lamina_ops::Document {
        self.inner
    }

    /// Writes and rebases (the last unit of a [`SaveJob`]). A failed save leaves the
    /// document as it was; otherwise the revision moves on once, as the job's only effect on
    /// the document.
    pub(crate) fn write(&mut self, options: SaveOptions) -> Result<Saved> {
        let mut bytes = Vec::new();
        let result = self.inner.save_and_rebase(Output { bytes: &mut bytes }, &options);
        if matches!(&result, Ok(_) | Err(lamina_save::SaveError::NotRebased { .. })) {
            self.bump();
        }
        Ok(Saved { bytes, report: result? })
    }

    pub(crate) const fn instance(&self) -> &Arc<Instance> {
        &self.instance
    }

    pub(crate) const fn revision(&self) -> u64 {
        self.revision
    }

    const fn bump(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
}

impl From<lamina_ops::Document> for Document {
    fn from(inner: lamina_ops::Document) -> Self {
        Self {
            inner,
            instance: Arc::new(Instance),
            revision: 0,
            _not_sync: PhantomData,
        }
    }
}

/// The façade's sink: the output stays in `bytes` (returned to the caller, F7), and a full
/// rewrite rebases onto a shared copy of it.
struct Output<'a> {
    bytes: &'a mut Vec<u8>,
}

impl WriteSink for Output<'_> {
    fn write_all(&mut self, bytes: &[u8]) -> core::result::Result<(), SinkError> {
        self.bytes.write_all(bytes)
    }

    fn held(&self) -> Option<u64> {
        WriteSink::held(&*self.bytes)
    }
}

impl ReadableSink for Output<'_> {
    fn into_source(self) -> core::result::Result<Arc<dyn ByteSource>, SinkError> {
        Ok(Arc::new(self.bytes.clone()))
    }
}
