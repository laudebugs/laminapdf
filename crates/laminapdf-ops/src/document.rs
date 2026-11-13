//! [`Document`]: a model document that can only be changed through transactions.

use std::collections::{BTreeMap, BTreeSet};
use std::ops;
use std::sync::Arc;

use laminapdf_cos::{OpenOptions, Reserved, Source};

use crate::blob::{BlobStore, SourceId};
use crate::error::{Error, Result};
use crate::history::{Entry, History, Reverted, UndoOutcome};
use crate::op::Op;
use crate::pending::PendingChanges;
use crate::record::{UndoRecord, revert};
use crate::tx::{Applied, Origin, Tx, TxState};

/// An editable PDF ([ADR 0006]): a [`laminapdf_model::Document`] (it dereferences to one, for
/// reading) whose only write path is a transaction.
///
/// * [`Document::transact`] runs a closure as one atomic transaction; the handle form
///   ([`begin`](Document::begin) / [`apply`](Document::apply) /
///   [`commit`](Document::commit) / [`rollback`](Document::rollback)) is the same thing
///   across calls, for bindings where a closure can't cross the boundary.
/// * [`Document::undo`] / [`redo`](Document::redo) revert local transactions, computing the
///   inverse from the current state.
/// * Two dirty levels (ADR 0006 §7): [`has_unsaved_changes`](Document::has_unsaved_changes)
///   (local transactions since the last save) and the COS level,
///   [`laminapdf_cos::Document::is_dirty`] (the overlay is non-empty).
///
/// There is no `&mut` access to the model or COS document, so nothing changes it outside a
/// transaction:
///
/// ```compile_fail
/// fn sneak(doc: &mut laminapdf_ops::Document) {
///     doc.cos_mut().edit(); // no DerefMut: the model document is read-only from here
/// }
/// ```
///
/// Like the documents below it, it is `Send` but `!Sync`.
///
/// [ADR 0006]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0006-op-log-and-undo.md
///
/// Added in 0.2.0.
#[derive(Debug)]
pub struct Document {
    model: laminapdf_model::Document,
    history: History,
    open: Option<TxState>,
    /// Set by a save whose output the document has not been rebased onto yet.
    unrebased: Option<PendingRebase>,
    /// The op classes of the changes since the last rebase (for the save planner).
    pending: PendingChanges,
    /// The bytes ops name by `BlobId`: source documents for now.
    blobs: BlobStore,
    /// The source documents added in this session, opened (and decrypted).
    sources: BTreeMap<SourceId, laminapdf_model::Document>,
}

/// What a save that wrote its bytes means for the rebase onto them
/// ([`Document::saved_not_rebased`]).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PendingRebase {
    /// The output is the old base plus an appended revision that freed nothing: every object
    /// the undo history names still exists under its number and generation, so the history
    /// and the numbers reserved in the session are kept.
    KeepHistory,
    /// The output dropped or freed objects (a full rewrite, or an incremental save that
    /// deleted objects): undo could restore references to objects that no longer exist, so
    /// the history is cleared and reserved numbers are released.
    ClearHistory,
}

/// Where a document stands relative to its last save ([ADR 0005] §3).
///
/// [ADR 0005]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0005-document-model.md
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SaveState {
    /// The overlay is empty: the base is the last opened or saved bytes.
    Clean,
    /// The overlay holds changes.
    Dirty,
    /// Saved to a sink the core can't read back and not rebased yet: reads work; ops,
    /// undo, redo and saves are refused until [`Document::rebase`].
    SavedNotRebased,
}

// ADR 0005 §7: a document can move between threads.
const _: () = {
    const fn assert_send<T: Send>() {}
    assert_send::<Document>();
};

impl From<laminapdf_model::Document> for Document {
    /// Wraps `model`. An overlay that already holds changes was made without ops, so what
    /// kind of change it holds is unknown: [`PendingChanges::unknown`] (every class, and a
    /// full rewrite needed), never empty.
    fn from(model: laminapdf_model::Document) -> Self {
        let pending = if model.is_dirty() {
            PendingChanges::unknown()
        } else {
            PendingChanges::default()
        };
        Self {
            model,
            history: History::default(),
            open: None,
            unrebased: None,
            pending,
            blobs: BlobStore::default(),
            sources: BTreeMap::new(),
        }
    }
}

impl ops::Deref for Document {
    type Target = laminapdf_model::Document;

    fn deref(&self) -> &laminapdf_model::Document {
        &self.model
    }
}

impl Document {
    /// Opens a PDF for editing ([`laminapdf_cos::Document::open`]).
    ///
    /// # Errors
    /// As [`laminapdf_cos::Document::open`].
    ///
    /// Added in 0.2.0.
    pub fn open(source: impl Into<Source>, options: OpenOptions) -> Result<Self> {
        Ok(laminapdf_model::Document::open(source, options)?.into())
    }

    /// Adds another PDF as a source document for the ops that copy pages from one (E2 plan
    /// §2.1), and returns its ID: the [`BlobId`](crate::BlobId) of `bytes`, so adding the same
    /// bytes again, here or on another device, gives the same ID (and opens nothing twice).
    ///
    /// The file opens with this document's [`Limits`](laminapdf_cos::Limits); an encrypted
    /// one with `password` (or the empty user password), and its objects are read decrypted,
    /// so what is copied from it is written as plain objects. The password is used here only:
    /// it never enters an op or the log. The bytes stay in the session's [`BlobStore`] until
    /// the document is dropped, so undo, redo and a replayed log find them.
    ///
    /// ```
    /// use laminapdf_ops::{BlobId, Document};
    /// # use laminapdf_cos::{Dict, Object};
    /// # use laminapdf_save::test_support::Writer;
    /// # let file = || -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    /// #     let mut w = Writer::new(1, 7);
    /// #     let pages = w.add([("Type", Object::name("Pages")), ("Kids", Vec::new().into()), ("Count", 0.into())].into_iter().collect::<Dict>());
    /// #     let root = w.add([("Type", Object::name("Catalog")), ("Pages", pages.into())].into_iter().collect::<Dict>());
    /// #     Ok(w.finish(&[("Root", root)].into_iter().collect())?)
    /// # };
    /// # let (target_bytes, other_bytes) = (file()?, file()?);
    /// let mut doc = Document::open(target_bytes, Default::default())?;
    /// let id = doc.add_source(other_bytes.clone(), None)?;
    /// assert_eq!(id, BlobId::of(&other_bytes));
    /// assert_eq!(doc.add_source(other_bytes, None)?, id);
    /// assert_eq!(doc.source_document(&id)?.page_count()?, 0);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// As [`laminapdf_cos::Document::open`]: `wrong-password` and `password-required` for an
    /// encrypted file, `not-pdf` and the others for one that doesn't open. The session is
    /// unchanged then.
    ///
    /// Added in 0.2.0.
    pub fn add_source(&mut self, bytes: impl Into<Arc<[u8]>>, password: Option<&str>) -> Result<SourceId> {
        let bytes: Arc<[u8]> = bytes.into();
        let id = SourceId::of(&bytes);
        if self.sources.contains_key(&id) {
            return Ok(id);
        }
        let mut options = OpenOptions::default().with_limits(*self.model.limits());
        if let Some(password) = password {
            options = options.with_password(password);
        }
        let source = laminapdf_model::Document::open(Arc::clone(&bytes), options)?;
        self.blobs.insert(bytes);
        self.sources.insert(id, source);
        Ok(id)
    }

    /// The source document added with this ID ([`Document::add_source`]).
    ///
    /// # Errors
    /// [`Error::UnknownSource`] when no source with this ID was added in this session.
    ///
    /// Added in 0.2.0.
    pub fn source_document(&self, id: &SourceId) -> Result<&laminapdf_model::Document> {
        self.sources.get(id).ok_or(Error::UnknownSource(*id))
    }

    /// The bytes ops name by [`BlobId`](crate::BlobId) in this session: every source
    /// document's.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn blobs(&self) -> &BlobStore {
        &self.blobs
    }

    /// The undo and redo stacks.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn history(&self) -> &History {
        &self.history
    }

    /// Keeps at most `depth` undo entries (default [`History::DEFAULT_DEPTH`]).
    ///
    /// Added in 0.2.0.
    pub fn set_undo_depth(&mut self, depth: usize) {
        self.history.set_depth(depth);
    }

    /// Whether local transactions were committed (or undone) since the document was opened
    /// or last [`marked saved`](Document::mark_saved): the UI's "unsaved" dot (ADR 0006 §7).
    /// Remote and system transactions don't count.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn has_unsaved_changes(&self) -> bool {
        self.history.is_modified()
    }

    /// Records that the current state was saved (called by `laminapdf-save`).
    ///
    /// Added in 0.2.0.
    pub fn mark_saved(&mut self) {
        self.history.mark_saved();
    }

    /// The kinds of change not yet saved (ADR 0007 §2): what the save planner checks against
    /// the document's signatures. Empty when the overlay is.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn pending_changes(&self) -> &PendingChanges {
        &self.pending
    }

    /// Where the document stands relative to its last save.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn save_state(&self) -> SaveState {
        if self.unrebased.is_some() {
            SaveState::SavedNotRebased
        } else if self.model.is_dirty() {
            SaveState::Dirty
        } else {
            SaveState::Clean
        }
    }

    /// Records that the current state was written by a save that the document has not been
    /// rebased onto yet (called by `laminapdf-save` once the bytes are written): marks it saved
    /// and enters [`SaveState::SavedNotRebased`]; `pending` says what [`Document::rebase`]
    /// keeps.
    ///
    /// # Errors
    /// [`Error::NestedTransaction`] while a transaction is open (its changes would be lost by
    /// the rebase); [`Error::SavedNotRebased`] when a save is already waiting for its rebase.
    ///
    /// Added in 0.2.0.
    pub fn saved_not_rebased(&mut self, pending: PendingRebase) -> Result<()> {
        if self.open.is_some() {
            return Err(Error::NestedTransaction);
        }
        self.writable()?;
        self.mark_saved();
        self.unrebased = Some(pending);
        Ok(())
    }

    /// Makes `source`, the bytes the pending save wrote, the new base ([ADR 0005] §3): its
    /// newest xref is loaded, the overlay and its journal are dropped, and the state becomes
    /// [`SaveState::Clean`]. Object numbers, and so `LaminaPdfId`s, are the output's. The undo
    /// history and the reserved numbers are kept or dropped as the save said
    /// ([`PendingRebase`]).
    ///
    /// The shell calls it after a full rewrite to a sink the core couldn't read back
    /// ([`SaveState::SavedNotRebased`]); `laminapdf-save` calls it itself otherwise.
    ///
    /// # Errors
    /// [`Error::NestedTransaction`] while a transaction is open; [`Error::NoPendingSave`]
    /// unless a save is waiting for its rebase (any other bytes would not match the history,
    /// and the overlay's changes would be lost); COS errors opening `source` (the document is
    /// then unchanged and still waiting).
    ///
    /// [ADR 0005]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0005-document-model.md
    ///
    /// Added in 0.2.0.
    pub fn rebase(&mut self, source: impl Into<Source>) -> Result<()> {
        if self.open.is_some() {
            return Err(Error::NestedTransaction);
        }
        let pending = self.unrebased.ok_or(Error::NoPendingSave)?;
        let reserved = match pending {
            PendingRebase::KeepHistory => Reserved::Keep,
            PendingRebase::ClearHistory => Reserved::Release,
        };
        self.model.cos_mut().rebase(source, reserved)?;
        self.unrebased = None;
        self.pending.clear();
        if pending == PendingRebase::ClearHistory {
            self.history = History::new(self.history.depth());
            self.history.mark_saved();
        }
        Ok(())
    }

    /// An empty overlay has nothing left to save: no pending class applies any more.
    fn forget_pending_when_clean(&mut self) {
        if !self.model.is_dirty() {
            self.pending.clear();
        }
    }

    /// Refuses ops, undo and redo while a save waits for its rebase.
    const fn writable(&self) -> Result<()> {
        if self.unrebased.is_some() {
            Err(Error::SavedNotRebased)
        } else {
            Ok(())
        }
    }

    /// Whether a transaction is open (handle form).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub const fn in_transaction(&self) -> bool {
        self.open.is_some()
    }

    /// Runs `f` as one transaction from `origin` ([ADR 0006] §2) and commits it when `f`
    /// returns `Ok`. On `Err` every change is unwound in reverse and nothing is recorded.
    /// Local transactions go on the undo stack.
    ///
    /// ```
    /// use laminapdf_ops::{Document, MetadataPatch, Origin, SetMetadata};
    /// # use laminapdf_cos::{Dict, Object};
    /// # use laminapdf_save::test_support::Writer;
    /// # let mut w = Writer::new(1, 7);
    /// # let pages = w.add([("Type", Object::name("Pages")), ("Kids", Vec::new().into()), ("Count", 0.into())].into_iter().collect::<Dict>());
    /// # let root = w.add([("Type", Object::name("Catalog")), ("Pages", pages.into())].into_iter().collect::<Dict>());
    /// # let bytes = w.finish(&[("Root", root)].into_iter().collect())?;
    ///
    /// let mut doc = Document::open(bytes, Default::default())?;
    /// doc.transact(Origin::Local, |tx| {
    ///     tx.apply(SetMetadata::new(MetadataPatch::new().title(Some("Report")), None))?;
    ///     Ok(())
    /// })?;
    /// assert_eq!(doc.info()?.map(|i| i.title()).transpose()?.flatten().as_deref(), Some("Report"));
    ///
    /// doc.undo()?;
    /// assert!(!doc.is_dirty());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Errors
    /// [`Error::NestedTransaction`] when a transaction is already open;
    /// [`Error::SavedNotRebased`]; what `f` returns.
    ///
    /// [ADR 0006]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0006-op-log-and-undo.md
    ///
    /// Added in 0.2.0.
    pub fn transact<T>(&mut self, origin: Origin, f: impl FnOnce(&mut Tx<'_>) -> Result<T>) -> Result<(T, Applied)> {
        self.run(origin, None, f)
    }

    /// A local [`Document::transact`] that merges into the previous undo entry when that one
    /// has the same `key` and nothing was undone since (typing into a field, ADR 0006 §4).
    ///
    /// # Errors
    /// As [`Document::transact`].
    ///
    /// Added in 0.2.0.
    pub fn transact_coalescing<T>(
        &mut self,
        key: &str,
        f: impl FnOnce(&mut Tx<'_>) -> Result<T>,
    ) -> Result<(T, Applied)> {
        self.run(Origin::Local, Some(key.to_owned()), f)
    }

    /// Applies `ops` in order as one transaction from `origin`.
    ///
    /// # Errors
    /// As [`Document::transact`]; the first op that fails rolls back the whole list.
    ///
    /// Added in 0.2.0.
    pub fn apply_all(&mut self, origin: Origin, ops: impl IntoIterator<Item = Op>) -> Result<Applied> {
        self.transact(origin, |tx| ops.into_iter().try_for_each(|op| tx.apply_quietly(&op)))
            .map(|((), applied)| applied)
    }

    fn run<T>(
        &mut self,
        origin: Origin,
        coalesce: Option<String>,
        f: impl FnOnce(&mut Tx<'_>) -> Result<T>,
    ) -> Result<(T, Applied)> {
        self.begin_with(origin, coalesce)?;
        // If `f` panics (and the host catches it), the guard rolls the transaction back, so
        // the document is never left open with half its changes.
        let mut guard = OpenGuard {
            doc: self,
            settled: false,
        };
        let result = f(&mut guard.doc.tx()?);
        guard.settled = true;
        match result {
            Ok(value) => Ok((value, guard.doc.commit()?)),
            Err(e) => {
                guard.doc.rollback()?;
                Err(e)
            }
        }
    }

    /// Opens a transaction from `origin` (handle form, ADR 0006 §2).
    ///
    /// # Errors
    /// [`Error::NestedTransaction`] when one is already open; [`Error::SavedNotRebased`].
    ///
    /// Added in 0.2.0.
    pub fn begin(&mut self, origin: Origin) -> Result<()> {
        self.begin_with(origin, None)
    }

    fn begin_with(&mut self, origin: Origin, coalesce: Option<String>) -> Result<()> {
        if self.open.is_some() {
            return Err(Error::NestedTransaction);
        }
        self.writable()?;
        self.open = Some(TxState {
            origin,
            mark: self.model.journal_mark(),
            touched: BTreeSet::new(),
            pending: PendingChanges::default(),
            coalesce,
            packet_rewritten: false,
            removed: false,
        });
        Ok(())
    }

    fn tx(&mut self) -> Result<Tx<'_>> {
        let state = self.open.as_mut().ok_or(Error::NoTransaction)?;
        Ok(Tx {
            doc: &mut self.model,
            state,
        })
    }

    /// Applies `op` in the open transaction (handle form). On error the op's own changes are
    /// rolled back and the transaction stays open.
    ///
    /// # Errors
    /// [`Error::NoTransaction`]; as [`Tx::apply`].
    ///
    /// Added in 0.2.0.
    pub fn apply(&mut self, op: impl Into<Op>) -> Result<Applied> {
        self.tx()?.apply(op)
    }

    /// Commits the open transaction (handle form); a local one goes on the undo stack.
    ///
    /// # Errors
    /// [`Error::NoTransaction`].
    ///
    /// Added in 0.2.0.
    pub fn commit(&mut self) -> Result<Applied> {
        let mark = self.open.as_ref().ok_or(Error::NoTransaction)?.mark;
        // Close the transaction only once its changes are taken, so a failure leaves it
        // open (and still rollback-able) instead of applied but on no stack.
        let changes = self.model.cos_mut().take_changes(mark)?;
        let state = self.open.take().ok_or(Error::NoTransaction)?;
        let undo = UndoRecord::from_changes(&changes);
        let widgets = state.pending.widgets().iter().cloned().collect();
        if !undo.is_empty() {
            self.pending.merge(&state.pending);
            if state.origin == Origin::Local {
                self.history
                    .push(undo.clone(), state.touched.clone(), state.pending, state.coalesce);
            } else {
                // A remote or system change ends coalescing: merging across it would let
                // undo overwrite it (ADR 0006 §4).
                self.history.break_chain();
            }
        }
        self.forget_pending_when_clean();
        Ok(Applied {
            undo,
            touched_pages: state.touched,
            packet_rewritten: state.packet_rewritten,
            still_in_earlier_revisions: state.removed,
            widgets,
        })
    }

    /// Discards the open transaction (handle form): every change since
    /// [`begin`](Document::begin) is unwound.
    ///
    /// # Errors
    /// [`Error::NoTransaction`].
    ///
    /// Added in 0.2.0.
    pub fn rollback(&mut self) -> Result<()> {
        let state = self.open.take().ok_or(Error::NoTransaction)?;
        self.model.cos_mut().rollback_to(state.mark)?;
        Ok(())
    }

    /// Undoes the newest local transaction ([ADR 0006] §4). Each property it changed is
    /// read as it is now: one that still holds the value the transaction set is restored,
    /// one that another change (a remote one) replaced since is left alone, as is one whose
    /// target is gone. The restoring changes become the redo entry. `Ok(None)` when there is
    /// nothing to undo.
    ///
    /// # Errors
    /// [`Error::NestedTransaction`] while a transaction is open; [`Error::SavedNotRebased`];
    /// COS errors reading the document (the entry stays on the stack).
    ///
    /// [ADR 0006]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0006-op-log-and-undo.md
    ///
    /// Added in 0.2.0.
    pub fn undo(&mut self) -> Result<Option<Reverted>> {
        if self.open.is_some() {
            return Err(Error::NestedTransaction);
        }
        self.writable()?;
        let Some(entry) = self.history.undo.pop_back() else {
            return Ok(None);
        };
        match self.revert(&entry) {
            Ok((inverse, reverted)) => {
                if let Some(inverse) = inverse {
                    self.history.redo.push(inverse);
                }
                Ok(Some(reverted))
            }
            Err(e) => {
                self.history.undo.push_back(entry);
                Err(e)
            }
        }
    }

    /// Redoes the newest undone transaction, the mirror of [`Document::undo`]. `Ok(None)`
    /// when there is nothing to redo.
    ///
    /// # Errors
    /// As [`Document::undo`].
    ///
    /// Added in 0.2.0.
    pub fn redo(&mut self) -> Result<Option<Reverted>> {
        if self.open.is_some() {
            return Err(Error::NestedTransaction);
        }
        self.writable()?;
        let Some(entry) = self.history.redo.pop() else {
            return Ok(None);
        };
        match self.revert(&entry) {
            Ok((inverse, reverted)) => {
                if let Some(inverse) = inverse {
                    self.history.undo.push_back(inverse);
                }
                Ok(Some(reverted))
            }
            Err(e) => {
                self.history.redo.push(entry);
                Err(e)
            }
        }
    }

    /// Reverts `entry` as one transaction; returns the entry that reverts the reversion
    /// (`None` when nothing changed) and what happened.
    fn revert(&mut self, entry: &Entry) -> Result<(Option<Entry>, Reverted)> {
        let cos = self.model.cos_mut();
        let mark = cos.journal_mark();
        let result = revert(&mut cos.edit(), &entry.record);
        let (restored, skipped) = match result {
            Ok(r) => r,
            Err(e) => {
                cos.rollback_to(mark)?;
                return Err(e);
            }
        };
        // `mark` was taken above under this `&mut self` with nothing in between that drains
        // the journal, so it cannot be stale and this cannot fail after `revert` applied.
        let record = UndoRecord::from_changes(&cos.take_changes(mark)?);
        // Even an undo or redo that restored nothing ends coalescing.
        self.history.break_chain();
        if !record.is_empty() {
            self.pending.merge(&entry.pending);
        }
        self.forget_pending_when_clean();
        let outcome = match (restored, skipped.is_empty()) {
            (_, true) => UndoOutcome::Applied,
            (0, false) => UndoOutcome::Skipped { skipped },
            (_, false) => UndoOutcome::Partial { skipped },
        };
        let inverse = (!record.is_empty()).then(|| Entry {
            record,
            touched: entry.touched.clone(),
            key: None,
            serial: entry.serial,
            pending: entry.pending.clone(),
        });
        Ok((
            inverse,
            Reverted {
                outcome,
                touched_pages: entry.touched.clone(),
            },
        ))
    }
}

/// Rolls back the open transaction when dropped before it was committed or rolled back
/// (a panic in the closure of [`Document::transact`]).
struct OpenGuard<'d> {
    doc: &'d mut Document,
    settled: bool,
}

impl Drop for OpenGuard<'_> {
    fn drop(&mut self) {
        if !self.settled {
            // Nothing to report from a drop; a failed rollback leaves the transaction
            // closed and the journal as it was.
            let _ = self.doc.rollback();
        }
    }
}
