//! [`Document`]: a model document that can only be changed through transactions.

use std::collections::BTreeSet;
use std::fmt;
use std::ops;

use lamina_cos::{OpenOptions, Source};

use crate::error::{Error, Result};
use crate::history::{Entry, History, Reverted, UndoOutcome};
use crate::op::Op;
use crate::record::{UndoRecord, revert};
use crate::tx::{Applied, Origin, Tx, TxState};

/// An editable PDF ([ADR 0006]): a [`lamina_model::Document`] (it dereferences to one, for
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
///   [`lamina_cos::Document::is_dirty`] (the overlay is non-empty).
///
/// There is no `&mut` access to the model or COS document, so nothing changes it outside a
/// transaction:
///
/// ```compile_fail
/// fn sneak(doc: &mut lamina_ops::Document) {
///     doc.cos_mut().edit(); // no DerefMut: the model document is read-only from here
/// }
/// ```
///
/// Like the documents below it, it is `Send` but `!Sync`.
///
/// [ADR 0006]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0006-op-log-and-undo.md
pub struct Document {
    model: lamina_model::Document,
    history: History,
    open: Option<TxState>,
}

// ADR 0005 §7: a document can move between threads.
const _: () = {
    const fn assert_send<T: Send>() {}
    assert_send::<Document>();
};

impl fmt::Debug for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Document")
            .field("model", &self.model)
            .field("history", &self.history)
            .field("open", &self.open)
            .finish()
    }
}

impl From<lamina_model::Document> for Document {
    fn from(model: lamina_model::Document) -> Self {
        Self {
            model,
            history: History::default(),
            open: None,
        }
    }
}

impl ops::Deref for Document {
    type Target = lamina_model::Document;

    fn deref(&self) -> &lamina_model::Document {
        &self.model
    }
}

impl Document {
    /// Opens a PDF for editing ([`lamina_cos::Document::open`]).
    ///
    /// # Errors
    /// As [`lamina_cos::Document::open`].
    pub fn open(source: impl Into<Source>, options: OpenOptions) -> Result<Self> {
        Ok(lamina_model::Document::open(source, options)?.into())
    }

    /// The undo and redo stacks.
    #[must_use]
    pub const fn history(&self) -> &History {
        &self.history
    }

    /// Keeps at most `depth` undo entries (default [`History::DEFAULT_DEPTH`]).
    pub fn set_undo_depth(&mut self, depth: usize) {
        self.history.set_depth(depth);
    }

    /// Whether local transactions were committed (or undone) since the document was opened
    /// or last [`marked saved`](Document::mark_saved): the UI's "unsaved" dot (ADR 0006 §7).
    /// Remote and system transactions don't count.
    #[must_use]
    pub fn has_unsaved_changes(&self) -> bool {
        self.history.is_modified()
    }

    /// Records that the current state was saved (called by the save planner, B3b).
    pub fn mark_saved(&mut self) {
        self.history.mark_saved();
    }

    /// Whether a transaction is open (handle form).
    #[must_use]
    pub const fn in_transaction(&self) -> bool {
        self.open.is_some()
    }

    /// Runs `f` as one transaction from `origin` ([ADR 0006] §2) and commits it when `f`
    /// returns `Ok`. On `Err` every change is unwound in reverse and nothing is recorded.
    /// Local transactions go on the undo stack.
    ///
    /// ```
    /// use lamina_ops::{Document, InfoPatch, Origin, SetInfo};
    /// # use lamina_cos::{Dict, Object, Writer};
    /// # let mut w = Writer::new(1, 7);
    /// # let pages = w.add([("Type", Object::name("Pages")), ("Kids", Vec::new().into()), ("Count", 0.into())].into_iter().collect::<Dict>());
    /// # let root = w.add([("Type", Object::name("Catalog")), ("Pages", pages.into())].into_iter().collect::<Dict>());
    /// # let bytes = w.finish(&[("Root", root)].into_iter().collect())?;
    ///
    /// let mut doc = Document::open(bytes, Default::default())?;
    /// doc.transact(Origin::Local, |tx| {
    ///     tx.apply(SetInfo::new(InfoPatch::new().title(Some("Report")), None))?;
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
    /// [`Error::NestedTransaction`] when a transaction is already open; what `f` returns.
    ///
    /// [ADR 0006]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0006-op-log-and-undo.md
    pub fn transact<T>(&mut self, origin: Origin, f: impl FnOnce(&mut Tx<'_>) -> Result<T>) -> Result<(T, Applied)> {
        self.run(origin, None, f)
    }

    /// A local [`Document::transact`] that merges into the previous undo entry when that one
    /// has the same `key` and nothing was undone since (typing into a field, ADR 0006 §4).
    ///
    /// # Errors
    /// As [`Document::transact`].
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
    pub fn apply_all(&mut self, origin: Origin, ops: impl IntoIterator<Item = Op>) -> Result<Applied> {
        self.transact(origin, |tx| ops.into_iter().try_for_each(|op| tx.apply(op).map(drop)))
            .map(|((), applied)| applied)
    }

    fn run<T>(
        &mut self,
        origin: Origin,
        coalesce: Option<String>,
        f: impl FnOnce(&mut Tx<'_>) -> Result<T>,
    ) -> Result<(T, Applied)> {
        self.begin_with(origin, coalesce)?;
        let result = f(&mut self.tx()?);
        match result {
            Ok(value) => Ok((value, self.commit()?)),
            Err(e) => {
                self.rollback()?;
                Err(e)
            }
        }
    }

    /// Opens a transaction from `origin` (handle form, ADR 0006 §2).
    ///
    /// # Errors
    /// [`Error::NestedTransaction`] when one is already open.
    pub fn begin(&mut self, origin: Origin) -> Result<()> {
        self.begin_with(origin, None)
    }

    fn begin_with(&mut self, origin: Origin, coalesce: Option<String>) -> Result<()> {
        if self.open.is_some() {
            return Err(Error::NestedTransaction);
        }
        self.open = Some(TxState {
            origin,
            mark: self.model.journal_mark(),
            touched: BTreeSet::new(),
            coalesce,
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
    pub fn apply(&mut self, op: impl Into<Op>) -> Result<Applied> {
        self.tx()?.apply(op)
    }

    /// Commits the open transaction (handle form); a local one goes on the undo stack.
    ///
    /// # Errors
    /// [`Error::NoTransaction`].
    pub fn commit(&mut self) -> Result<Applied> {
        let state = self.open.take().ok_or(Error::NoTransaction)?;
        let changes = self.model.cos_mut().take_changes(state.mark);
        let undo = UndoRecord::from_changes(&changes);
        if state.origin == Origin::Local && !undo.is_empty() {
            self.history.push(undo.clone(), state.touched.clone(), state.coalesce);
        }
        Ok(Applied {
            undo,
            touched_pages: state.touched,
        })
    }

    /// Discards the open transaction (handle form): every change since
    /// [`begin`](Document::begin) is unwound.
    ///
    /// # Errors
    /// [`Error::NoTransaction`].
    pub fn rollback(&mut self) -> Result<()> {
        let state = self.open.take().ok_or(Error::NoTransaction)?;
        self.model.cos_mut().rollback_to(state.mark);
        Ok(())
    }

    /// Undoes the newest local transaction ([ADR 0006] §4). Each property it changed is
    /// read as it is now: one that still holds the value the transaction set is restored,
    /// one that another change (a remote one) replaced since is left alone, as is one whose
    /// target is gone. The restoring changes become the redo entry. `Ok(None)` when there is
    /// nothing to undo.
    ///
    /// # Errors
    /// [`Error::NestedTransaction`] while a transaction is open; COS errors reading the
    /// document (the entry stays on the stack).
    ///
    /// [ADR 0006]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0006-op-log-and-undo.md
    pub fn undo(&mut self) -> Result<Option<Reverted>> {
        if self.open.is_some() {
            return Err(Error::NestedTransaction);
        }
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
    pub fn redo(&mut self) -> Result<Option<Reverted>> {
        if self.open.is_some() {
            return Err(Error::NestedTransaction);
        }
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
                cos.rollback_to(mark);
                return Err(e);
            }
        };
        let record = UndoRecord::from_changes(&cos.take_changes(mark));
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
