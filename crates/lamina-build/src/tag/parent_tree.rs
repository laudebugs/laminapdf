//! The `ParentTree` (§14.7.5.4): a number tree from each page's `/StructParents` key to the
//! array of the elements owning its MCIDs (indexed by MCID), and from each annotation's
//! `/StructParent` key to its element. Keys are handed out in the order pages and links are
//! written, so the tree is sorted as it is built and the output is deterministic.

use lamina_cos::{Dict, Object};

/// The tree's entries, flat (`key value …`, sorted by key), and the next free key.
#[derive(Debug, Clone, Default)]
pub(crate) struct ParentTree {
    nums: Vec<Object>,
    next_key: i64,
}

impl ParentTree {
    /// Adds `value` (an element, or a page's array of them) under the next key and returns
    /// that key.
    pub(crate) fn push(&mut self, value: Object) -> i64 {
        let key = self.next_key;
        self.nums.push(Object::Integer(key));
        self.nums.push(value);
        self.next_key += 1;
        key
    }

    /// Where the tree is now, to go back to with [`ParentTree::truncate`].
    pub(crate) fn mark(&self) -> (usize, i64) {
        (self.nums.len(), self.next_key)
    }

    /// Forgets what was added after `mark`.
    pub(crate) fn truncate(&mut self, (len, next_key): (usize, i64)) {
        self.nums.truncate(len);
        self.next_key = next_key;
    }

    /// `/ParentTreeNextKey`.
    pub(crate) const fn next_key(&self) -> i64 {
        self.next_key
    }

    /// The tree as one node: `<< /Nums [key value …] >>`. A single leaf is a valid number tree
    /// (§7.9.7) of any size.
    pub(crate) fn to_dict(&self) -> Dict {
        let mut d = Dict::new();
        d.insert("Nums", self.nums.clone());
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_consecutive_from_zero() {
        let mut tree = ParentTree::default();
        let keys = [tree.push(Object::Null), tree.push(Object::Null)];
        assert_eq!((keys, tree.next_key()), ([0, 1], 2));
    }

    #[test]
    fn entries_are_key_value_pairs_in_key_order() {
        let mut tree = ParentTree::default();
        tree.push(Object::Array(vec![]));
        tree.push(Object::Null);
        let expected = Object::Array(vec![
            Object::Integer(0),
            Object::Array(vec![]),
            Object::Integer(1),
            Object::Null,
        ]);
        assert_eq!(tree.to_dict().get("Nums").cloned(), Some(expected));
    }

    #[test]
    fn truncating_forgets_later_entries_and_keys() {
        let mut tree = ParentTree::default();
        tree.push(Object::Null);
        let mark = tree.mark();
        tree.push(Object::Null);
        tree.truncate(mark);
        assert_eq!((tree.mark(), tree.next_key()), ((2, 1), 1));
    }
}
