//! Undo records (ADR 0006 §3): what a transaction changed, as before/after values per target,
//! derived from the COS journal so every op gets one without writing it by hand.

use std::collections::BTreeMap;
use std::sync::Arc;

use lamina_cos::{CosChange, CosMut, Dict, Name, ObjRef, Object, Stream};

use crate::error::Result;

/// Something a transaction changed: an indirect object, or the trailer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Target {
    /// The indirect object `r`.
    Object(ObjRef),
    /// The trailer dictionary.
    Trailer,
}

/// One target's value before and after a transaction; `None` when it did not exist (a
/// creation) or no longer exists (a deletion). The trailer is an [`Object::Dict`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Change {
    /// What changed.
    pub target: Target,
    /// The value before the transaction.
    pub before: Option<Arc<Object>>,
    /// The value after it.
    pub after: Option<Arc<Object>>,
}

/// Which part of a target a [`Property`] (or a [`Skipped`] one) is.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Field {
    /// The whole value: the target is not a dictionary (or not a stream) on both sides,
    /// or was created or deleted.
    Value,
    /// One key of a dictionary, or of a stream's dictionary.
    Key(Name),
    /// A stream's data.
    Data,
}

/// A property's value: an object, or a stream's (encoded) data.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum PropertyValue<'r> {
    /// An object (a whole value, or a dictionary key's value).
    Object(&'r Object),
    /// Stream data, as stored (encoded by the stream's `/Filter`).
    Data(&'r [u8]),
}

/// One property a [`Change`] touched. `None` values are absent.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Property<'r> {
    /// The object or trailer.
    pub target: Target,
    /// Which part of it.
    pub field: Field,
    /// The value before.
    pub before: Option<PropertyValue<'r>>,
    /// The value after.
    pub after: Option<PropertyValue<'r>>,
}

/// What a transaction changed ([ADR 0006] §3), in the order it first touched each target.
/// Undo reads every property as it is *now* and restores only those that still hold the
/// value this transaction set (§4); see [`Document::undo`](crate::Document::undo).
///
/// [ADR 0006]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0006-op-log-and-undo.md
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UndoRecord {
    changes: Vec<Change>,
    /// Where each target is in `changes`.
    index: BTreeMap<Target, usize>,
}

impl UndoRecord {
    /// The record of `changes` (oldest first): per target, the value before the first
    /// change and after the last; targets that ended where they started are left out.
    pub(crate) fn from_changes<'c>(changes: impl IntoIterator<Item = &'c CosChange>) -> Self {
        let mut record = Self::default();
        for change in changes {
            let (target, before, after) = match change {
                CosChange::Create { r, object } => (Target::Object(*r), None, Some(Arc::clone(object))),
                CosChange::Set { r, before, after } => {
                    (Target::Object(*r), Some(Arc::clone(before)), Some(Arc::clone(after)))
                }
                CosChange::Delete { r, before } => (Target::Object(*r), Some(Arc::clone(before)), None),
                CosChange::SetTrailer { before, after } => (
                    Target::Trailer,
                    Some(Arc::new(Object::Dict((**before).clone()))),
                    Some(Arc::new(Object::Dict((**after).clone()))),
                ),
                // `CosChange` is non-exhaustive; a new kind of change would need its own rule.
                _ => continue,
            };
            record.push(Change { target, before, after });
        }
        record.drop_unchanged();
        record
    }

    /// Adds `change` after this record's: the target keeps its earliest `before` and takes
    /// the latest `after`.
    fn push(&mut self, change: Change) {
        if let Some(existing) = self.index.get(&change.target).and_then(|&i| self.changes.get_mut(i)) {
            existing.after = change.after;
        } else {
            self.index.insert(change.target, self.changes.len());
            self.changes.push(change);
        }
    }

    /// Drops targets that ended where they started, and reindexes.
    fn drop_unchanged(&mut self) {
        self.changes.retain(|c| !same(c.before.as_ref(), c.after.as_ref()));
        self.index = self.changes.iter().enumerate().map(|(i, c)| (c.target, i)).collect();
    }

    /// This record followed by `later`, as one (transaction coalescing, ADR 0006 §4).
    pub(crate) fn merge(&mut self, later: Self) {
        for change in later.changes {
            self.push(change);
        }
        self.drop_unchanged();
    }

    /// True when the transaction changed nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    /// The changed targets with their values before and after.
    #[must_use]
    pub fn changes(&self) -> &[Change] {
        &self.changes
    }

    /// Every property changed: for targets that are dictionaries (or streams) before and
    /// after, each key that differs (and a stream's data when it differs); else the whole
    /// value.
    pub fn properties(&self) -> impl Iterator<Item = Property<'_>> + '_ {
        self.changes.iter().flat_map(Change::properties)
    }
}

/// A value seen as a dictionary or a stream (dictionary and data).
#[derive(Clone, Copy)]
enum Shape<'a> {
    Dict(&'a Dict),
    Stream(&'a Dict, &'a [u8]),
}

fn shape(value: Option<&Object>) -> Option<Shape<'_>> {
    match value? {
        Object::Dict(d) => Some(Shape::Dict(d)),
        Object::Stream(s) => Some(Shape::Stream(&s.dict, &s.data)),
        _ => None,
    }
}

/// The dictionaries (and both data, for streams) when `before` and `after` have the same
/// kind; `None` when the change is to the whole value.
fn same_kind<'a>(before: Option<&'a Object>, after: Option<&'a Object>) -> Option<(Shape<'a>, Shape<'a>)> {
    match (shape(before)?, shape(after)?) {
        pair @ ((Shape::Dict(_), Shape::Dict(_)) | (Shape::Stream(..), Shape::Stream(..))) => Some(pair),
        _ => None,
    }
}

const fn dict(shape: Shape<'_>) -> &Dict {
    match shape {
        Shape::Dict(d) | Shape::Stream(d, _) => d,
    }
}

const fn data(shape: Shape<'_>) -> Option<&[u8]> {
    match shape {
        Shape::Dict(_) => None,
        Shape::Stream(_, data) => Some(data),
    }
}

impl Change {
    fn properties(&self) -> Vec<Property<'_>> {
        let Some((before, after)) = same_kind(self.before.as_deref(), self.after.as_deref()) else {
            return vec![Property {
                target: self.target,
                field: Field::Value,
                before: self.before.as_deref().map(PropertyValue::Object),
                after: self.after.as_deref().map(PropertyValue::Object),
            }];
        };
        let (b, a) = (dict(before), dict(after));
        let mut out: Vec<Property<'_>> = changed_keys(b, a)
            .map(|key| Property {
                target: self.target,
                field: Field::Key(key.clone()),
                before: lookup(b, key).map(PropertyValue::Object),
                after: lookup(a, key).map(PropertyValue::Object),
            })
            .collect();
        if let (Some(bd), Some(ad)) = (data(before), data(after))
            && bd != ad
        {
            out.push(Property {
                target: self.target,
                field: Field::Data,
                before: Some(PropertyValue::Data(bd)),
                after: Some(PropertyValue::Data(ad)),
            });
        }
        out
    }

    /// `self.properties().len()`, without building them.
    fn property_count(&self) -> usize {
        match same_kind(self.before.as_deref(), self.after.as_deref()) {
            Some((before, after)) => {
                let data_changed = data(before) != data(after);
                changed_keys(dict(before), dict(after)).count() + usize::from(data_changed)
            }
            None => 1,
        }
    }
}

/// Why undo or redo left a property alone ([ADR 0006] §4).
///
/// [ADR 0006]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0006-op-log-and-undo.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SkipReason {
    /// Another change (for example a remote one) replaced the value since; it wins.
    Changed,
    /// The target no longer exists.
    Missing,
}

/// A property undo or redo did not restore.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Skipped {
    /// The object or trailer.
    pub target: Target,
    /// Which part of it.
    pub field: Field,
    /// Why.
    pub reason: SkipReason,
}

/// Stream dictionary keys that describe the data: they are restored together with it, never
/// on their own (a `/Filter` that does not match the data corrupts the stream).
const ENCODING_KEYS: [&[u8]; 7] = [
    b"Length",
    b"Filter",
    b"DecodeParms",
    b"DL",
    b"F",
    b"FFilter",
    b"FDecodeParms",
];

/// Restores `record`'s `before` values where the document still holds its `after` values,
/// newest change first, and reports what it left alone (ADR 0006 §4). Returns the number of
/// properties restored and the skipped ones.
pub(crate) fn revert(cos: &mut CosMut<'_>, record: &UndoRecord) -> Result<(usize, Vec<Skipped>)> {
    let mut restored = 0;
    let mut skipped = Vec::new();
    for change in record.changes.iter().rev() {
        let current = read(cos, change.target)?;
        if same(current.as_ref(), change.after.as_ref()) {
            write(cos, change.target, change.before.as_deref())?;
            restored += change.property_count();
            continue;
        }
        let skip_whole = |skipped: &mut Vec<Skipped>| {
            skipped.push(Skipped {
                target: change.target,
                field: Field::Value,
                reason: if current.is_none() {
                    SkipReason::Missing
                } else {
                    SkipReason::Changed
                },
            });
        };
        // Another change touched the target since. Restore key by key only when it is still
        // the same kind of value (dictionary or stream) on every side; never mix kinds.
        let Some((before, after)) = same_kind(change.before.as_deref(), change.after.as_deref()) else {
            skip_whole(&mut skipped);
            continue;
        };
        let Some((now, _)) = same_kind(current.as_deref(), change.after.as_deref()) else {
            skip_whole(&mut skipped);
            continue;
        };
        let (b, a, n) = (dict(before), dict(after), dict(now));
        // The data and the keys describing it move together: when another change replaced
        // the data, they stay as they are.
        let data_changed = data(before) != data(after);
        let data_ours = data(now) == data(after);
        let mut merged = n.clone();
        let mut new_data = data(now);
        let mut any = false;
        for key in changed_keys(b, a) {
            let tied_to_foreign_data = data_changed && !data_ours && ENCODING_KEYS.contains(&key.as_bytes());
            if !tied_to_foreign_data && lookup(n, key) == lookup(a, key) {
                merged = restore_key(&merged, key, lookup(b, key), b);
                restored += 1;
                any = true;
            } else {
                skipped.push(Skipped {
                    target: change.target,
                    field: Field::Key(key.clone()),
                    reason: SkipReason::Changed,
                });
            }
        }
        if data_changed {
            if data_ours {
                new_data = data(before);
                restored += 1;
                any = true;
            } else {
                skipped.push(Skipped {
                    target: change.target,
                    field: Field::Data,
                    reason: SkipReason::Changed,
                });
            }
        }
        if any {
            let value = match new_data {
                Some(bytes) => Object::Stream(Stream::new(merged, bytes.to_vec())),
                None => Object::Dict(merged),
            };
            write(cos, change.target, Some(&value))?;
        }
    }
    Ok((restored, skipped))
}

/// Value equality, shortcut when both share one allocation.
fn same(a: Option<&Arc<Object>>, b: Option<&Arc<Object>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => Arc::ptr_eq(a, b) || a == b,
        (None, None) => true,
        _ => false,
    }
}

fn read(cos: &CosMut<'_>, target: Target) -> Result<Option<Arc<Object>>> {
    Ok(match target {
        Target::Object(r) => cos.get(r)?,
        Target::Trailer => Some(Arc::new(Object::Dict(cos.trailer().clone()))),
    })
}

fn write(cos: &mut CosMut<'_>, target: Target, value: Option<&Object>) -> Result<()> {
    match (target, value) {
        (Target::Object(r), Some(value)) => cos.set(r, value.clone())?,
        (Target::Object(r), None) => {
            if cos.get(r)?.is_some() {
                cos.delete(r)?;
            }
        }
        (Target::Trailer, Some(Object::Dict(trailer))) => cos.set_trailer(trailer.clone()),
        // The trailer is always a dictionary; a record never holds anything else for it.
        (Target::Trailer, _) => {}
    }
    Ok(())
}

/// The keys whose values differ between `before` and `after`, in `before`'s order, then
/// `after`'s new keys.
fn changed_keys<'d>(before: &'d Dict, after: &'d Dict) -> impl Iterator<Item = &'d Name> + 'd {
    let removed_or_changed = before
        .iter()
        .filter(|(k, v)| lookup(after, k) != Some(*v))
        .map(|(k, _)| k);
    let added = after
        .iter()
        .filter(|(k, _)| lookup(before, k).is_none())
        .map(|(k, _)| k);
    removed_or_changed.chain(added)
}

/// `dict` with `key` set to `value` (removed for `None`). A key that is not in `dict` goes
/// where it was in `reference`: after the nearest key before it that `dict` still has, so
/// undoing a removal restores the key order and the bytes.
fn restore_key(dict: &Dict, key: &Name, value: Option<&Object>, reference: &Dict) -> Dict {
    let mut entries: Vec<(Name, Object)> = dict
        .iter()
        .filter(|(n, _)| *n != key)
        .map(|(n, v)| (n.clone(), v.clone()))
        .collect();
    if let Some(value) = value {
        let at = match dict.iter().position(|(n, _)| n == key) {
            Some(i) => i,
            None => reference
                .iter()
                .map(|(name, _)| name)
                .take_while(|name| *name != key)
                .filter_map(|name| entries.iter().position(|(n, _)| n == name))
                .last()
                .map_or(0, |i| i + 1),
        };
        entries.insert(at.min(entries.len()), (key.clone(), value.clone()));
    }
    entries.into_iter().collect()
}

/// `dict`'s value at `key`, compared as bytes (a name need not be UTF-8).
fn lookup<'d>(dict: &'d Dict, key: &Name) -> Option<&'d Object> {
    dict.iter().find(|(n, _)| *n == key).map(|(_, v)| v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lamina_cos::{Document, OpenOptions, Writer};

    /// Catalog 1, pages 2 (no kids), object 3 `<< /A 1 /B 2 >>`.
    fn doc() -> Document {
        let mut w = Writer::new(1, 7);
        let pages = w.add(
            [
                ("Type", Object::name("Pages")),
                ("Kids", Vec::new().into()),
                ("Count", 0.into()),
            ]
            .into_iter()
            .collect::<Dict>(),
        );
        let root = w.add(
            [("Type", Object::name("Catalog")), ("Pages", pages.into())]
                .into_iter()
                .collect::<Dict>(),
        );
        w.add(dict(&[("A", 1), ("B", 2)]));
        w.add(Stream::new(dict(&[("A", 1), ("B", 2)]), b"old".to_vec()));
        let bytes = w.finish(&[("Root", root)].into_iter().collect()).unwrap();
        Document::open(bytes, OpenOptions::default()).unwrap()
    }

    const OBJ: ObjRef = ObjRef::new(3, 0);
    const STREAM: ObjRef = ObjRef::new(4, 0);

    /// The base stream's dictionary (which has the `/Length` the writer added) with `entries`
    /// set, and `data`.
    fn stream(d: &Document, entries: &[(&str, i64)], data: &[u8]) -> Object {
        let mut dict = match &*d.resolve(STREAM).unwrap() {
            Object::Stream(s) => s.dict.clone(),
            other => panic!("{other:?}"),
        };
        for &(k, v) in entries {
            dict.insert(k, v);
        }
        Object::Stream(Stream::new(dict, data.to_vec()))
    }

    /// Sets `r` to `value` and returns the record of that change.
    fn change_of(doc: &mut Document, r: ObjRef, value: Object) -> UndoRecord {
        let mark = doc.journal_mark();
        doc.edit().set(r, value).unwrap();
        UndoRecord::from_changes(&doc.take_changes(mark).unwrap())
    }

    #[test]
    fn a_data_only_stream_change_is_one_property() {
        let mut d = doc();
        let new = stream(&d, &[], b"new");
        let record = change_of(&mut d, STREAM, new);
        assert_eq!(record.properties().count(), 1);
    }

    #[test]
    fn key_by_key_revert_of_a_stream_restores_its_data() {
        let mut d = doc();
        let (ours, remote, expected) = (
            stream(&d, &[("A", 5)], b"new"),
            stream(&d, &[("A", 5), ("B", 9)], b"new"),
            stream(&d, &[("B", 9)], b"old"),
        );
        let record = change_of(&mut d, STREAM, ours);
        d.edit().set(STREAM, remote).unwrap();
        revert(&mut d.edit(), &record).unwrap();
        assert_eq!(*d.resolve(STREAM).unwrap(), expected);
    }

    #[test]
    fn the_filter_stays_with_data_replaced_since() {
        let mut d = doc();
        let ours = stream(&d, &[("Filter", 1)], b"ours");
        let remote = stream(&d, &[("Filter", 1)], b"remote");
        let record = change_of(&mut d, STREAM, ours);
        d.edit().set(STREAM, remote.clone()).unwrap();
        let (_, skipped) = revert(&mut d.edit(), &record).unwrap();
        assert_eq!(
            (
                skipped.iter().map(|s| s.field.clone()).collect::<Vec<_>>(),
                (*d.resolve(STREAM).unwrap()).clone()
            ),
            (vec![Field::Key(Name::from("Filter")), Field::Data], remote)
        );
    }

    #[test]
    fn a_dictionary_turned_stream_and_changed_since_is_not_reverted_into_a_mix() {
        let mut d = doc();
        let ours = Object::Stream(Stream::new(dict(&[("A", 5), ("B", 2)]), b"x".to_vec()));
        let remote = Object::Stream(Stream::new(dict(&[("A", 5), ("B", 9)]), b"x".to_vec()));
        let record = change_of(&mut d, OBJ, ours);
        d.edit().set(OBJ, remote.clone()).unwrap();
        let (_, skipped) = revert(&mut d.edit(), &record).unwrap();
        assert_eq!(
            (skipped.is_empty(), (*d.resolve(OBJ).unwrap()).clone()),
            (false, remote)
        );
    }

    /// Sets object 3 to `value` and returns the record of that change.
    fn change(doc: &mut Document, value: Object) -> UndoRecord {
        let mark = doc.journal_mark();
        doc.edit().set(OBJ, value).unwrap();
        UndoRecord::from_changes(&doc.take_changes(mark).unwrap())
    }

    #[test]
    fn revert_restores_a_value_that_is_still_ours() {
        let mut d = doc();
        let record = change(&mut d, Object::Dict(dict(&[("A", 1), ("B", 3)])));
        let (restored, skipped) = revert(&mut d.edit(), &record).unwrap();
        assert_eq!((restored, skipped.len(), d.is_dirty()), (1, 0, false));
    }

    #[test]
    fn revert_leaves_a_key_changed_since_and_restores_the_others() {
        let mut d = doc();
        let record = change(&mut d, Object::Dict(dict(&[("A", 5), ("B", 6)])));
        d.edit().set(OBJ, Object::Dict(dict(&[("A", 9), ("B", 6)]))).unwrap();
        let (restored, skipped) = revert(&mut d.edit(), &record).unwrap();
        assert_eq!(
            (
                restored,
                skipped.iter().map(|s| s.field.clone()).collect::<Vec<_>>(),
                (*d.resolve(OBJ).unwrap()).clone()
            ),
            (
                1,
                vec![Field::Key(Name::from("A"))],
                Object::Dict(dict(&[("A", 9), ("B", 2)]))
            )
        );
    }

    #[test]
    fn revert_reports_a_deleted_target_missing() {
        let mut d = doc();
        let record = change(&mut d, Object::Dict(dict(&[("A", 5)])));
        d.edit().delete(OBJ).unwrap();
        let (_, skipped) = revert(&mut d.edit(), &record).unwrap();
        assert_eq!(skipped.first().map(|s| s.reason), Some(SkipReason::Missing));
    }

    #[test]
    fn revert_of_a_non_dictionary_replaced_since_is_skipped_whole() {
        let mut d = doc();
        let record = change(&mut d, Object::Integer(1));
        d.edit().set(OBJ, Object::Integer(2)).unwrap();
        let (restored, skipped) = revert(&mut d.edit(), &record).unwrap();
        assert_eq!(
            (restored, skipped.first().map(|s| (s.field.clone(), s.reason))),
            (0, Some((Field::Value, SkipReason::Changed)))
        );
    }

    #[test]
    fn a_record_drops_targets_that_end_where_they_started() {
        let mut d = doc();
        let mark = d.journal_mark();
        d.edit().set(OBJ, Object::Integer(1)).unwrap();
        d.edit().set(OBJ, Object::Dict(dict(&[("A", 1), ("B", 2)]))).unwrap();
        assert!(UndoRecord::from_changes(&d.take_changes(mark).unwrap()).is_empty());
    }

    #[test]
    fn merging_keeps_the_first_before_and_the_last_after() {
        let mut d = doc();
        let mut first = change(&mut d, Object::Integer(1));
        first.merge(change(&mut d, Object::Integer(2)));
        let c = first.changes().first().unwrap();
        assert_eq!(
            (c.before.as_deref().cloned(), c.after.as_deref().cloned()),
            (
                Some(Object::Dict(dict(&[("A", 1), ("B", 2)]))),
                Some(Object::Integer(2))
            )
        );
    }

    fn dict(entries: &[(&str, i64)]) -> Dict {
        entries.iter().map(|&(k, v)| (k, v)).collect()
    }

    #[test]
    fn restoring_a_removed_key_puts_it_back_in_place() {
        let reference = dict(&[("A", 1), ("B", 2), ("C", 3)]);
        let restored = restore_key(
            &dict(&[("A", 1), ("C", 3)]),
            &Name::from("B"),
            Some(&Object::Integer(2)),
            &reference,
        );
        assert_eq!(restored, reference);
    }

    #[test]
    fn restoring_a_removed_first_key_puts_it_first() {
        let reference = dict(&[("A", 1), ("B", 2)]);
        let restored = restore_key(
            &dict(&[("B", 2)]),
            &Name::from("A"),
            Some(&Object::Integer(1)),
            &reference,
        );
        assert_eq!(restored, reference);
    }

    #[test]
    fn changed_keys_lists_removed_changed_and_added() {
        let before = dict(&[("A", 1), ("B", 2), ("C", 3)]);
        let after = dict(&[("A", 1), ("C", 4), ("D", 5)]);
        let keys: Vec<_> = changed_keys(&before, &after).cloned().collect();
        assert_eq!(keys, [Name::from("B"), Name::from("C"), Name::from("D")]);
    }
}
