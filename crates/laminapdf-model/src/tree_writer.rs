//! Writing name trees and number trees (ISO 32000-2 §7.9.6, §7.9.7) from a sorted set of
//! entries: a fresh, balanced tree whose keys are sorted, whose leaves and intermediate nodes
//! hold at most [`TreeWriter::max_kids`] entries or kids, and whose `/Limits` are right.
//!
//! The writers never edit a tree in place: a changed tree is written again from its entries
//! and the holder (the catalog's `/PageLabels`, `/Names /Dests`, a `/ParentTree`, …) is pointed
//! at the new root. The old tree's nodes are left unreferenced; a full rewrite drops them.

use std::collections::BTreeMap;

use laminapdf_cos::{CosMut, Dict, ObjRef, Object, PdfString, Result};

/// Writes name and number trees, balanced, with a bounded node size.
///
/// ```
/// use std::collections::BTreeMap;
/// use laminapdf_cos::Object;
/// use laminapdf_model::{Document, OpenOptions, TreeWriter};
/// # use laminapdf_save::test_support::Writer;
/// # let mut w = Writer::new(1, 7);
/// # let pages = w.add([("Type", Object::name("Pages")), ("Kids", Vec::<Object>::new().into()), ("Count", 0.into())].into_iter().collect::<laminapdf_cos::Dict>());
/// # let root = w.add([("Type", Object::name("Catalog")), ("Pages", pages.into())].into_iter().collect::<laminapdf_cos::Dict>());
/// # let bytes = w.finish(&[("Root", root)].into_iter().collect())?;
/// let mut doc = Document::open(bytes, OpenOptions::default())?;
/// let entries: BTreeMap<i64, Object> = (0..200).map(|i| (i * 2, Object::Integer(i))).collect();
/// let root = TreeWriter::new().with_max_kids(16).write_numbers(&mut doc.cos_mut().edit(), &entries)?;
/// let tree = doc.number_tree(root.into());
/// assert_eq!(tree.get(100)?, Some(Object::Integer(50)));
/// assert_eq!(tree.entries()?.len(), 200);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeWriter {
    max_kids: usize,
}

impl Default for TreeWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeWriter {
    /// The default node size: 64 entries per leaf and 64 kids per intermediate node, so two
    /// levels hold 4,096 entries and three hold 262,144, while a reader's `/Limits` lookup
    /// reads at most a few small arrays.
    pub const DEFAULT_MAX_KIDS: usize = 64;

    /// A writer with [`TreeWriter::DEFAULT_MAX_KIDS`].
    #[must_use]
    pub const fn new() -> Self {
        Self {
            max_kids: Self::DEFAULT_MAX_KIDS,
        }
    }

    /// The same writer with at most `max_kids` entries per leaf and kids per intermediate
    /// node (at least 2, so every level shrinks the next).
    #[must_use]
    pub const fn with_max_kids(self, max_kids: usize) -> Self {
        Self {
            max_kids: if max_kids < 2 { 2 } else { max_kids },
        }
    }

    /// The largest number of entries in a leaf, and of kids in an intermediate node.
    #[must_use]
    pub const fn max_kids(self) -> usize {
        self.max_kids
    }

    /// Writes a name tree (§7.9.6) holding `entries`, keys sorted by their bytes and written
    /// as literal strings, and returns its root, a new indirect object.
    ///
    /// # Errors
    /// Errors of the overlay ([`CosMut::create`]).
    pub fn write_names(self, cos: &mut CosMut<'_>, entries: &BTreeMap<Vec<u8>, Object>) -> Result<ObjRef> {
        let entries: Vec<(Object, Object)> = entries
            .iter()
            .map(|(k, v)| (Object::String(PdfString::literal(k.clone())), v.clone()))
            .collect();
        self.write(cos, "Names", entries)
    }

    /// Writes a number tree (§7.9.7) holding `entries`, keys ascending, and returns its root,
    /// a new indirect object.
    ///
    /// # Errors
    /// As [`TreeWriter::write_names`].
    pub fn write_numbers(self, cos: &mut CosMut<'_>, entries: &BTreeMap<i64, Object>) -> Result<ObjRef> {
        let entries: Vec<(Object, Object)> = entries.iter().map(|(k, v)| (Object::Integer(*k), v.clone())).collect();
        self.write(cos, "Nums", entries)
    }

    /// The tree over `entries` (sorted, unique keys). A tree that fits one node is the root
    /// alone, without `/Limits` (Tables 36 and 37: the root has none). Otherwise the entries
    /// are cut into the fewest leaves of at most `max_kids`, sizes differing by at most one,
    /// and each level is grouped the same way until the root's `/Kids` fits.
    fn write(self, cos: &mut CosMut<'_>, key: &str, entries: Vec<(Object, Object)>) -> Result<ObjRef> {
        if entries.len() <= self.max_kids {
            let flat = entries.into_iter().flat_map(|(k, v)| [k, v]).collect::<Vec<_>>();
            return cos.create(Object::Dict([(key, Object::Array(flat))].into_iter().collect()));
        }
        // Each written node with its lowest and highest key.
        let mut level: Vec<(ObjRef, Object, Object)> = Vec::new();
        for leaf in even_chunks(entries, self.max_kids) {
            let (Some((lo, _)), Some((hi, _))) = (leaf.first(), leaf.last()) else {
                continue;
            };
            let (lo, hi) = (lo.clone(), hi.clone());
            let flat = leaf.into_iter().flat_map(|(k, v)| [k, v]).collect::<Vec<_>>();
            let mut dict = Dict::new();
            dict.insert("Limits", Object::Array(vec![lo.clone(), hi.clone()]));
            dict.insert(key, Object::Array(flat));
            level.push((cos.create(Object::Dict(dict))?, lo, hi));
        }
        while level.len() > self.max_kids {
            let mut next = Vec::new();
            for group in even_chunks(level, self.max_kids) {
                let (Some((_, lo, _)), Some((_, _, hi))) = (group.first(), group.last()) else {
                    continue;
                };
                let (lo, hi) = (lo.clone(), hi.clone());
                let mut dict = Dict::new();
                dict.insert("Limits", Object::Array(vec![lo.clone(), hi.clone()]));
                dict.insert("Kids", kids(&group));
                next.push((cos.create(Object::Dict(dict))?, lo, hi));
            }
            level = next;
        }
        cos.create(Object::Dict([("Kids", kids(&level))].into_iter().collect()))
    }
}

fn kids(nodes: &[(ObjRef, Object, Object)]) -> Object {
    Object::Array(nodes.iter().map(|(r, _, _)| Object::Ref(*r)).collect())
}

/// `items` cut into the fewest chunks of at most `max` items, their sizes differing by at
/// most one (so no leaf is left nearly empty).
pub(crate) fn even_chunks<T>(items: Vec<T>, max: usize) -> Vec<Vec<T>> {
    let total = items.len();
    let chunks = total.div_ceil(max.max(1)).max(1);
    let (base, extra) = (total / chunks, total % chunks);
    let mut out = Vec::with_capacity(chunks);
    let mut items = items.into_iter();
    for i in 0..chunks {
        let size = base + usize::from(i < extra);
        out.push(items.by_ref().take(size).collect());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::even_chunks;

    #[test]
    fn even_chunks_should_balance_sizes() {
        let sizes: Vec<usize> = even_chunks((0..130).collect(), 64).iter().map(Vec::len).collect();
        assert_eq!(sizes, [44, 43, 43]);
    }

    #[test]
    fn even_chunks_should_keep_a_small_set_whole() {
        assert_eq!(even_chunks(vec![1, 2, 3], 64), [vec![1, 2, 3]]);
    }
}
