//! Finding one page from its object without walking the page tree, for edits that change a
//! single page.

use std::sync::Arc;

use laminapdf_cos::{Dict, ObjRef, Object, Resolved, Result};

use crate::document::{Document, tolerant};
use crate::page::Rotation;

/// A page found by [`Document::page_by_parents`].
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ParentedPage {
    /// The page dictionary.
    pub dict: Dict,
    /// The `/Rotate` the page inherits from its ancestors (its own entry left out), read as
    /// [`Page::rotate_quietly`](crate::Page::rotate_quietly) reads it: [`Rotation::None`]
    /// when no ancestor has one or the nearest one is malformed.
    pub inherited_rotate: Rotation,
}

impl Document {
    /// The page object `r`, found by following `/Parent` up to the root `/Pages` and checking
    /// that each parent's `/Kids` lists the child: a few lookups instead of a page-tree walk.
    /// `None` when that path is broken (not a page, a wrong `/Parent`, something unreadable);
    /// the walk ([`Document::pages`]) is then the authority.
    ///
    /// # Errors
    /// Only those that [must propagate](laminapdf_cos::Error::must_propagate).
    ///
    /// Added in 0.2.0.
    pub fn page_by_parents(&self, r: ObjRef) -> Result<Option<ParentedPage>> {
        let Some(page) = tolerant(self.resolve(r))? else {
            return Ok(None);
        };
        let Some(dict) = page.as_dict() else {
            return Ok(None);
        };
        let is_page = match dict.get_name("Type") {
            Some(t) => t == b"Page",
            None => !dict.contains_key("Kids"),
        };
        let root = tolerant(self.catalog())?.and_then(|c| c.get("Pages").and_then(Object::as_ref));
        let (true, Some(root)) = (is_page, root) else {
            return Ok(None);
        };
        let mut inherited = None;
        let (mut child, mut node) = (r, Arc::clone(&page));
        for _ in 0..self.limits().max_page_tree_depth() {
            let Some(parent_ref) = node.as_dict().and_then(|d| d.get("Parent")).and_then(Object::as_ref) else {
                return Ok(None);
            };
            let Some(parent) = tolerant(self.resolve(parent_ref))? else {
                return Ok(None);
            };
            let Some(parent_dict) = parent.as_dict() else {
                return Ok(None);
            };
            let kids = match parent_dict.get("Kids") {
                Some(kids) => tolerant(self.deref(kids).map(Resolved::into_arc))?,
                None => None,
            };
            let listed = kids
                .as_deref()
                .and_then(Object::as_array)
                .is_some_and(|kids| kids.contains(&Object::Ref(child)));
            if !listed {
                return Ok(None);
            }
            // The nearest ancestor with a non-null `/Rotate` wins, as in the walk.
            if inherited.is_none()
                && let Some(value) = parent_dict.get("Rotate")
            {
                inherited =
                    tolerant(self.deref(value).map(Resolved::into_owned))?.filter(|v| !matches!(v, Object::Null));
            }
            if parent_ref == root {
                return Ok(Some(ParentedPage {
                    dict: dict.clone(),
                    inherited_rotate: inherited.as_ref().and_then(Rotation::from_object).unwrap_or_default(),
                }));
            }
            (child, node) = (parent_ref, parent);
        }
        Ok(None)
    }

    /// A stored `/Rotate` value (direct or a reference) read as a rotation, without a
    /// warning: `None` when it is null, unreadable or not a multiple of 90.
    ///
    /// # Errors
    /// Only those that [must propagate](laminapdf_cos::Error::must_propagate).
    ///
    /// Added in 0.2.0.
    pub fn read_rotate(&self, value: &Object) -> Result<Option<Rotation>> {
        Ok(self.read(value)?.and_then(|v| Rotation::from_object(&v)))
    }
}
