//! Undo and redo ([ADR 0006] §4): per document, local transactions only, inverses computed
//! at undo time from the current state.
//!
//! [ADR 0006]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0006-op-log-and-undo.md

use std::collections::{BTreeSet, VecDeque};

use laminapdf_model::PageId;

use crate::pending::PendingChanges;
use crate::record::{Skipped, UndoRecord};

/// The result of an undo or redo ([ADR 0006] §4).
///
/// [ADR 0006]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0006-op-log-and-undo.md
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum UndoOutcome {
    /// Every property was restored.
    Applied,
    /// Some properties were restored; these were left alone because another change replaced
    /// them since (it wins) or their target is gone.
    Partial {
        /// What was left alone.
        skipped: Vec<Skipped>,
    },
    /// Nothing could be restored.
    Skipped {
        /// What was left alone.
        skipped: Vec<Skipped>,
    },
}

/// What [`Document::undo`](crate::Document::undo) or
/// [`redo`](crate::Document::redo) did.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Reverted {
    /// Whether everything was restored.
    pub outcome: UndoOutcome,
    /// The pages the reverted transaction touched (re-render these).
    pub touched_pages: BTreeSet<PageId>,
}

/// One undoable (or redoable) transaction.
#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub(crate) record: UndoRecord,
    pub(crate) touched: BTreeSet<PageId>,
    /// The coalescing key, for [`Document::transact_coalescing`](crate::Document::transact_coalescing).
    pub(crate) key: Option<String>,
    /// Identifies the document state this entry leads to, for the semantic dirty state.
    pub(crate) serial: u64,
    /// The op classes of the transaction, added back to the document's pending changes when
    /// the entry is undone or redone.
    pub(crate) pending: PendingChanges,
}

/// A document's undo and redo stacks ([ADR 0006] §4). Only local transactions are pushed;
/// the oldest entries are dropped beyond [`History::depth`] (500 by default).
///
/// [ADR 0006]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0006-op-log-and-undo.md
#[derive(Debug, Clone)]
pub struct History {
    pub(crate) undo: VecDeque<Entry>,
    pub(crate) redo: Vec<Entry>,
    depth: usize,
    next_serial: u64,
    /// Where the last save (or the open) left the undo stack.
    saved: Saved,
    /// Whether the next coalescing transaction may merge into the top entry: true right
    /// after a local commit; false after an undo or redo (even one that restored nothing)
    /// and after a remote or system transaction changed something, so a merged entry never
    /// spans a change that is not its own (ADR 0006 §4).
    chain_open: bool,
}

/// The undo position of the last save.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Saved {
    /// The empty stack: the document as opened (or saved with nothing to undo).
    Empty,
    /// The top entry had this serial.
    At(u64),
    /// The entries leading back to it were dropped (depth), so no position matches it.
    Lost,
}

impl Default for History {
    fn default() -> Self {
        Self::new(Self::DEFAULT_DEPTH)
    }
}

impl History {
    /// The default undo depth: 500 transactions.
    pub const DEFAULT_DEPTH: usize = 500;

    /// Empty stacks keeping at most `depth` undo entries.
    #[must_use]
    pub fn new(depth: usize) -> Self {
        Self {
            undo: VecDeque::new(),
            redo: Vec::new(),
            depth,
            next_serial: 1,
            saved: Saved::Empty,
            chain_open: false,
        }
    }

    /// The most undo entries kept.
    #[must_use]
    pub const fn depth(&self) -> usize {
        self.depth
    }

    /// Keeps at most `depth` undo entries, dropping the oldest now if there are more.
    pub fn set_depth(&mut self, depth: usize) {
        self.depth = depth;
        self.trim();
    }

    /// The number of transactions that can be undone.
    #[must_use]
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    /// The number of transactions that can be redone.
    #[must_use]
    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    /// Whether there is something to undo.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Whether there is something to redo.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Records a committed local transaction: merged into the top entry when it has the
    /// same coalescing `key` and the chain is still open (nothing was undone or redone and
    /// no other origin changed the document since), else pushed. Clears redo.
    pub(crate) fn push(
        &mut self,
        record: UndoRecord,
        touched: BTreeSet<PageId>,
        pending: PendingChanges,
        key: Option<String>,
    ) {
        let serial = self.serial();
        let coalesce = self.chain_open && key.is_some() && self.undo.back().is_some_and(|top| top.key == key);
        self.redo.clear();
        self.chain_open = true;
        if coalesce && let Some(top) = self.undo.back_mut() {
            top.record.merge(record);
            top.touched.extend(touched);
            top.pending.merge(&pending);
            top.serial = serial;
            if top.record.is_empty() {
                self.undo.pop_back();
            }
        } else {
            self.undo.push_back(Entry {
                record,
                touched,
                key,
                serial,
                pending,
            });
            self.trim();
        }
    }

    /// A fresh serial.
    fn serial(&mut self) -> u64 {
        let serial = self.next_serial;
        self.next_serial += 1;
        serial
    }

    /// Ends the coalescing chain: the next transaction starts a new entry.
    pub(crate) fn break_chain(&mut self) {
        self.chain_open = false;
    }

    fn trim(&mut self) {
        while self.undo.len() > self.depth {
            let Some(dropped) = self.undo.pop_front() else { break };
            // Undo can no longer reach the state before `dropped`: if that was the saved
            // one, no position is saved any more.
            if self.saved == Saved::Empty || self.saved == Saved::At(dropped.serial) {
                self.saved = Saved::Lost;
            }
        }
    }

    /// Whether the undo position differs from the last save's (ADR 0006 §7, semantic level).
    pub(crate) fn is_modified(&self) -> bool {
        let top = self.undo.back().map(|e| e.serial);
        match self.saved {
            Saved::Empty => top.is_some(),
            Saved::At(serial) => top != Some(serial),
            Saved::Lost => true,
        }
    }

    /// Records the current position as saved.
    pub(crate) fn mark_saved(&mut self) {
        self.saved = self.undo.back().map_or(Saved::Empty, |e| Saved::At(e.serial));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use laminapdf_cos::{CosChange, ObjRef, Object};

    use super::*;

    fn record(n: i64) -> UndoRecord {
        let change = CosChange::Create {
            r: ObjRef::new(10, 0),
            object: Arc::new(Object::Integer(n)),
        };
        UndoRecord::from_changes([&change])
    }

    #[test]
    fn push_trims_the_oldest_entries_beyond_the_depth() {
        let mut h = History::new(2);
        for n in 0..3 {
            h.push(record(n), BTreeSet::new(), PendingChanges::default(), None);
        }
        assert_eq!(h.undo.front().map(|e| e.serial), Some(2));
    }

    #[test]
    fn push_clears_redo() {
        let mut h = History::default();
        h.redo.push(Entry {
            record: record(1),
            touched: BTreeSet::new(),
            key: None,
            serial: 1,
            pending: PendingChanges::default(),
        });
        h.push(record(2), BTreeSet::new(), PendingChanges::default(), None);
        assert!(!h.can_redo());
    }

    #[test]
    fn push_coalesces_entries_with_the_same_key() {
        let mut h = History::default();
        h.push(
            record(1),
            BTreeSet::new(),
            PendingChanges::default(),
            Some("k".to_owned()),
        );
        h.push(
            record(2),
            BTreeSet::new(),
            PendingChanges::default(),
            Some("k".to_owned()),
        );
        assert_eq!(h.undo_len(), 1);
    }

    #[test]
    fn entries_without_a_key_never_coalesce() {
        let mut h = History::default();
        h.push(record(1), BTreeSet::new(), PendingChanges::default(), None);
        h.push(record(2), BTreeSet::new(), PendingChanges::default(), None);
        assert_eq!(h.undo_len(), 2);
    }

    #[test]
    fn a_fresh_history_is_unmodified_and_a_push_modifies_it() {
        let mut h = History::default();
        let fresh = h.is_modified();
        h.push(record(1), BTreeSet::new(), PendingChanges::default(), None);
        assert_eq!((fresh, h.is_modified()), (false, true));
    }

    #[test]
    fn coalescing_after_a_save_counts_as_a_modification() {
        let mut h = History::default();
        h.push(
            record(1),
            BTreeSet::new(),
            PendingChanges::default(),
            Some("k".to_owned()),
        );
        h.mark_saved();
        h.push(
            record(2),
            BTreeSet::new(),
            PendingChanges::default(),
            Some("k".to_owned()),
        );
        assert!(h.is_modified());
    }

    #[test]
    fn set_depth_drops_entries_now() {
        let mut h = History::default();
        for n in 0..5 {
            h.push(record(n), BTreeSet::new(), PendingChanges::default(), None);
        }
        h.set_depth(1);
        assert_eq!(h.undo_len(), 1);
    }
}
