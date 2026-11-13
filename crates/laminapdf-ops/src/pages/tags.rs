//! What duplicating a page does to the structure tree (ISO 32000-2 §14.7.5.4; E2 plan §2.2):
//! the copy shares its source's content streams, so its marked content keeps the same MCIDs,
//! and it gets its own `/StructParents` key (from `/ParentTreeNextKey`, which advances) whose
//! `/ParentTree` entry maps each MCID to a clone of the element that owned it. An element whose
//! content lies only on the source page is cloned as a sibling after the original, with `/Pg`
//! set to the copy; one that also holds content on other pages, or other elements, can't be
//! split, so its MCIDs map to null and the copy's tags are reported incomplete
//! (`DuplicateTagsIncomplete`). Copied annotations with `/StructParent` get keys the same way.
//! The source page's keys are never reused.

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_cos::{Dict, ObjRef, Object};
use laminapdf_model::TreeWriter;

use super::cleanup::edit_dict;
use crate::error::Result;

/// One duplicated page: its source, its copy, and the annotations copied with it (source to
/// copy).
pub(crate) struct Pair<'a> {
    pub(crate) source: ObjRef,
    pub(crate) copy: ObjRef,
    pub(crate) annots: &'a BTreeMap<ObjRef, ObjRef>,
}

/// Gives each copy in `pairs` its structure; returns the copies whose tags couldn't all be
/// cloned. An untagged document is left alone.
pub(crate) fn clone_tags(doc: &mut laminapdf_model::Document, pairs: &[Pair<'_>]) -> Result<BTreeSet<ObjRef>> {
    let mut incomplete = BTreeSet::new();
    let Some(Object::Ref(root)) = doc.catalog()?.get("StructTreeRoot").cloned() else {
        return Ok(incomplete);
    };
    let Some(root_dict) = doc.object(root.num)?.as_dict().cloned() else {
        return Ok(incomplete);
    };
    let mut entries: BTreeMap<i64, Object> = match root_dict.get("ParentTree") {
        Some(tree) => doc.number_tree(tree.clone()).entries()?.into_iter().collect(),
        None => BTreeMap::new(),
    };
    let after_last = entries.keys().next_back().map_or(0, |k| k.saturating_add(1));
    let mut next = root_dict.get_int("ParentTreeNextKey").unwrap_or(0).max(after_last);
    let start = next;
    let mut cloner = Cloner {
        clones: BTreeMap::new(),
    };
    for pair in pairs {
        let mut whole = true;
        let source = doc.object(pair.source.num)?.as_dict().cloned().unwrap_or_default();
        if let Some(key) = source.get_int("StructParents")
            && let Some(Object::Array(owners)) =
                entries.get(&key).map(|v| doc.deref(v)).transpose()?.as_deref().cloned()
        {
            let mut mapped = Vec::with_capacity(owners.len());
            for owner in owners {
                let clone = match owner {
                    Object::Ref(elem) => cloner.clone_elem(doc, elem, pair)?,
                    _ => None,
                };
                // An MCID with no owner stays without one; an owner that can't be cloned
                // leaves the copy's content untagged.
                whole &= clone.is_some() || !matches!(owner, Object::Ref(_));
                mapped.push(clone.map_or(Object::Null, Object::Ref));
            }
            entries.insert(next, Object::Array(mapped));
            let key = next;
            edit_dict(doc, pair.copy, |d| {
                d.insert("StructParents", Object::Integer(key));
            })?;
            next += 1;
        }
        for (&old, &new) in pair.annots {
            let Some(key) = doc.object(old.num)?.as_dict().and_then(|d| d.get_int("StructParent")) else {
                continue;
            };
            let owner = entries.get(&key).cloned();
            let clone = match owner {
                Some(Object::Ref(elem)) => cloner.clone_elem(doc, elem, pair)?,
                _ => None,
            };
            match clone {
                Some(c) => {
                    entries.insert(next, Object::Ref(c));
                    let key = next;
                    edit_dict(doc, new, |d| {
                        d.insert("StructParent", Object::Integer(key));
                    })?;
                    next += 1;
                }
                None => whole = false,
            }
        }
        if !whole {
            incomplete.insert(pair.copy);
        }
    }
    if next != start {
        let tree = TreeWriter::new().write_numbers(&mut doc.cos_mut().edit(), &entries)?;
        edit_dict(doc, root, |d| {
            d.insert("ParentTree", Object::Ref(tree));
            d.insert("ParentTreeNextKey", Object::Integer(next));
        })?;
    }
    Ok(incomplete)
}

/// Clones elements once per copy.
struct Cloner {
    /// (element, copy page) → its clone; `None` when it can't be cloned.
    clones: BTreeMap<(ObjRef, ObjRef), Option<ObjRef>>,
}

impl Cloner {
    /// The clone of `elem` for `pair`'s copy, made the first time it is asked for; `None`
    /// when `elem` holds anything but marked content and annotations of the source page.
    fn clone_elem(
        &mut self,
        doc: &mut laminapdf_model::Document,
        elem: ObjRef,
        pair: &Pair<'_>,
    ) -> Result<Option<ObjRef>> {
        if let Some(done) = self.clones.get(&(elem, pair.copy)) {
            return Ok(*done);
        }
        let clone = make_clone(doc, elem, pair)?;
        self.clones.insert((elem, pair.copy), clone);
        Ok(clone)
    }
}

/// Builds the clone of `elem` and puts it after `elem` in its parent's `/K`.
fn make_clone(doc: &mut laminapdf_model::Document, elem: ObjRef, pair: &Pair<'_>) -> Result<Option<ObjRef>> {
    let Some(dict) = doc.object(elem.num)?.as_dict().cloned() else {
        return Ok(None);
    };
    let Some(parent) = dict.get("P").and_then(Object::as_ref) else {
        return Ok(None);
    };
    let own_page = dict.get("Pg").and_then(Object::as_ref);
    let kids = match dict.get("K") {
        None => Vec::new(),
        Some(Object::Array(items)) => items.clone(),
        Some(single) => vec![single.clone()],
    };
    let mut mapped = Vec::with_capacity(kids.len());
    for kid in kids {
        let Some(m) = map_kid(doc, &kid, own_page, pair)? else {
            return Ok(None);
        };
        mapped.push(m);
    }
    let mut clone = dict;
    clone.insert("Pg", Object::Ref(pair.copy));
    clone.remove("ID");
    match mapped.len() {
        0 => {
            clone.remove("K");
        }
        1 => {
            if let Some(only) = mapped.pop() {
                clone.insert("K", only);
            }
        }
        _ => {
            clone.insert("K", Object::Array(mapped));
        }
    }
    let new = doc.cos_mut().edit().create(Object::Dict(clone))?;
    edit_dict(doc, parent, |d| {
        let mut items = match d.get("K") {
            Some(Object::Array(items)) => items.clone(),
            Some(single) => vec![single.clone()],
            None => Vec::new(),
        };
        let at = items
            .iter()
            .position(|k| k.as_ref() == Some(elem))
            .map_or(items.len(), |i| i + 1);
        items.insert(at, Object::Ref(new));
        d.insert("K", Object::Array(items));
    })?;
    Ok(Some(new))
}

/// `kid` of an element (own page `own_page`) as its clone holds it: an MCID as it is, a
/// marked-content or object reference on the source page pointed at the copy (an object
/// reference at the annotation's copy); `None` for anything else, which makes the element
/// impossible to clone.
fn map_kid(
    doc: &mut laminapdf_model::Document,
    kid: &Object,
    own_page: Option<ObjRef>,
    pair: &Pair<'_>,
) -> Result<Option<Object>> {
    let on_source = |pg: Option<ObjRef>| pg.or(own_page).is_none_or(|p| p == pair.source);
    match kid {
        Object::Integer(_) => Ok(on_source(None).then(|| kid.clone())),
        Object::Dict(d) => Ok(map_ref_dict(d, on_source, pair).map(Object::Dict)),
        Object::Ref(r) => {
            let Some(d) = doc.object(r.num)?.as_dict().cloned() else {
                return Ok(None);
            };
            let Some(mapped) = map_ref_dict(&d, on_source, pair) else {
                return Ok(None);
            };
            Ok(Some(Object::Ref(doc.cos_mut().edit().create(Object::Dict(mapped))?)))
        }
        _ => Ok(None),
    }
}

/// A marked-content reference (`/Type /MCR`) or object reference (`/Type /OBJR`) on the
/// source page, pointed at the copy; `None` for an element or a reference elsewhere.
fn map_ref_dict(d: &Dict, on_source: impl Fn(Option<ObjRef>) -> bool, pair: &Pair<'_>) -> Option<Dict> {
    let pg = d.get("Pg").and_then(Object::as_ref);
    if !on_source(pg) {
        return None;
    }
    let mut out = d.clone();
    match d.get_name("Type") {
        Some(b"MCR") => {}
        Some(b"OBJR") => {
            let obj = d.get("Obj").and_then(Object::as_ref)?;
            out.insert("Obj", Object::Ref(*pair.annots.get(&obj)?));
        }
        _ if d.contains_key("MCID") => {}
        _ => return None,
    }
    if pg.is_some() {
        out.insert("Pg", Object::Ref(pair.copy));
    }
    Some(out)
}
