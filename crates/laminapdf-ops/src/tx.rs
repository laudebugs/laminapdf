//! Transactions ([ADR 0006] §2): atomic groups of ops, with savepoints for trying something
//! and recovering.
//!
//! [ADR 0006]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0006-op-log-and-undo.md

use std::collections::BTreeSet;
use std::fmt;

use laminapdf_cos::JournalMark;
use laminapdf_model::{OutlineItemId, PageId};

use crate::error::Result;
use crate::op::{Effect, Op};
use crate::outline::DestinationUse;
use crate::pages::{FieldChange, ReferenceChange, RenamedDestination};
use crate::pending::PendingChanges;
use crate::record::UndoRecord;
use crate::widgets::TouchedWidget;

/// The identity of a collaborator (ADR 0012), as sync assigns it.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PeerId(String);

impl PeerId {
    /// The peer called `id`.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The ID as a string.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Where a transaction comes from ([ADR 0006] §2). Only [`Origin::Local`] ones are undoable
/// (§4): remote and system changes never rewind.
///
/// [ADR 0006]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0006-op-log-and-undo.md
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Origin {
    /// This user.
    Local,
    /// A collaborator, through sync.
    Remote(PeerId),
    /// Lamina PDF itself (for example a migration on open).
    System,
}

/// What a committed transaction, or one op inside it, did ([ADR 0006] §3).
///
/// [ADR 0006]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0006-op-log-and-undo.md
///
/// Added in 0.2.0.
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct Applied {
    /// Every property changed, with its value before and after.
    pub undo: UndoRecord,
    /// The pages whose rendering may have changed (viewers re-render only these).
    pub touched_pages: BTreeSet<PageId>,
    /// A metadata op rewrote the XMP packet whole (it was UTF-16, or read-only by its
    /// `end="r"` trailer): its properties are kept, its layout and comments are not.
    pub packet_rewritten: bool,
    /// A metadata op removed values that the file's earlier revisions still hold: an
    /// incremental save keeps them in the file, a full rewrite drops them.
    pub still_in_earlier_revisions: bool,
    /// The properties an XMP op declared in a PDF/A packet's extension schemas, so it stays
    /// valid PDF/A ([`SetXmpProperty`](crate::SetXmpProperty),
    /// [`ReplaceXmpPacket`](crate::ReplaceXmpPacket)).
    ///
    /// Added in 0.3.0.
    pub declared: Vec<laminapdf_model::xmp::ExtensionProperty>,
    /// Annotations an annotation op deleted besides the one it names: Popups, replies and the
    /// rest of a group.
    pub also_deleted: Vec<laminapdf_model::AnnotId>,
    /// The form fields whose widgets it deleted, reshowed or moved, which the save planner
    /// checks against signed signature fields and field locks (E2 plan §6).
    ///
    /// Added in 0.2.0.
    pub widgets: Vec<TouchedWidget>,
    /// The bookmarks [`AddOutlineItem`](crate::AddOutlineItem) created, in order.
    ///
    /// Added in 0.3.0.
    pub created_outline_items: Vec<OutlineItemId>,
    /// The links [`AddLink`](crate::AddLink) created, in order.
    ///
    /// Added in 0.3.0.
    pub created_annotations: Vec<laminapdf_model::AnnotId>,
    /// An outline op found the outline's links broken (a cycle, a wrong `/Last`, an item
    /// stored as a direct dictionary) and rewrote them whole; items that were direct have new
    /// IDs.
    ///
    /// Added in 0.3.0.
    pub outline_repaired: bool,
    /// What still names a destination that [`DeleteNamedDestination`](crate::DeleteNamedDestination)
    /// deleted: those bookmarks and links no longer go anywhere.
    ///
    /// Added in 0.3.0.
    pub destination_uses: Vec<DestinationUse>,
    /// The page IDs in order after the edit, when it changed the page list (inserted,
    /// deleted, moved or reversed pages); `None` otherwise.
    ///
    /// Added in 0.3.0.
    pub page_order: Option<Vec<PageId>>,
    /// The references to deleted pages that the edit retargeted or removed (E2 plan §11):
    /// outline items, links, named destinations, `/OpenAction` and structure elements.
    ///
    /// Added in 0.3.0.
    pub references: Vec<ReferenceChange>,
    /// The form fields whose widgets a duplicated page showed: each copy's new field, or a
    /// signature field whose widget was not copied (E2 plan §2.2).
    ///
    /// Added in 0.3.0.
    pub fields: Vec<FieldChange>,
    /// The copies a page duplication made whose tags couldn't all be cloned: content of
    /// elements that also hold content on other pages is left untagged on them
    /// (`DuplicateTagsIncomplete`).
    ///
    /// Added in 0.3.0.
    pub tags_incomplete: Vec<PageId>,
    /// The page holding each link a page import removed because the page its destination
    /// named didn't come along (E2 plan §2.2): one entry per link.
    ///
    /// Added in 0.3.0.
    pub links_removed: Vec<PageId>,
    /// The pages brought in from another document whose source pages were tagged: they come
    /// without their tags (`TagsDropped`) until structure trees can be merged.
    ///
    /// Added in 0.3.0.
    pub tags_dropped: Vec<PageId>,
    /// The named destinations a merge or a page import brought in under another name, since
    /// the target already used theirs (E2 plan §2.2).
    ///
    /// Added in 0.3.0.
    pub renamed_destinations: Vec<RenamedDestination>,
    /// The bookmarks a merge left out because the pages they went to didn't come along (and
    /// none of their children did either).
    ///
    /// Added in 0.3.0.
    pub bookmarks_dropped: usize,
}

/// The state of the open transaction, kept on the [`Document`](crate::Document) so the
/// handle API (`begin` / `apply` / `commit`) works across calls.
#[derive(Debug)]
pub(crate) struct TxState {
    pub(crate) origin: Origin,
    pub(crate) mark: JournalMark,
    pub(crate) touched: BTreeSet<PageId>,
    /// The classes of the ops applied so far.
    pub(crate) pending: PendingChanges,
    pub(crate) coalesce: Option<String>,
    /// What the ops applied so far did besides their changes (`Applied`'s flags).
    pub(crate) packet_rewritten: bool,
    pub(crate) removed: bool,
    pub(crate) outline_repaired: bool,
    pub(crate) created_outline_items: Vec<OutlineItemId>,
    pub(crate) created_annotations: Vec<laminapdf_model::AnnotId>,
    pub(crate) destination_uses: Vec<DestinationUse>,
    pub(crate) declared: Vec<laminapdf_model::xmp::ExtensionProperty>,
    /// The page order after the last op that changed it.
    pub(crate) page_order: Option<Vec<PageId>>,
    /// The references the ops so far retargeted or removed.
    pub(crate) references: Vec<ReferenceChange>,
    pub(crate) also_deleted: Vec<laminapdf_model::AnnotId>,
    /// The fields the ops so far copied or left out.
    pub(crate) fields: Vec<FieldChange>,
    /// The copies the ops so far couldn't tag whole.
    pub(crate) tags_incomplete: Vec<PageId>,
    /// The pages holding links the ops so far removed, one per link.
    pub(crate) links_removed: Vec<PageId>,
    /// The imported pages the ops so far brought in without their tags.
    pub(crate) tags_dropped: Vec<PageId>,
    /// The named destinations the ops so far brought in under another name.
    pub(crate) renamed_destinations: Vec<RenamedDestination>,
    /// The bookmarks the ops so far left out.
    pub(crate) bookmarks_dropped: usize,
}

/// An open transaction ([ADR 0006] §2), handed to the closure of
/// [`Document::transact`](crate::Document::transact). Ops applied through it are atomic: if
/// the transaction fails, every change is unwound. The document can be read through
/// [`Tx::document`]; the only way to change it is [`Tx::apply`].
///
/// There is no way to start a transaction from inside one: code that wants to try something
/// and recover uses [`Tx::savepoint`].
///
/// [ADR 0006]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0006-op-log-and-undo.md
///
/// Added in 0.2.0.
pub struct Tx<'d> {
    pub(crate) doc: &'d mut laminapdf_model::Document,
    /// The session's source documents, which ops that copy pages read.
    pub(crate) sources: &'d crate::document::Sources,
    pub(crate) state: &'d mut TxState,
}

impl fmt::Debug for Tx<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tx").field("state", &self.state).finish_non_exhaustive()
    }
}

impl Tx<'_> {
    /// The document as this transaction has changed it so far.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn document(&self) -> &laminapdf_model::Document {
        self.doc
    }

    /// Where this transaction comes from.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn origin(&self) -> &Origin {
        &self.state.origin
    }

    /// Applies `op`. On error its own changes are rolled back before the error returns, so a
    /// caught error never leaves half an op behind; the transaction stays open.
    ///
    /// # Errors
    /// The op's errors ([`Error::UnknownPage`](crate::Error::UnknownPage), …) and COS errors
    /// reading the document.
    ///
    /// Added in 0.2.0.
    pub fn apply(&mut self, op: impl Into<Op>) -> Result<Applied> {
        let mark = self.doc.journal_mark();
        let effect = self.run_op(&op.into())?;
        let undo = UndoRecord::from_changes(self.doc.changes_since(mark)?);
        Ok(Applied {
            undo,
            touched_pages: effect.pages.into_iter().collect(),
            packet_rewritten: effect.packet_rewritten,
            still_in_earlier_revisions: effect.removed,
            declared: effect.declared,
            also_deleted: effect.also_deleted,
            widgets: effect.widgets,
            created_outline_items: effect.created_outline_items,
            created_annotations: effect.created_annotations,
            outline_repaired: effect.outline_repaired,
            destination_uses: effect.destination_uses,
            page_order: effect.page_order,
            references: effect.references,
            fields: effect.fields,
            tags_incomplete: effect.tags_incomplete,
            links_removed: effect.links_removed,
            tags_dropped: effect.tags_dropped,
            renamed_destinations: effect.renamed_destinations,
            bookmarks_dropped: effect.bookmarks_dropped,
        })
    }

    /// [`Tx::apply`] without the per-op undo record (the transaction's record is built
    /// once, at commit).
    pub(crate) fn apply_quietly(&mut self, op: &Op) -> Result<()> {
        self.run_op(op).map(drop)
    }

    /// Runs `op`, rolling back its own changes on error; returns what it did.
    fn run_op(&mut self, op: &Op) -> Result<Effect> {
        let mark = self.doc.journal_mark();
        match op.apply_to(self.doc, self.sources) {
            Ok(effect) => {
                self.state.touched.extend(effect.pages.iter().cloned());
                self.state.pending.record(op, &effect);
                self.state.packet_rewritten |= effect.packet_rewritten;
                self.state.removed |= effect.removed;
                self.state.outline_repaired |= effect.outline_repaired;
                self.state
                    .created_outline_items
                    .extend(effect.created_outline_items.iter().cloned());
                self.state
                    .created_annotations
                    .extend(effect.created_annotations.iter().cloned());
                self.state
                    .destination_uses
                    .extend(effect.destination_uses.iter().cloned());
                self.state.declared.extend(effect.declared.iter().cloned());
                if effect.page_order.is_some() {
                    self.state.page_order.clone_from(&effect.page_order);
                }
                self.state.references.extend(effect.references.iter().cloned());
                self.state.also_deleted.extend(effect.also_deleted.iter().cloned());
                self.state.fields.extend(effect.fields.iter().cloned());
                self.state
                    .tags_incomplete
                    .extend(effect.tags_incomplete.iter().cloned());
                self.state.links_removed.extend(effect.links_removed.iter().cloned());
                self.state.tags_dropped.extend(effect.tags_dropped.iter().cloned());
                self.state
                    .renamed_destinations
                    .extend(effect.renamed_destinations.iter().cloned());
                self.state.bookmarks_dropped += effect.bookmarks_dropped;
                Ok(effect)
            }
            Err(e) => {
                self.doc.cos_mut().rollback_to(mark)?;
                Err(e)
            }
        }
    }

    /// Runs `f` and, when it returns `Err`, rolls back only what `f` changed; the
    /// transaction carries on either way ([ADR 0006] §2).
    ///
    /// # Errors
    /// What `f` returns.
    ///
    /// [ADR 0006]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0006-op-log-and-undo.md
    ///
    /// Added in 0.2.0.
    pub fn savepoint<T>(&mut self, f: impl FnOnce(&mut Tx<'_>) -> Result<T>) -> Result<T> {
        let mark = self.doc.journal_mark();
        let touched = self.state.touched.clone();
        let pending = self.state.pending.clone();
        let reports = (
            self.state.outline_repaired,
            self.state.created_outline_items.len(),
            self.state.destination_uses.len(),
            self.state.created_annotations.len(),
        );
        let page_order = self.state.page_order.clone();
        let references = self.state.references.len();
        let fields = self.state.fields.len();
        let tags_incomplete = self.state.tags_incomplete.len();
        let links_removed = self.state.links_removed.len();
        let tags_dropped = self.state.tags_dropped.len();
        let merged = (self.state.renamed_destinations.len(), self.state.bookmarks_dropped);
        let result = f(self);
        if result.is_err() {
            self.doc.cos_mut().rollback_to(mark)?;
            self.state.touched = touched;
            self.state.pending = pending;
            // What the rolled-back ops reported goes with them.
            self.state.outline_repaired = reports.0;
            self.state.created_outline_items.truncate(reports.1);
            self.state.destination_uses.truncate(reports.2);
            self.state.created_annotations.truncate(reports.3);
            self.state.page_order = page_order;
            self.state.references.truncate(references);
            self.state.fields.truncate(fields);
            self.state.tags_incomplete.truncate(tags_incomplete);
            self.state.links_removed.truncate(links_removed);
            self.state.tags_dropped.truncate(tags_dropped);
            self.state.renamed_destinations.truncate(merged.0);
            self.state.bookmarks_dropped = merged.1;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use laminapdf_cos::{Dict, Object, OpenOptions};
    use laminapdf_model::Rotation;
    use laminapdf_save::test_support::Writer;

    use super::*;
    use crate::{Error, RotatePage};

    fn doc() -> laminapdf_model::Document {
        let mut w = Writer::new(1, 7);
        let pages = w.reserve();
        let page = w.add(
            [("Type", Object::name("Page")), ("Parent", pages.into())]
                .into_iter()
                .collect::<Dict>(),
        );
        w.set(
            pages,
            [
                ("Type", Object::name("Pages")),
                ("Kids", vec![page.into()].into()),
                ("Count", 1.into()),
            ]
            .into_iter()
            .collect::<Dict>(),
        );
        let root = w.add(
            [("Type", Object::name("Catalog")), ("Pages", pages.into())]
                .into_iter()
                .collect::<Dict>(),
        );
        let bytes = w.finish(&[("Root", root)].into_iter().collect()).unwrap();
        laminapdf_model::Document::open(bytes, OpenOptions::default()).unwrap()
    }

    fn state(doc: &laminapdf_model::Document) -> TxState {
        TxState {
            origin: Origin::Local,
            mark: doc.journal_mark(),
            touched: BTreeSet::new(),
            pending: PendingChanges::default(),
            coalesce: None,
            packet_rewritten: false,
            removed: false,
            outline_repaired: false,
            created_outline_items: Vec::new(),
            created_annotations: Vec::new(),
            destination_uses: Vec::new(),
            declared: Vec::new(),
            page_order: None,
            references: Vec::new(),
            also_deleted: Vec::new(),
            fields: Vec::new(),
            tags_incomplete: Vec::new(),
            links_removed: Vec::new(),
            tags_dropped: Vec::new(),
            renamed_destinations: Vec::new(),
            bookmarks_dropped: 0,
        }
    }

    fn rotate(page: &str) -> RotatePage {
        RotatePage::new(page.parse().unwrap(), Rotation::Half)
    }

    #[test]
    fn apply_records_the_touched_page_on_the_transaction() {
        let mut doc = doc();
        let mut state = state(&doc);
        let sources = crate::document::Sources::new();
        Tx {
            doc: &mut doc,
            sources: &sources,
            state: &mut state,
        }
        .apply(rotate("p2.0"))
        .unwrap();
        assert_eq!(state.touched.len(), 1);
    }

    #[test]
    fn a_failing_savepoint_forgets_the_pages_it_touched() {
        let mut doc = doc();
        let mut state = state(&doc);
        let sources = crate::document::Sources::new();
        let mut tx = Tx {
            doc: &mut doc,
            sources: &sources,
            state: &mut state,
        };
        let result = tx.savepoint(|tx| {
            tx.apply(rotate("p2.0"))?;
            Err::<(), _>(Error::NoTransaction)
        });
        assert_eq!(
            (result, state.touched.len(), doc.is_dirty()),
            (Err(Error::NoTransaction), 0, false)
        );
    }

    #[test]
    fn a_failing_op_rolls_back_before_returning() {
        let mut doc = doc();
        let mut state = state(&doc);
        let mark = doc.journal_mark();
        let sources = crate::document::Sources::new();
        let err = Tx {
            doc: &mut doc,
            sources: &sources,
            state: &mut state,
        }
        .apply(rotate("p9.0"));
        assert_eq!((err.is_err(), doc.journal_mark()), (true, mark));
    }

    #[test]
    fn the_per_op_undo_record_covers_only_that_op() {
        let mut doc = doc();
        let mut state = state(&doc);
        let sources = crate::document::Sources::new();
        let mut tx = Tx {
            doc: &mut doc,
            sources: &sources,
            state: &mut state,
        };
        tx.apply(rotate("p2.0")).unwrap();
        let second = tx
            .apply(RotatePage::new("p2.0".parse().unwrap(), Rotation::Clockwise90))
            .unwrap();
        let before = second
            .undo
            .changes()
            .first()
            .and_then(|c| c.before.as_deref())
            .and_then(Object::as_dict)
            .and_then(|d| d.get_int("Rotate"));
        assert_eq!(before, Some(180));
    }
}
