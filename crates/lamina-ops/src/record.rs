//! Undo records (ADR 0006 §3): what a transaction changed, as before/after values per target,
//! derived from the COS journal so every op gets one without writing it by hand.

use std::sync::Arc;

use lamina_cos::{CosChange, CosMut, Dict, Name, ObjRef, Object};

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

/// One property a [`Change`] touched: a dictionary key (`key: Some`), or the whole object
/// when the value is not a dictionary on both sides (`key: None`). `None` values are absent.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct Property<'r> {
    /// The object or trailer.
    pub target: Target,
    /// The dictionary key; `None` for the whole value.
    pub key: Option<&'r Name>,
    /// The value before.
    pub before: Option<&'r Object>,
    /// The value after.
    pub after: Option<&'r Object>,
}

/// What a transaction changed ([ADR 0006] §3), in the order it first touched each target.
/// Undo reads every property as it is *now* and restores only those that still hold the
/// value this transaction set (§4); see [`Document::undo`](crate::Document::undo).
///
/// [ADR 0006]: https://github.com/laudebugs/lamina/blob/main/docs/decisions/0006-op-log-and-undo.md
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UndoRecord {
    changes: Vec<Change>,
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
        record.changes.retain(|c| c.before != c.after);
        record
    }

    /// Adds `change` after this record's: the target keeps its earliest `before` and takes
    /// the latest `after`.
    fn push(&mut self, change: Change) {
        match self.changes.iter_mut().find(|c| c.target == change.target) {
            Some(existing) => existing.after = change.after,
            None => self.changes.push(change),
        }
    }

    /// This record followed by `later`, as one (transaction coalescing, ADR 0006 §4).
    pub(crate) fn merge(&mut self, later: Self) {
        for change in later.changes {
            self.push(change);
        }
        self.changes.retain(|c| c.before != c.after);
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

    /// Every property changed: the keys that differ, for targets that are dictionaries
    /// before and after, else the whole value.
    pub fn properties(&self) -> impl Iterator<Item = Property<'_>> + '_ {
        self.changes.iter().flat_map(Change::properties)
    }
}

impl Change {
    fn properties(&self) -> Vec<Property<'_>> {
        match (dict_of(self.before.as_deref()), dict_of(self.after.as_deref())) {
            (Some(before), Some(after)) => changed_keys(before, after)
                .map(|key| Property {
                    target: self.target,
                    key: Some(key),
                    before: lookup(before, key),
                    after: lookup(after, key),
                })
                .collect(),
            _ => vec![Property {
                target: self.target,
                key: None,
                before: self.before.as_deref(),
                after: self.after.as_deref(),
            }],
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
    /// The dictionary key; `None` for the whole value.
    pub key: Option<Name>,
    /// Why.
    pub reason: SkipReason,
}

/// Restores `record`'s `before` values where the document still holds its `after` values,
/// newest change first, and reports what it left alone (ADR 0006 §4). Returns the number of
/// properties restored and the skipped ones.
pub(crate) fn revert(cos: &mut CosMut<'_>, record: &UndoRecord) -> Result<(usize, Vec<Skipped>)> {
    let mut restored = 0;
    let mut skipped = Vec::new();
    for change in record.changes.iter().rev() {
        let current = read(cos, change.target)?;
        if current == change.after {
            write(cos, change.target, change.before.as_deref())?;
            restored += change.properties().len();
            continue;
        }
        let (Some(now), Some(before), Some(after)) = (
            dict_of(current.as_deref()),
            dict_of(change.before.as_deref()),
            dict_of(change.after.as_deref()),
        ) else {
            let reason = if current.is_none() {
                SkipReason::Missing
            } else {
                SkipReason::Changed
            };
            skipped.push(Skipped {
                target: change.target,
                key: None,
                reason,
            });
            continue;
        };
        // Another change touched this dictionary: restore key by key, leaving the keys whose
        // value is no longer ours.
        let mut merged = now.clone();
        let mut any = false;
        for key in changed_keys(before, after) {
            if lookup(now, key) == lookup(after, key) {
                merged = restore_key(&merged, key, lookup(before, key), before);
                restored += 1;
                any = true;
            } else {
                skipped.push(Skipped {
                    target: change.target,
                    key: Some(key.clone()),
                    reason: SkipReason::Changed,
                });
            }
        }
        if any {
            let value = match current.as_deref() {
                Some(Object::Stream(s)) => Object::Stream(lamina_cos::Stream::new(merged, s.data.clone())),
                _ => Object::Dict(merged),
            };
            write(cos, change.target, Some(&value))?;
        }
    }
    Ok((restored, skipped))
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

/// The dictionary of a dictionary or stream value.
fn dict_of(value: Option<&Object>) -> Option<&Dict> {
    match value? {
        Object::Dict(d) => Some(d),
        Object::Stream(s) => Some(&s.dict),
        _ => None,
    }
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
        let bytes = w.finish(&[("Root", root)].into_iter().collect()).unwrap();
        Document::open(bytes, OpenOptions::default()).unwrap()
    }

    const OBJ: ObjRef = ObjRef::new(3, 0);

    /// Sets object 3 to `value` and returns the record of that change.
    fn change(doc: &mut Document, value: Object) -> UndoRecord {
        let mark = doc.journal_mark();
        doc.edit().set(OBJ, value).unwrap();
        UndoRecord::from_changes(&doc.take_changes(mark))
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
                skipped.iter().map(|s| s.key.clone()).collect::<Vec<_>>(),
                (*d.resolve(OBJ).unwrap()).clone()
            ),
            (
                1,
                vec![Some(Name::from("A"))],
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
            (restored, skipped.first().map(|s| (s.key.clone(), s.reason))),
            (0, Some((None, SkipReason::Changed)))
        );
    }

    #[test]
    fn a_record_drops_targets_that_end_where_they_started() {
        let mut d = doc();
        let mark = d.journal_mark();
        d.edit().set(OBJ, Object::Integer(1)).unwrap();
        d.edit().set(OBJ, Object::Dict(dict(&[("A", 1), ("B", 2)]))).unwrap();
        assert!(UndoRecord::from_changes(&d.take_changes(mark)).is_empty());
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
