//! The structure tree's parent tree (ISO 32000-2 §14.7.5.4) as the ops that add structure edit
//! it: read once into a map from key to owner, given new keys from `/ParentTreeNextKey` (past
//! the last key in use, whatever the root says), and written back whole as a fresh number tree.

use std::collections::BTreeMap;

use laminapdf_cos::{Dict, ObjRef, Object};
use laminapdf_model::TreeWriter;

use crate::annot::object_dict;
use crate::error::Result;
use crate::pages::edit_dict;

/// The structure tree's root and its parent tree, read once per op.
pub(crate) struct ParentTree {
    pub(crate) root: ObjRef,
    pub(crate) root_dict: Dict,
    /// Each key's owner: an element, or an array of them for a page's marked content.
    pub(crate) entries: BTreeMap<i64, Object>,
    /// The next free key.
    pub(crate) next: i64,
}

impl ParentTree {
    /// The document's parent tree; `None` when it has no structure tree.
    pub(crate) fn read(doc: &laminapdf_model::Document) -> Result<Option<Self>> {
        let Some(Object::Ref(root)) = doc.catalog()?.get("StructTreeRoot").cloned() else {
            return Ok(None);
        };
        let Some(root_dict) = object_dict(doc, root)? else {
            return Ok(None);
        };
        let entries: BTreeMap<i64, Object> = match root_dict.get("ParentTree") {
            Some(tree) => doc.number_tree(tree.clone()).entries()?.into_iter().collect(),
            None => BTreeMap::new(),
        };
        let after_last = entries.keys().next_back().map_or(0, |k| k.saturating_add(1));
        let next = root_dict.get_int("ParentTreeNextKey").unwrap_or(0).max(after_last);
        Ok(Some(Self {
            root,
            root_dict,
            entries,
            next,
        }))
    }

    /// A new key for `owner`.
    pub(crate) fn add(&mut self, owner: Object) -> i64 {
        let key = self.next;
        self.entries.insert(key, owner);
        self.next += 1;
        key
    }

    /// Writes the tree as a new number tree and the root's `/ParentTree` and `/ParentTreeNextKey`.
    pub(crate) fn write(self, doc: &mut laminapdf_model::Document) -> Result<()> {
        let tree = TreeWriter::new().write_numbers(&mut doc.cos_mut().edit(), &self.entries)?;
        edit_dict(doc, self.root, |d| {
            d.insert("ParentTree", Object::Ref(tree));
            d.insert("ParentTreeNextKey", Object::Integer(self.next));
        })
    }
}
