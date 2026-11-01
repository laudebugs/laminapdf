//! The overlay ([ADR 0005] §3): every change to a document lands here, never in the base.
//!
//! The base (the opened bytes) is immutable. [`CosMut`], obtained from
//! [`Document::edit`], is the only way to create, change or delete an object or change the
//! trailer; each change is recorded in the overlay and in its journal, so dirty tracking is
//! automatic and `PDFBox`'s lost-changes bug is unrepresentable. Resolution
//! ([`Document::resolve`], [`Document::object`], [`Document::trailer`],
//! [`Document::object_numbers`]) reads the overlay first, then the base.
//!
//! * An entry whose value equals the base object is dropped, so changing a value back
//!   leaves nothing dirty ([`Document::is_dirty`]). A base object that cannot be loaded
//!   counts as `null` here: it can be overwritten or deleted, and setting it to `null`
//!   leaves it as it is.
//! * New objects get fresh numbers above every xref section's highest number and the
//!   trailer's `/Size`, within [`Limits::max_object_number`](crate::Limits::max_object_number).
//!   Numbers are never reused in a session: a deleted or rolled-back creation keeps its
//!   number reserved, so redo recreates the same object.
//! * The journal records every change with the overlay state it replaced, so a caller
//!   (`laminapdf-ops`' transactions and savepoints) can take a [`JournalMark`] and roll back to
//!   it exactly ([`Document::rollback_to`]), or take the changes since it as
//!   [`CosChange`]s ([`Document::take_changes`]). A mark that no longer names a position in
//!   the journal (its records were taken or rolled back and others recorded since) is
//!   refused.
//!
//! **API review note:** `edit()` is public, so code holding a bare `laminapdf_cos::Document`
//! can change it outside a `laminapdf-ops` transaction. Such changes still go through the
//! overlay and the journal (they are dirty-tracked and can be rolled back); `laminapdf-ops`
//! keeps its document private so its callers can only write through transactions.
//!
//! [ADR 0005]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0005-document-model.md

use std::cell::{Cell, OnceCell};
use std::collections::BTreeMap;
use std::sync::Arc;

use laminapdf_syntax::{Dict, ObjRef, Object};

use crate::error::{Error, Result};
use crate::reader::{Document, OpenOptions};
use crate::source::Source;

/// An overlay entry for one object number.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum OverlayEntry {
    /// An object created in this session (generation 0, a number above the base's).
    New(Arc<Object>),
    /// A base object with a new value.
    Modified(Arc<Object>),
    /// A base object that was deleted.
    Deleted,
}

impl OverlayEntry {
    /// The object's value, `None` for [`OverlayEntry::Deleted`].
    #[must_use]
    pub fn object(&self) -> Option<&Arc<Object>> {
        match self {
            Self::New(o) | Self::Modified(o) => Some(o),
            Self::Deleted => None,
        }
    }
}

/// One change made through [`CosMut`], with the effective values (overlay, then base)
/// before and after it.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum CosChange {
    /// `r` did not exist (never did, was deleted, or was a reserved number) and now holds
    /// `object`.
    Create {
        /// The object.
        r: ObjRef,
        /// Its value.
        object: Arc<Object>,
    },
    /// `r` changed value.
    Set {
        /// The object.
        r: ObjRef,
        /// The value before.
        before: Arc<Object>,
        /// The value after.
        after: Arc<Object>,
    },
    /// `r` was deleted.
    Delete {
        /// The object.
        r: ObjRef,
        /// The value it had.
        before: Arc<Object>,
    },
    /// The trailer changed (for example a new `/Info`).
    SetTrailer {
        /// The trailer before.
        before: Arc<Dict>,
        /// The trailer after.
        after: Arc<Dict>,
    },
}

/// A position in the journal: [`Document::rollback_to`] undoes every change recorded after
/// it, [`Document::take_changes`] returns them. It stays valid while the journal still has
/// the records it was taken after.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JournalMark {
    /// Records in the journal when the mark was taken.
    len: usize,
    /// The sequence number of the last of them (0 for none).
    last: u64,
}

/// What a change replaced, so rolling back restores the overlay exactly.
#[derive(Debug, Clone)]
enum Prior {
    Object(u32, Option<OverlayEntry>),
    Trailer(Option<Arc<Dict>>),
}

#[derive(Debug, Clone)]
struct Record {
    change: CosChange,
    prior: Prior,
    /// Unique in the session, so a mark can tell its record from a later one at the same
    /// position.
    seq: u64,
}

/// A document's overlay: new, modified and deleted objects and a changed trailer, over the
/// immutable base ([ADR 0005] §3). Read it through [`Document::overlay`]; change it only
/// through [`CosMut`].
///
/// [ADR 0005]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0005-document-model.md
#[derive(Debug, Default)]
pub struct Overlay {
    objects: BTreeMap<u32, OverlayEntry>,
    trailer: Option<Arc<Dict>>,
    journal: Vec<Record>,
    /// The sequence number of the last journal record.
    seq: u64,
    /// The next number to hand out (`u64` so `u32::MAX` itself can be allocated); `None`
    /// until the first allocation.
    next_number: Cell<Option<u64>>,
    /// The first number a new object may get, computed from the base once.
    first_new: OnceCell<u64>,
}

impl Overlay {
    /// True when nothing differs from the base.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty() && self.trailer.is_none()
    }

    /// The number of differences from the base: object entries, plus one for a changed
    /// trailer (so `len() == 0` exactly when [`Overlay::is_empty`]).
    #[must_use]
    pub fn len(&self) -> usize {
        self.objects.len() + usize::from(self.trailer.is_some())
    }

    /// The entry for object `num`, if the overlay has one.
    #[must_use]
    pub fn get(&self, num: u32) -> Option<&OverlayEntry> {
        self.objects.get(&num)
    }

    /// Every object entry, by ascending number.
    pub fn iter(&self) -> impl Iterator<Item = (u32, &OverlayEntry)> + '_ {
        self.objects.iter().map(|(&n, e)| (n, e))
    }

    /// The changed trailer, when it differs from the base's.
    #[must_use]
    pub fn trailer(&self) -> Option<&Dict> {
        self.trailer.as_deref()
    }

    /// The records after `mark`, or an error for a stale mark.
    fn after(&self, mark: JournalMark) -> Result<&[Record]> {
        let valid = mark.len == 0 || self.journal.get(mark.len - 1).is_some_and(|r| r.seq == mark.last);
        match self.journal.get(mark.len..) {
            Some(records) if valid => Ok(records),
            _ => Err(Error::InvalidStructure {
                kind: "journal mark",
                message: "the mark's records were taken or rolled back",
            }),
        }
    }
}

impl Document {
    /// The overlay: what this session changed relative to the base.
    #[must_use]
    pub fn overlay(&self) -> &Overlay {
        self.overlay_ref()
    }

    /// True when the overlay is non-empty: a save would write something (ADR 0006 §7's COS
    /// dirty level).
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        !self.overlay_ref().is_empty()
    }

    /// The only way to change the document: a [`CosMut`] that records every change in the
    /// overlay and its journal. `laminapdf-ops` wraps it in transactions; code that edits at
    /// the COS level directly should take a [`Document::journal_mark`] first and
    /// [`Document::rollback_to`] it on failure.
    pub fn edit(&mut self) -> CosMut<'_> {
        CosMut { doc: self }
    }

    /// The current end of the journal.
    #[must_use]
    pub fn journal_mark(&self) -> JournalMark {
        let journal = &self.overlay_ref().journal;
        JournalMark {
            len: journal.len(),
            last: journal.last().map_or(0, |r| r.seq),
        }
    }

    /// Undoes every change recorded after `mark`, newest first, restoring the overlay
    /// exactly as it was at the mark. Reserved numbers stay reserved.
    ///
    /// # Errors
    /// [`Error::InvalidStructure`] for a stale mark (nothing is changed).
    pub fn rollback_to(&mut self, mark: JournalMark) -> Result<()> {
        self.overlay_ref().after(mark)?;
        let overlay = self.overlay_mut();
        while overlay.journal.len() > mark.len {
            let Some(record) = overlay.journal.pop() else { break };
            match record.prior {
                Prior::Object(num, Some(entry)) => {
                    overlay.objects.insert(num, entry);
                }
                Prior::Object(num, None) => {
                    overlay.objects.remove(&num);
                }
                Prior::Trailer(trailer) => overlay.trailer = trailer,
            }
        }
        Ok(())
    }

    /// The changes recorded after `mark`, oldest first (the journal keeps them).
    ///
    /// # Errors
    /// [`Error::InvalidStructure`] for a stale mark.
    pub fn changes_since(&self, mark: JournalMark) -> Result<impl Iterator<Item = &CosChange> + '_> {
        Ok(self.overlay_ref().after(mark)?.iter().map(|r| &r.change))
    }

    /// Removes the changes recorded after `mark` from the journal and returns them, oldest
    /// first. The overlay keeps them: this commits them, so they can no longer be rolled
    /// back, and marks taken after `mark` become stale.
    ///
    /// # Errors
    /// [`Error::InvalidStructure`] for a stale mark (nothing is changed).
    pub fn take_changes(&mut self, mark: JournalMark) -> Result<Vec<CosChange>> {
        self.overlay_ref().after(mark)?;
        let journal = &mut self.overlay_mut().journal;
        Ok(journal.drain(mark.len..).map(|r| r.change).collect())
    }

    /// Reserves a fresh object number (generation 0) without creating the object; a later
    /// [`CosMut::set`] creates it. Numbers start above every xref section's highest number
    /// and the trailer's `/Size` (so a dangling reference below `/Size` never resolves to a
    /// new object) and are never handed out twice.
    ///
    /// # Errors
    /// [`Error::InvalidStructure`] once the next number would exceed
    /// [`Limits::max_object_number`](crate::Limits::max_object_number).
    pub fn reserve_number(&self) -> Result<ObjRef> {
        let first = self.first_new_number();
        let cell = &self.overlay_ref().next_number;
        let next = cell.get().unwrap_or(first);
        let num = u32::try_from(next)
            .ok()
            .filter(|n| *n <= self.limits().max_object_number())
            .ok_or(Error::InvalidStructure {
                kind: "object number",
                message: "no object number left within Limits::max_object_number",
            })?;
        cell.set(Some(next + 1));
        Ok(ObjRef::new(num, 0))
    }

    /// Whether `num` was handed out by [`Document::reserve_number`] (or [`CosMut::create`]).
    #[must_use]
    pub fn is_reserved_number(&self, num: u32) -> bool {
        let first = self.first_new_number();
        self.overlay_ref()
            .next_number
            .get()
            .is_some_and(|next| u64::from(num) >= first && u64::from(num) < next)
    }

    /// The lowest `/Size` an update may write: the base trailer's `/Size`, and above every
    /// xref entry (the writers in `laminapdf-save` and the number allocator share this rule).
    #[must_use]
    pub fn size_floor(&self) -> i64 {
        let above_entries = self.max_object_number().map_or(0, |n| i64::from(n) + 1);
        self.base_trailer().get_int("Size").unwrap_or(0).max(above_entries)
    }

    /// The number [`Document::reserve_number`] would hand out next, without reserving it: at
    /// least [`Document::size_floor`], and above every number reserved in this session (up to
    /// `u32::MAX + 1` once every number is taken). A writer's `/Size` is never below it, so a
    /// reserved number never names another object after a save.
    #[must_use]
    pub fn next_number(&self) -> u64 {
        self.overlay_ref()
            .next_number
            .get()
            .unwrap_or_else(|| self.first_new_number())
    }

    /// Makes `source` the new base, after a save wrote it ([ADR 0005] §3, steps 1–3): opens it
    /// with this document's [`Limits`](crate::Limits) and strict mode, loads its newest xref,
    /// and drops the overlay and its journal. The warnings raised so far are kept (before the
    /// new base's own), and the journal's sequence numbers continue, so a mark taken before
    /// the rebase is refused as stale afterwards.
    ///
    /// `reserved` says what happens to the numbers reserved in this session:
    /// [`Reserved::Keep`] when `source` is the old base plus an appended revision (an
    /// incremental save keeps every number), so a reserved number stays reserved and redo can
    /// recreate its object; [`Reserved::Release`] when `source` is a new numbering (a full
    /// rewrite): allocation starts afresh above the new base.
    ///
    /// This is the mechanism under `laminapdf-save`'s writers and `laminapdf_ops::Document::rebase`;
    /// the overlay's changes are discarded, so call it only with bytes that contain them. A
    /// password is not carried over (the writers refuse encrypted bases).
    ///
    /// # Errors
    /// As [`Document::open`]; on error the document is unchanged.
    ///
    /// [ADR 0005]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0005-document-model.md
    pub fn rebase(&mut self, source: impl Into<Source>, reserved: Reserved) -> Result<()> {
        let options = OpenOptions::default()
            .with_limits(*self.limits())
            .with_strict(self.is_strict());
        let mut next = Self::open(source, options)?;
        next.carry_warnings(self);
        let old = self.overlay_ref();
        next.overlay_mut().seq = old.seq;
        let fresh = next.overlay_ref();
        // Reserved numbers start where the old range did (they are absent from the new base,
        // which was written with `/Size` above them); new ones start above the new base.
        if reserved == Reserved::Keep
            && let Some(&first) = old.first_new.get()
        {
            let floor = next.first_new_number_fresh();
            fresh.first_new.get_or_init(|| first.min(floor));
            fresh
                .next_number
                .set(Some(old.next_number.get().unwrap_or(first).max(floor)));
        }
        *self = next;
        Ok(())
    }

    /// One past the highest number reserved in this session, `None` when none was.
    #[must_use]
    pub fn reserved_end(&self) -> Option<u64> {
        self.overlay_ref().next_number.get()
    }

    /// [`Document::first_new_number`] as the base alone gives it (no carried-over state).
    fn first_new_number_fresh(&self) -> u64 {
        u64::try_from(self.size_floor().max(1)).unwrap_or(1)
    }

    /// The first number a new object may get: [`Document::size_floor`], at least 1.
    fn first_new_number(&self) -> u64 {
        *self
            .overlay_ref()
            .first_new
            .get_or_init(|| self.first_new_number_fresh())
    }

    /// Object `num` as the overlay has it (the one lookup [`CosMut::get`] and resolution
    /// share).
    pub(crate) fn overlay_value(&self, num: u32, gen_nr: Option<u16>) -> InOverlay<'_> {
        let Some(entry) = self.overlay_ref().objects.get(&num) else {
            return InOverlay::NoEntry;
        };
        let matches = gen_nr.is_none_or(|g| self.overlay_generation_matches(num, g));
        match entry.object() {
            Some(value) if matches => InOverlay::Value(value),
            _ => InOverlay::Absent,
        }
    }

    /// Whether `gen_nr` is the generation of overlay object `num`: 0 for a new object, the
    /// base entry's for a modified or deleted one.
    pub(crate) fn overlay_generation_matches(&self, num: u32, gen_nr: u16) -> bool {
        match self.overlay_ref().objects.get(&num) {
            Some(OverlayEntry::New(_)) => gen_nr == 0,
            _ => self.base_has(ObjRef::new(num, gen_nr)),
        }
    }
}

/// What [`Document::rebase`] does with the numbers reserved in the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Reserved {
    /// Keep them reserved: the new base is the old one plus an appended revision.
    Keep,
    /// Release them: the new base has a new numbering (a full rewrite).
    Release,
}

/// What the overlay says about an object number.
pub(crate) enum InOverlay<'a> {
    /// Its current value.
    Value(&'a Arc<Object>),
    /// It does not exist: deleted, or named with a generation that is not its own.
    Absent,
    /// The overlay has no entry; the base decides.
    NoEntry,
}

/// Write access to a document, through the overlay ([ADR 0005] §3). Every change is
/// journaled ([`Document::journal_mark`], [`Document::rollback_to`]); none touches the base.
///
/// Values are compared with the base on every write: setting an object (or the trailer)
/// back to its base value drops the overlay entry.
///
/// [ADR 0005]: https://github.com/laudebugs/laminapdf/blob/main/docs/decisions/0005-document-model.md
#[derive(Debug)]
pub struct CosMut<'d> {
    doc: &'d mut Document,
}

impl CosMut<'_> {
    /// The document as it is now (overlay over base), for reading.
    #[must_use]
    pub fn document(&self) -> &Document {
        self.doc
    }

    /// The current value of object `r` (overlay, then base), without following a top-level
    /// reference; `None` when it does not exist (free, deleted, never created, or `r`'s
    /// generation is not the object's). A base object that cannot be loaded reads as `null`.
    ///
    /// # Errors
    /// Strict-mode refusals and unavailable input ([`Error::DataPending`],
    /// [`Error::Source`]) while loading a base object.
    pub fn get(&self, r: ObjRef) -> Result<Option<Arc<Object>>> {
        match self.doc.overlay_value(r.num, Some(r.gen_nr)) {
            InOverlay::Value(value) => Ok(Some(Arc::clone(value))),
            InOverlay::NoEntry if self.doc.base_has(r) => self.doc.base_value(r.num).map(Some),
            InOverlay::Absent | InOverlay::NoEntry => Ok(None),
        }
    }

    /// Creates a new object with a fresh number ([`Document::reserve_number`]).
    ///
    /// # Errors
    /// As [`Document::reserve_number`].
    pub fn create(&mut self, object: Object) -> Result<ObjRef> {
        let r = self.doc.reserve_number()?;
        self.set(r, object)?;
        Ok(r)
    }

    /// Sets object `r` to `object`: a base object (also one deleted in this session), an
    /// object created in this session, or a reserved number (which creates it). An entry
    /// equal to the base object is dropped; a write that changes nothing records nothing.
    ///
    /// # Errors
    /// [`Error::InvalidStructure`] when `r` is none of those (or its generation does not
    /// match); as [`CosMut::get`].
    pub fn set(&mut self, r: ObjRef, object: Object) -> Result<()> {
        let before = self.get(r)?;
        if before.as_deref() == Some(&object) {
            return Ok(());
        }
        let prior = self.doc.overlay_ref().objects.get(&r.num).cloned();
        let object = Arc::new(object);
        let entry = if self.doc.base_has(r) {
            // Without an overlay entry the current value is the base's, which differs.
            let back_to_base = prior.is_some() && *self.doc.base_value(r.num)? == *object;
            (!back_to_base).then(|| OverlayEntry::Modified(Arc::clone(&object)))
        } else if r.gen_nr == 0 && (matches!(prior, Some(OverlayEntry::New(_))) || self.doc.is_reserved_number(r.num)) {
            Some(OverlayEntry::New(Arc::clone(&object)))
        } else {
            return Err(unknown_object());
        };
        let change = match before {
            Some(before) => CosChange::Set {
                r,
                before,
                after: object,
            },
            None => CosChange::Create { r, object },
        };
        self.record(change, Prior::Object(r.num, prior));
        let objects = &mut self.doc.overlay_mut().objects;
        match entry {
            Some(entry) => objects.insert(r.num, entry),
            None => objects.remove(&r.num),
        };
        Ok(())
    }

    /// Deletes object `r`. A base object becomes [`OverlayEntry::Deleted`]; an object created
    /// in this session leaves the overlay, and its number stays reserved.
    ///
    /// # Errors
    /// [`Error::InvalidStructure`] when `r` does not exist; as [`CosMut::get`].
    pub fn delete(&mut self, r: ObjRef) -> Result<()> {
        let before = self.get(r)?.ok_or_else(unknown_object)?;
        let prior = self.doc.overlay_ref().objects.get(&r.num).cloned();
        self.record(CosChange::Delete { r, before }, Prior::Object(r.num, prior));
        let base = self.doc.base_has(r);
        let objects = &mut self.doc.overlay_mut().objects;
        if base {
            objects.insert(r.num, OverlayEntry::Deleted);
        } else {
            objects.remove(&r.num);
        }
        Ok(())
    }

    /// The trailer as it is now (overlay, then base).
    #[must_use]
    pub fn trailer(&self) -> &Dict {
        self.doc.trailer()
    }

    /// Replaces the trailer (for example to point `/Info` at a new object). A trailer equal
    /// to the base's drops the overlay's.
    pub fn set_trailer(&mut self, trailer: Dict) {
        if *self.doc.trailer() == trailer {
            return;
        }
        let before = Arc::new(self.doc.trailer().clone());
        let after = Arc::new(trailer);
        let prior = self.doc.overlay_ref().trailer.clone();
        self.record(
            CosChange::SetTrailer {
                before,
                after: Arc::clone(&after),
            },
            Prior::Trailer(prior),
        );
        let base_equal = *self.doc.base_trailer() == *after;
        self.doc.overlay_mut().trailer = (!base_equal).then_some(after);
    }

    fn record(&mut self, change: CosChange, prior: Prior) {
        let overlay = self.doc.overlay_mut();
        overlay.seq += 1;
        let seq = overlay.seq;
        overlay.journal.push(Record { change, prior, seq });
    }
}

const fn unknown_object() -> Error {
    Error::InvalidStructure {
        kind: "overlay",
        message: "no such object (not in the base, not created and not a reserved number)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OpenOptions;

    use crate::test_files::classic;

    /// Catalog 1, pages 2, one page 3 (`/Rotate 90`), Info 4; `/Size 5`.
    fn doc_with(limits: Option<crate::Limits>) -> Document {
        let bytes = classic(
            &[
                b"<< /Type /Catalog /Pages 2 0 R >>",
                b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                b"<< /Type /Page /Parent 2 0 R /Rotate 90 >>",
                b"<< /Title (Old) >>",
            ],
            "<< /Size 5 /Root 1 0 R /Info 4 0 R >>",
        );
        let options = limits.map_or_else(OpenOptions::default, |l| OpenOptions::default().with_limits(l));
        Document::open(bytes, options).unwrap()
    }

    fn doc() -> Document {
        doc_with(None)
    }

    const PAGE: ObjRef = ObjRef::new(3, 0);

    fn rotated(doc: &Document, degrees: i64) -> Object {
        let mut dict = doc.resolve(PAGE).unwrap().as_dict().unwrap().clone();
        dict.insert("Rotate", degrees);
        Object::Dict(dict)
    }

    #[test]
    fn a_new_document_is_clean() {
        assert!(!doc().is_dirty());
    }

    #[test]
    fn set_is_read_back_through_resolution() {
        let mut d = doc();
        let page = rotated(&d, 180);
        d.edit().set(PAGE, page.clone()).unwrap();
        assert_eq!(*d.resolve(PAGE).unwrap(), page);
    }

    #[test]
    fn set_marks_the_document_dirty_with_a_modified_entry() {
        let mut d = doc();
        let page = rotated(&d, 180);
        d.edit().set(PAGE, page).unwrap();
        assert!(matches!(d.overlay().get(3), Some(OverlayEntry::Modified(_))));
    }

    #[test]
    fn setting_the_base_value_back_drops_the_entry() {
        let mut d = doc();
        let (changed, base) = (rotated(&d, 180), rotated(&d, 90));
        d.edit().set(PAGE, changed).unwrap();
        d.edit().set(PAGE, base).unwrap();
        assert!(!d.is_dirty());
    }

    #[test]
    fn setting_an_unchanged_value_records_nothing() {
        let mut d = doc();
        let base = rotated(&d, 90);
        let mark = d.journal_mark();
        d.edit().set(PAGE, base).unwrap();
        assert_eq!(d.changes_since(mark).unwrap().count(), 0);
    }

    #[test]
    fn create_numbers_above_the_trailer_size() {
        let mut d = doc();
        let r = d.edit().create(Object::Integer(1)).unwrap();
        assert_eq!(r, ObjRef::new(5, 0));
    }

    #[test]
    fn created_objects_resolve_and_are_new() {
        let mut d = doc();
        let r = d.edit().create(Object::Integer(7)).unwrap();
        assert_eq!(
            (
                d.resolve(r).unwrap().as_int(),
                matches!(d.overlay().get(r.num), Some(OverlayEntry::New(_)))
            ),
            (Some(7), true)
        );
    }

    #[test]
    fn deleting_a_created_object_leaves_the_overlay_and_keeps_the_number_reserved() {
        let mut d = doc();
        let r = d.edit().create(Object::Integer(7)).unwrap();
        d.edit().delete(r).unwrap();
        let next = d.edit().create(Object::Integer(8)).unwrap();
        assert_eq!(
            (d.overlay().get(r.num), d.is_reserved_number(r.num), next.num),
            (None, true, r.num + 1)
        );
    }

    #[test]
    fn a_reserved_number_can_be_recreated() {
        let mut d = doc();
        let r = d.edit().create(Object::Integer(7)).unwrap();
        d.edit().delete(r).unwrap();
        d.edit().set(r, Object::Integer(7)).unwrap();
        assert_eq!(d.resolve(r).unwrap().as_int(), Some(7));
    }

    #[test]
    fn deleting_a_base_object_resolves_to_null() {
        let mut d = doc();
        d.edit().delete(PAGE).unwrap();
        assert_eq!(
            (*d.resolve(PAGE).unwrap() == Object::Null, d.overlay().get(3)),
            (true, Some(&OverlayEntry::Deleted))
        );
    }

    #[test]
    fn restoring_a_deleted_base_object_to_its_value_drops_the_entry() {
        let mut d = doc();
        let base = rotated(&d, 90);
        d.edit().delete(PAGE).unwrap();
        d.edit().set(PAGE, base).unwrap();
        assert!(!d.is_dirty());
    }

    #[test]
    fn unknown_numbers_and_stale_generations_are_rejected() {
        let mut d = doc();
        let mut edit = d.edit();
        assert!(
            edit.set(ObjRef::new(99, 0), Object::Null).is_err()
                && edit.set(ObjRef::new(3, 1), Object::Null).is_err()
                && edit.delete(ObjRef::new(99, 0)).is_err()
        );
    }

    #[test]
    fn rollback_restores_the_overlay_exactly() {
        let mut d = doc();
        let first = rotated(&d, 180);
        d.edit().set(PAGE, first.clone()).unwrap();
        let mark = d.journal_mark();
        let second = rotated(&d, 270);
        let mut edit = d.edit();
        edit.set(PAGE, second).unwrap();
        let r = edit.create(Object::Integer(1)).unwrap();
        edit.delete(ObjRef::new(4, 0)).unwrap();
        let mut trailer = edit.trailer().clone();
        trailer.remove("Info");
        edit.set_trailer(trailer);
        d.rollback_to(mark).unwrap();
        assert_eq!(
            (
                d.overlay().len(),
                (*d.resolve(PAGE).unwrap()).clone(),
                d.overlay().get(r.num),
                d.trailer().contains_key("Info")
            ),
            (1, first, None, true)
        );
    }

    #[test]
    fn rollback_to_the_start_leaves_the_document_clean() {
        let mut d = doc();
        let mark = d.journal_mark();
        let page = rotated(&d, 0);
        d.edit().set(PAGE, page).unwrap();
        d.edit().create(Object::Null).unwrap();
        d.rollback_to(mark).unwrap();
        assert!(!d.is_dirty());
    }

    #[test]
    fn take_changes_commits_and_reports_create_set_delete() {
        let mut d = doc();
        let mark = d.journal_mark();
        let page = rotated(&d, 0);
        let mut edit = d.edit();
        let r = edit.create(Object::Integer(1)).unwrap();
        edit.set(PAGE, page).unwrap();
        edit.delete(ObjRef::new(4, 0)).unwrap();
        let kinds: Vec<_> = d
            .take_changes(mark)
            .unwrap()
            .iter()
            .map(|c| match c {
                CosChange::Create { r, .. } => ("create", r.num),
                CosChange::Set { r, .. } => ("set", r.num),
                CosChange::Delete { r, .. } => ("delete", r.num),
                CosChange::SetTrailer { .. } => ("trailer", 0),
            })
            .collect();
        assert_eq!(
            (kinds, d.journal_mark()),
            (
                vec![("create", r.num), ("set", 3), ("delete", 4)],
                JournalMark { len: 0, last: 0 }
            )
        );
    }

    #[test]
    fn a_trailer_set_back_to_the_base_drops_it() {
        let mut d = doc();
        let base = d.trailer().clone();
        let mut changed = base.clone();
        changed.remove("Info");
        d.edit().set_trailer(changed);
        let dirty = d.is_dirty();
        d.edit().set_trailer(base);
        assert_eq!((dirty, d.is_dirty()), (true, false));
    }

    #[test]
    fn allocation_never_reuses_a_number() {
        let d = doc();
        let numbers: std::collections::BTreeSet<_> = (0..100).map(|_| d.reserve_number().unwrap().num).collect();
        assert_eq!(numbers, (5..105).collect());
    }

    #[test]
    fn allocation_stops_at_the_maximum_object_number() {
        let d = doc_with(Some(crate::Limits::default().with_max_object_number(5)));
        assert!(d.reserve_number().is_ok() && d.reserve_number().is_err());
    }

    #[test]
    fn existing_numbers_are_not_reserved() {
        let d = doc();
        d.reserve_number().unwrap();
        assert!(!d.is_reserved_number(3) && d.is_reserved_number(5));
    }

    #[test]
    fn a_mark_whose_records_were_taken_is_refused() {
        let mut d = doc();
        let outer = d.journal_mark();
        d.edit().create(Object::Integer(1)).unwrap();
        let inner = d.journal_mark();
        d.edit().create(Object::Integer(2)).unwrap();
        d.take_changes(outer).unwrap();
        d.edit().create(Object::Integer(3)).unwrap();
        d.edit().create(Object::Integer(4)).unwrap();
        assert!(d.rollback_to(inner).is_err() && d.take_changes(inner).is_err() && d.changes_since(inner).is_err());
    }

    #[test]
    fn a_mark_at_the_take_point_stays_valid() {
        let mut d = doc();
        d.edit().create(Object::Integer(1)).unwrap();
        let mark = d.journal_mark();
        d.edit().create(Object::Integer(2)).unwrap();
        d.take_changes(mark).unwrap();
        assert!(d.rollback_to(mark).is_ok());
    }

    #[test]
    fn allocation_can_hand_out_u32_max_once() {
        let mut bytes = b"%PDF-1.7\n".to_vec();
        let offset = bytes.len();
        bytes.extend_from_slice(b"1 0 obj << /Type /Catalog >> endobj\n");
        let xref = bytes.len();
        bytes.extend_from_slice(
            format!(
                "xref\n0 2\n0000000000 65535 f\r\n{offset:010} 00000 n\r\n\
                 trailer << /Root 1 0 R /Size 4294967295 >>\nstartxref\n{xref}\n%%EOF\n"
            )
            .as_bytes(),
        );
        let limits = crate::Limits::default().with_max_object_number(u32::MAX);
        let d = Document::open(bytes, OpenOptions::default().with_limits(limits)).unwrap();
        assert!(!d.was_repaired());
        assert_eq!(
            (d.reserve_number().map(|r| r.num).ok(), d.reserve_number().is_err()),
            (Some(u32::MAX), true)
        );
    }

    /// The [`doc`] file with `/Size 9`: a base a save could have written.
    fn saved_bytes() -> Vec<u8> {
        classic(
            &[
                b"<< /Type /Catalog /Pages 2 0 R >>",
                b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
                b"<< /Type /Page /Parent 2 0 R /Rotate 90 >>",
                b"<< /Title (Old) >>",
            ],
            "<< /Size 9 /Root 1 0 R /Info 4 0 R >>",
        )
    }

    #[test]
    fn a_mark_from_before_a_rebase_is_stale_after_it() {
        let mut d = doc();
        d.edit().create(Object::Integer(1)).unwrap();
        let mark = d.journal_mark();
        d.rebase(saved_bytes(), Reserved::Keep).unwrap();
        d.edit().create(Object::Integer(2)).unwrap();
        assert!(d.rollback_to(mark).is_err());
    }

    #[test]
    fn a_rebase_keeps_the_warnings_raised_so_far() {
        let mut d = doc();
        d.resolve(ObjRef::new(3, 1)).unwrap(); // stale-generation
        d.rebase(saved_bytes(), Reserved::Keep).unwrap();
        assert_eq!(
            d.warnings().iter().map(crate::Warning::code).collect::<Vec<_>>(),
            ["stale-generation"]
        );
    }

    #[test]
    fn keep_carries_reserved_numbers_over_a_rebase() {
        let mut d = doc();
        let r = d.edit().create(Object::Integer(1)).unwrap(); // 5
        d.edit().delete(r).unwrap();
        d.rebase(saved_bytes(), Reserved::Keep).unwrap();
        assert_eq!((d.is_reserved_number(5), d.reserve_number().unwrap().num), (true, 9));
    }

    #[test]
    fn release_starts_allocation_afresh_above_the_new_base() {
        let mut d = doc();
        for _ in 0..20 {
            d.reserve_number().unwrap();
        }
        d.rebase(saved_bytes(), Reserved::Release).unwrap();
        assert_eq!((d.is_reserved_number(5), d.reserve_number().unwrap().num), (false, 9));
    }
}
