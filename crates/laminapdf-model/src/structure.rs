//! A minimal read-only view of the logical structure (ISO 32000-2 §14.7): the structure tree
//! root's elements with their types, attributes, IDs, classes and kids (child elements,
//! marked-content references and object references), the `RoleMap`, the `ClassMap`, the
//! `IDTree`, the namespaces and the `ParentTree`.
//!
//! The walk is depth-first and tolerant: an element reached twice, or nested deeper than
//! [`Limits::max_tree_depth`](laminapdf_cos::Limits::max_tree_depth), is skipped with
//! [`Warning::InvalidEntry`] (key `K`); a kid that is neither an element, an MCID, an `MCR`
//! nor an `OBJR` is skipped the same way.

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_cos::{Dict, ObjRef, Object, Result, Warning};

use crate::document::{Document, object_number};
use crate::id::PageId;

/// A kid of a structure element, in the element's (reading) order.
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum StructKid {
    /// A child element.
    Elem(StructElem),
    /// Marked content: MCID `mcid` of `page` (the element's `/Pg` unless an `MCR` names one).
    Mcid {
        /// The page whose content stream holds it.
        page: Option<PageId>,
        /// The marked-content ID.
        mcid: i64,
    },
    /// An object reference (`OBJR`): an annotation or an XObject.
    Object {
        /// The page it is on.
        page: Option<PageId>,
        /// The object.
        object: ObjRef,
    },
}

/// A structure element (`/Type /StructElem`).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct StructElem {
    /// The element's object, when it is indirect.
    pub object: Option<ObjRef>,
    /// `/S`, as stored (a standard type or a custom name the `RoleMap` maps).
    pub kind: Vec<u8>,
    /// `/Pg`: the page its content is on by default.
    pub page: Option<PageId>,
    /// `/Alt`.
    pub alt: Option<String>,
    /// `/ActualText`.
    pub actual_text: Option<String>,
    /// `/E`.
    pub expansion: Option<String>,
    /// `/Lang`.
    pub lang: Option<String>,
    /// `/K`: the kids, in order.
    pub kids: Vec<StructKid>,
    /// `/ID`: the element's identifier, which the `IDTree` maps to it (§14.7.2).
    ///
    /// Added in 0.3.0.
    pub id: Option<Vec<u8>>,
    /// `/C`: the attribute classes the element takes from the `ClassMap`, in order.
    ///
    /// Added in 0.3.0.
    pub classes: Vec<Vec<u8>>,
}

impl StructElem {
    /// The child elements, in order.
    ///
    /// Added in 0.2.0.
    pub fn children(&self) -> impl Iterator<Item = &StructElem> {
        self.kids.iter().filter_map(|k| match k {
            StructKid::Elem(e) => Some(e),
            _ => None,
        })
    }

    /// This element and every element below it, depth-first (reading order).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn descendants(&self) -> Vec<&StructElem> {
        let mut out = Vec::new();
        let mut stack = vec![self];
        while let Some(e) = stack.pop() {
            out.push(e);
            // Pushed last child first, so the first is visited next.
            stack.extend(e.kids.iter().rev().filter_map(|k| match k {
                StructKid::Elem(child) => Some(child),
                _ => None,
            }));
        }
        out
    }
}

/// The structure tree (`/StructTreeRoot`).
///
/// Added in 0.2.0.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct StructTree {
    /// The root's `/K`: its top-level elements (one `Document` in Tagged PDF).
    pub roots: Vec<StructElem>,
    /// `/RoleMap`: custom type → the type it maps to.
    pub role_map: BTreeMap<Vec<u8>, Vec<u8>>,
    /// `/ParentTree`, as stored: read it with [`Document::number_tree`].
    pub parent_tree: Option<Object>,
    /// `/ParentTreeNextKey`.
    pub parent_tree_next_key: Option<i64>,
    /// `/ClassMap`: class name → its attribute object or array of them, as stored (§14.7.6.2).
    ///
    /// Added in 0.3.0.
    pub class_map: BTreeMap<Vec<u8>, Object>,
    /// `/IDTree`, as stored: a name tree from element IDs to elements (§14.7.2).
    ///
    /// Added in 0.3.0.
    pub id_tree: Option<Object>,
    /// `/Namespaces`: the namespace dictionaries the tree uses (PDF 2.0, §14.7.4), as stored.
    ///
    /// Added in 0.3.0.
    pub namespaces: Vec<Object>,
}

impl StructTree {
    /// The standard type `kind` stands for: followed through the `RoleMap` (at most as many
    /// steps as it has entries, so a cycle ends), else `kind` itself.
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn standard_kind<'a>(&'a self, kind: &'a [u8]) -> &'a [u8] {
        let mut current = kind;
        for _ in 0..self.role_map.len() {
            match self.role_map.get(current) {
                Some(next) => current = next,
                None => break,
            }
        }
        current
    }

    /// Every element, depth-first (reading order).
    ///
    /// Added in 0.2.0.
    #[must_use]
    pub fn elements(&self) -> Vec<&StructElem> {
        self.roots.iter().flat_map(StructElem::descendants).collect()
    }
}

struct Walk<'d> {
    doc: &'d Document,
    max_depth: usize,
    visited: BTreeSet<u32>,
}

fn page_of(dict: &Dict) -> Option<PageId> {
    dict.get("Pg").and_then(Object::as_ref).map(PageId::object)
}

impl Walk<'_> {
    fn skip(&self, obj: &Object) -> Result<()> {
        self.doc.warn(Warning::InvalidEntry {
            object: object_number(obj),
            key: "K",
        })
    }

    /// Whether the element at `obj` may be visited: not deeper than the limit, not reached
    /// before (each refusal is warned about).
    fn admit(&mut self, obj: &Object, depth: usize) -> Result<bool> {
        let fresh = obj.as_ref().is_none_or(|r| self.visited.insert(r.num));
        if depth > self.max_depth || !fresh {
            self.skip(obj)?;
            return Ok(false);
        }
        Ok(true)
    }

    /// The element at `obj` (`None` when it is skipped).
    fn elem(&mut self, obj: &Object, depth: usize) -> Result<Option<StructElem>> {
        if !self.admit(obj, depth)? {
            return Ok(None);
        }
        let Some(node) = self.doc.read(obj)? else {
            self.skip(obj)?;
            return Ok(None);
        };
        let Some(dict) = node.as_dict() else {
            self.skip(obj)?;
            return Ok(None);
        };
        self.read_elem(obj.as_ref(), dict, depth).map(Some)
    }

    /// The element whose dictionary is `dict` (already resolved and admitted).
    fn read_elem(&mut self, object: Option<ObjRef>, dict: &Dict, depth: usize) -> Result<StructElem> {
        let doc = self.doc;
        let page = page_of(dict);
        let mut elem = StructElem {
            object,
            kind: dict.get_name("S").map(<[u8]>::to_vec).unwrap_or_default(),
            page: page.clone(),
            alt: doc.text_entry(dict, "Alt")?,
            actual_text: doc.text_entry(dict, "ActualText")?,
            expansion: doc.text_entry(dict, "E")?,
            lang: doc.text_entry(dict, "Lang")?,
            kids: Vec::new(),
            id: match dict.get("ID") {
                Some(id) => match doc.read(id)?.as_deref() {
                    Some(Object::String(s)) => Some(s.bytes.clone()),
                    _ => None,
                },
                None => None,
            },
            classes: match dict.get("C") {
                Some(c) => classes_of(doc.read(c)?.as_deref()),
                None => Vec::new(),
            },
        };
        if let Some(k) = dict.get("K") {
            for item in kids_of(doc, k)? {
                if let Some(kid) = self.kid(&item, page.as_ref(), depth)? {
                    elem.kids.push(kid);
                }
            }
        }
        Ok(elem)
    }

    fn kid(&mut self, item: &Object, page: Option<&PageId>, depth: usize) -> Result<Option<StructKid>> {
        if let Some(mcid) = item.as_int() {
            return Ok(Some(StructKid::Mcid {
                page: page.cloned(),
                mcid,
            }));
        }
        if matches!(item, Object::Null) {
            return Ok(None);
        }
        let resolved = self.doc.read(item)?;
        let Some(dict) = resolved.as_deref().and_then(Object::as_dict) else {
            self.skip(item)?;
            return Ok(None);
        };
        let kid_page = page_of(dict).or_else(|| page.cloned());
        let kid = match dict.get_name("Type") {
            Some(b"MCR") => dict
                .get("MCID")
                .and_then(Object::as_int)
                .map(|mcid| StructKid::Mcid { page: kid_page, mcid }),
            Some(b"OBJR") => dict
                .get("Obj")
                .and_then(Object::as_ref)
                .map(|object| StructKid::Object { page: kid_page, object }),
            _ => {
                if !self.admit(item, depth + 1)? {
                    return Ok(None);
                }
                return Ok(Some(StructKid::Elem(self.read_elem(item.as_ref(), dict, depth + 1)?)));
            }
        };
        if kid.is_none() {
            self.skip(item)?;
        }
        Ok(kid)
    }
}

/// The class names in a `/C` value: one name, or an array of names (each optionally followed
/// by a revision number, which is skipped).
fn classes_of(value: Option<&Object>) -> Vec<Vec<u8>> {
    match value {
        Some(Object::Name(n)) => vec![n.as_bytes().to_vec()],
        Some(Object::Array(items)) => items
            .iter()
            .filter_map(|i| match i {
                Object::Name(n) => Some(n.as_bytes().to_vec()),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// The items of a `/K`: an array (direct or indirect), else the one value (a reference to an
/// element stays a reference, so the element keeps its object).
pub(crate) fn kids_of(doc: &Document, k: &Object) -> Result<Vec<Object>> {
    Ok(match k {
        Object::Array(items) => items.clone(),
        Object::Ref(_) => match doc.read(k)?.as_deref() {
            Some(Object::Array(items)) => items.clone(),
            _ => vec![k.clone()],
        },
        other => vec![other.clone()],
    })
}

/// Reads the catalog's `/StructTreeRoot`.
pub(crate) fn read(doc: &Document) -> Result<Option<StructTree>> {
    let catalog = doc.catalog()?;
    let Some(root_obj) = catalog.get("StructTreeRoot") else {
        return Ok(None);
    };
    let root = doc.read(root_obj)?;
    let Some(root) = root.as_deref().and_then(Object::as_dict) else {
        doc.warn(Warning::InvalidEntry {
            object: doc.trailer().get("Root").and_then(Object::as_ref).map(|r| r.num),
            key: "StructTreeRoot",
        })?;
        return Ok(None);
    };
    let mut walk = Walk {
        doc,
        max_depth: doc.limits().max_tree_depth(),
        visited: root_obj.as_ref().map(|r| r.num).into_iter().collect(),
    };
    let mut tree = StructTree {
        parent_tree: root.get("ParentTree").cloned(),
        parent_tree_next_key: root.get("ParentTreeNextKey").and_then(Object::as_int),
        ..StructTree::default()
    };
    if let Some(map) = root.get("RoleMap")
        && let Some(map) = doc.read(map)?
        && let Some(map) = map.as_dict()
    {
        for (key, value) in map.iter() {
            if let Object::Name(name) = value {
                tree.role_map.insert(key.as_bytes().to_vec(), name.as_bytes().to_vec());
            }
        }
    }
    if let Some(map) = root.get("ClassMap")
        && let Some(map) = doc.read(map)?
        && let Some(map) = map.as_dict()
    {
        for (key, value) in map.iter() {
            tree.class_map.insert(key.as_bytes().to_vec(), value.clone());
        }
    }
    tree.id_tree = root.get("IDTree").cloned();
    if let Some(ns) = root.get("Namespaces")
        && let Some(Object::Array(items)) = doc.read(ns)?.as_deref()
    {
        tree.namespaces.clone_from(items);
    }
    if let Some(k) = root.get("K") {
        for item in kids_of(doc, k)? {
            if let Some(elem) = walk.elem(&item, 1)? {
                tree.roots.push(elem);
            }
        }
    }
    Ok(Some(tree))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn elem(kind: &str, kids: Vec<StructKid>) -> StructElem {
        StructElem {
            kind: kind.as_bytes().to_vec(),
            kids,
            ..StructElem::default()
        }
    }

    #[test]
    fn descendants_are_in_reading_order() {
        let tree = elem(
            "Document",
            vec![
                StructKid::Elem(elem("H1", vec![])),
                StructKid::Elem(elem("P", vec![StructKid::Elem(elem("Span", vec![]))])),
                StructKid::Elem(elem("P", vec![])),
            ],
        );
        let kinds: Vec<&[u8]> = tree.descendants().iter().map(|e| e.kind.as_slice()).collect();
        assert_eq!(kinds, [&b"Document"[..], b"H1", b"P", b"Span", b"P"]);
    }

    #[test]
    fn a_role_map_cycle_ends() {
        let mut tree = StructTree::default();
        tree.role_map.insert(b"A".to_vec(), b"B".to_vec());
        tree.role_map.insert(b"B".to_vec(), b"A".to_vec());
        assert!(matches!(tree.standard_kind(b"A"), b"A" | b"B"));
    }
}
