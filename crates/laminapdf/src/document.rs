//! [`Document`]: the façade's document handle (ADR 0028 F1–F12).

use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::sync::Arc;

use laminapdf_cos::{ByteSource, OpenOptions};
use laminapdf_model::PageId;
use laminapdf_ops::{Op, OpLog, Origin, Reverted};
use laminapdf_save::{ReadableSink, Save as _, SaveOptions, SaveReport, SinkError, WriteSink};

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
/// a [`CancelToken`] ([`Document::save_with`], F10); [`Document::save`] is that form without
/// one.
///
/// **Threads.** `Send` but not `Sync` ([ADR 0005] §7, [ADR 0028] §2): move it to another
/// thread, or share it behind the lock of the edge that needs one (`Mutex` for `UniFFI` and
/// Tauri, `Rc<RefCell<…>>` in wasm).
///
/// **Revision.** Every committed edit, undo, redo and save bumps a revision counter that
/// every job checks before each step (`document-changed`, ADR 0028 §3).
///
/// [ADR 0005]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0005-document-model.md
/// [ADR 0028]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0028-facade-rules-jobs-and-cancellation.md
#[derive(Debug)]
pub struct Document {
    inner: laminapdf_ops::Document,
    instance: Arc<Instance>,
    revision: u64,
    /// Warnings raised while building it (font embedding), before it existed as a document.
    build_warnings: Vec<Warning>,
    /// Every page's facts, read in one page-tree walk and kept until the revision moves
    /// (a `RefCell` cache, as ADR 0005 §7 allows: the document is `!Sync` anyway).
    pages: RefCell<Option<Vec<PageInfo>>>,
    /// The text extractor (its font cache), kept for the revision it was made at.
    #[cfg(feature = "extract")]
    extractor: RefCell<Option<(u64, laminapdf_text::TextExtractor)>>,
    /// `!Sync` whatever the inner document becomes: the lock lives in the edge (ADR 0028 §2).
    _not_sync: PhantomData<Cell<()>>,
}

/// One page's facts, owned (F1, F5): its index, its [`PageId`], its displayed size in points
/// (the `CropBox` with `/Rotate` and `/UserUnit` applied, [ADR 0027]) and its rotation.
///
/// [ADR 0027]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0027-coordinates-units-and-page-indexing.md
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
    /// Whether the edit changed the document. One that only set values they already had
    /// changes nothing and leaves no step to undo.
    pub changed: bool,
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
        Ok(Self::from(laminapdf_ops::Document::open(bytes, options)?))
    }

    /// Opens a document from a shared [`ByteSource`]: for edge crates that wrap a host's
    /// source (ranges over HTTP, a file) behind their own concrete constructor (F3). A source
    /// that answers `Pending` makes calls fail with `data-pending`.
    ///
    /// # Errors
    /// As [`Document::open_bytes`], and `data-pending`/`source`.
    pub fn open_source(source: Arc<dyn ByteSource>, options: OpenOptions) -> Result<Self> {
        Ok(Self::from(laminapdf_ops::Document::open(source, options)?))
    }

    /// The number of pages.
    ///
    /// # Errors
    /// Reading errors (the page tree).
    pub fn page_count(&self) -> Result<u32> {
        self.with_pages(|pages| Ok(u32::try_from(pages.len()).unwrap_or(u32::MAX)))
    }

    /// Page `index`'s facts.
    ///
    /// # Errors
    /// `page-index-out-of-range`; reading errors.
    pub fn page_info(&self, index: u32) -> Result<PageInfo> {
        self.with_pages(|pages| {
            let count = u32::try_from(pages.len()).unwrap_or(u32::MAX);
            usize::try_from(index)
                .ok()
                .and_then(|i| pages.get(i))
                .cloned()
                .ok_or(Error::PageIndexOutOfRange { index, count })
        })
    }

    /// The text page `index` shows (feature `extract`): glyphs with Unicode and boxes in
    /// page space, runs, lines ([`crate::extract`]). Fonts are parsed once per document and
    /// revision; problems are warnings (`font-invalid`, `cmap-invalid`, `unmapped-glyph`, the
    /// content warnings) in [`Document::warnings`].
    /// Each call walks the page tree once: for every page, use [`Document::page_texts`].
    ///
    /// # Errors
    /// `page-index-out-of-range`; the content limits, `strict`, `data-pending` and the other
    /// reading errors.
    #[cfg(feature = "extract")]
    pub fn page_text(&self, index: u32, options: &crate::extract::TextOptions) -> Result<crate::extract::PageText> {
        let pages = self.inner.pages()?;
        let count = u32::try_from(pages.len()).unwrap_or(u32::MAX);
        let page = usize::try_from(index)
            .ok()
            .and_then(|i| pages.get(i))
            .ok_or(Error::PageIndexOutOfRange { index, count })?;
        let mut cache = self.extractor.borrow_mut();
        if cache.as_ref().is_some_and(|(revision, _)| *revision != self.revision) {
            *cache = None;
        }
        let (_, extractor) = cache.get_or_insert_with(|| (self.revision, laminapdf_text::TextExtractor::new()));
        let text = extractor.page(&self.inner, &page)?;
        Ok(if options.skip_invisible { text.visible() } else { text })
    }

    /// The text of every page, in order ([`Document::page_text`] for each, but one walk of the
    /// page tree and one font cache for all: `page_text` walks the tree on each call, so a
    /// loop over it is quadratic in the page count).
    ///
    /// # Errors
    /// As [`Document::page_text`].
    #[cfg(feature = "extract")]
    pub fn page_texts(&self, options: &crate::extract::TextOptions) -> Result<Vec<crate::extract::PageText>> {
        let pages = self.inner.pages()?;
        let mut cache = self.extractor.borrow_mut();
        if cache.as_ref().is_some_and(|(revision, _)| *revision != self.revision) {
            *cache = None;
        }
        let (_, extractor) = cache.get_or_insert_with(|| (self.revision, laminapdf_text::TextExtractor::new()));
        pages
            .iter()
            .map(|page| {
                let text = extractor.page(&self.inner, &page)?;
                Ok(if options.skip_invisible { text.visible() } else { text })
            })
            .collect()
    }

    /// Every page's facts, in order: one read of the page tree (cached until the next
    /// change), for listing pages without one call per page.
    ///
    /// # Errors
    /// Reading errors.
    pub fn page_infos(&self) -> Result<Vec<PageInfo>> {
        self.with_pages(|pages| Ok(pages.to_vec()))
    }

    /// The facts of the page with `id` (F1: sub-objects are addressed by value).
    ///
    /// # Errors
    /// `unknown-page` when the document has no such page; reading errors.
    pub fn page_by_id(&self, id: &PageId) -> Result<PageInfo> {
        self.with_pages(|pages| {
            pages
                .iter()
                .find(|p| p.id == *id)
                .cloned()
                .ok_or_else(|| Error::from(laminapdf_ops::Error::UnknownPage(id.clone())))
        })
    }

    /// Runs `f` over the cached page facts, reading them first when the cache is empty.
    fn with_pages<T>(&self, f: impl FnOnce(&[PageInfo]) -> Result<T>) -> Result<T> {
        if let Some(pages) = self.pages.borrow().as_deref() {
            return f(pages);
        }
        let read = self.read_pages()?;
        let result = f(&read);
        *self.pages.borrow_mut() = Some(read);
        result
    }

    /// One walk of the page tree: every page's facts.
    fn read_pages(&self) -> Result<Vec<PageInfo>> {
        let pages = self.inner.pages()?;
        pages
            .iter()
            .map(|page| {
                let frame = page.frame()?;
                Ok(PageInfo {
                    index: u32::try_from(page.index()).unwrap_or(u32::MAX),
                    id: page.id().clone(),
                    width: frame.width().get(),
                    height: frame.height().get(),
                    rotation: frame.rotate().degrees(),
                    label: page.label()?,
                })
            })
            .collect()
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

    /// The PDF version the file declares (ISO 32000-2 §7.2.2): the catalog's `/Version` when
    /// it is later than the `%PDF-x.y` header's, else the header's (`"1.7"`); `None` with
    /// neither. A `/Version` that isn't `<digits>.<digits>` is ignored.
    ///
    /// # Errors
    /// Reading errors (the catalog).
    pub fn version(&self) -> Result<Option<String>> {
        let header = self.inner.header_version().filter(|v| version_number(v).is_some());
        let catalog = self.inner.catalog()?;
        let declared = catalog
            .get_name("Version")
            .and_then(|name| std::str::from_utf8(name).ok())
            .filter(|v| version_number(v).is_some());
        let later = match (header, declared) {
            (Some(header), Some(declared)) if version_number(declared) > version_number(header) => Some(declared),
            (Some(header), _) => Some(header),
            (None, declared) => declared,
        };
        Ok(later.map(str::to_owned))
    }

    /// Whether the file is encrypted. An encrypted document opens only when its password, or
    /// the empty user password, decrypts it, so this says only that it was encrypted.
    #[must_use]
    pub fn is_encrypted(&self) -> bool {
        self.inner.encryption().is_some()
    }

    /// Whether the document says it is tagged: its catalog's `/MarkInfo` has `/Marked true`
    /// (ISO 32000-2 §14.7.1). It doesn't check the structure tree.
    ///
    /// # Errors
    /// Reading errors (the catalog, `/MarkInfo`).
    pub fn is_tagged(&self) -> Result<bool> {
        Ok(self.inner.is_tagged()?)
    }

    /// Reads every object the document holds, once, and returns how many there were: for a
    /// validator (`laminapdf check`), since the other calls read only what they need. What a
    /// tolerant read repairs becomes a warning ([`Document::warnings`]).
    ///
    /// # Errors
    /// The first object that can't be read (in strict mode, the first repair too).
    pub fn read_every_object(&self) -> Result<u32> {
        let numbers: Vec<u32> = self.inner.object_numbers().collect();
        for &number in &numbers {
            self.inner.object(number)?;
        }
        Ok(u32::try_from(numbers.len()).unwrap_or(u32::MAX))
    }

    /// A cursor over the warnings raised so far (F8), oldest first: those of building the
    /// document (font embedding), then those of reading it.
    #[must_use]
    pub fn warnings(&self) -> WarningCursor {
        let read = self.inner.warnings().into_iter().map(Warning::from);
        WarningCursor::new(self.build_warnings.iter().cloned().chain(read).collect())
    }

    /// Whether there are changes no save has written yet: an edit since the last save, or a
    /// built document that was never saved (its objects are all in the overlay).
    #[must_use]
    pub fn has_unsaved_changes(&self) -> bool {
        self.inner.has_unsaved_changes() || self.inner.is_dirty()
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
        // An edit that changed nothing (setting a value it already had) commits no record:
        // the document is unchanged, so running jobs stay valid.
        let changed = !applied.undo.is_empty();
        if changed {
            self.bump();
        }
        Ok(Edited {
            touched_pages: applied.touched_pages.into_iter().collect(),
            changed,
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

    /// Undoes the newest transaction: how it went and the pages it touched; `None` when there
    /// is nothing to undo.
    ///
    /// # Errors
    /// `nested-transaction`, `saved-not-rebased`; reading errors (nothing is undone).
    pub fn undo(&mut self) -> Result<Option<Reverted>> {
        let reverted = self.inner.undo()?;
        if reverted.is_some() {
            self.bump();
        }
        Ok(reverted)
    }

    /// Redoes the newest undone transaction: how it went and the pages it touched; `None` when
    /// there is nothing to redo.
    ///
    /// # Errors
    /// As [`Document::undo`].
    pub fn redo(&mut self) -> Result<Option<Reverted>> {
        let reverted = self.inner.redo()?;
        if reverted.is_some() {
            self.bump();
        }
        Ok(reverted)
    }

    /// Saves the document and rebases it onto the output, running to completion: the same
    /// work as stepping [`Document::save_job`] with [`Budget::UNLIMITED`]. To stop it from
    /// another thread, use [`Document::save_with`].
    ///
    /// [`Budget::UNLIMITED`]: crate::Budget::UNLIMITED
    ///
    /// # Errors
    /// As [`SaveJob::step`], except `cancelled`.
    pub fn save(&mut self, options: &SaveOptions) -> Result<Saved> {
        self.save_with(options, &CancelToken::new())
    }

    /// As [`Document::save`], stopped by `cancel` at the same points as a [`SaveJob`] (F10).
    ///
    /// # Errors
    /// As [`SaveJob::step`].
    pub fn save_with(&mut self, options: &SaveOptions, cancel: &CancelToken) -> Result<Saved> {
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

    /// Rebases the document onto `bytes`, the output of a save whose rebase failed
    /// (`not-rebased`: the error carries the output, see [`Error::saved_output`]): ops, undo
    /// and saves are refused until the document is rebased.
    ///
    /// # Errors
    /// `no-pending-save` when no save is waiting; reading errors in `bytes` (the document
    /// keeps waiting).
    pub fn rebase(&mut self, bytes: Vec<u8>) -> Result<()> {
        self.inner.rebase(bytes)?;
        self.bump();
        Ok(())
    }

    /// The `laminapdf-ops` document underneath, for Lamina PDF's own crates and tests that need
    /// the typed, borrowed views of `laminapdf-model` or `laminapdf-content`'s interpreter.
    /// Read-only: every change goes through the façade, so the revision moves. Hidden from the
    /// docs: the façade has one `Document`, and this is a second.
    #[doc(hidden)]
    #[must_use]
    pub const fn ops(&self) -> &laminapdf_ops::Document {
        &self.inner
    }

    /// The `laminapdf-ops` document, giving up the façade. Hidden from the docs, like
    /// [`Document::ops`].
    #[doc(hidden)]
    #[must_use]
    pub fn into_ops(self) -> laminapdf_ops::Document {
        self.inner
    }

    /// Writes and rebases (the last unit of a [`SaveJob`]). A failed save leaves the
    /// document as it was; otherwise the revision moves on once, as the job's only effect on
    /// the document.
    pub(crate) fn write(&mut self, options: SaveOptions) -> Result<Saved> {
        let mut bytes = Vec::new();
        let result = self.inner.save_and_rebase(Output { bytes: &mut bytes }, &options);
        if matches!(&result, Ok(_) | Err(laminapdf_save::SaveError::NotRebased { .. })) {
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

    fn bump(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        *self.pages.get_mut() = None;
        #[cfg(feature = "extract")]
        {
            *self.extractor.get_mut() = None;
        }
    }

    /// A built document with the warnings building it raised.
    #[cfg(feature = "build")]
    pub(crate) fn built(inner: laminapdf_ops::Document, warnings: Vec<Warning>) -> Self {
        Self {
            build_warnings: warnings,
            ..Self::from(inner)
        }
    }
}

impl From<laminapdf_ops::Document> for Document {
    fn from(inner: laminapdf_ops::Document) -> Self {
        Self {
            inner,
            instance: Arc::new(Instance),
            revision: 0,
            build_warnings: Vec::new(),
            pages: RefCell::new(None),
            #[cfg(feature = "extract")]
            extractor: RefCell::new(None),
            _not_sync: PhantomData,
        }
    }
}

/// The façade's sink: the output stays in `bytes` (returned to the caller, F7), and a full
/// rewrite rebases onto a copy of it. Two buffers at the peak are the minimum while both the
/// caller and the document own the file as bytes (F7: `Vec<u8>` out).
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

/// A version string's numbers, `"1.7"` → `(1, 7)`; `None` unless it is `<digits>.<digits>`.
fn version_number(version: &str) -> Option<(u32, u32)> {
    let (major, minor) = version.split_once('.')?;
    // `u32::from_str` takes a leading `+`, which a version never has.
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !(digits(major) && digits(minor)) {
        return None;
    }
    Some((major.parse().ok()?, minor.parse().ok()?))
}
