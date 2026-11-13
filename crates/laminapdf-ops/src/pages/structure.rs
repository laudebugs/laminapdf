//! What a page deletion does to the structure tree (ISO 32000-2 §14.7; E2 plan §11): elements
//! whose content was all on deleted pages are taken out of their parents' `/K`, kids of
//! surviving elements that name a deleted page are taken out of theirs, and the deleted pages'
//! `/ParentTree` entries (their `/StructParents` and their annotations' `/StructParent`) are
//! removed, the tree written again whole.

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_cos::{ObjRef, Object};
use laminapdf_model::{PageId, PageReference, ReferenceKind, TreeWriter};

use super::cleanup::{ReferenceChange, ReferenceOutcome, edit_dict, edit_object};
use crate::error::Result;

/// The `/ParentTree` keys of `pages` (each page's `/StructParents` and its annotations'
/// `/StructParent`), read before the pages leave the tree.
pub(crate) fn parent_tree_keys(doc: &laminapdf_model::Document, pages: &[PageId]) -> Result<BTreeSet<i64>> {
    let mut keys = BTreeSet::new();
    for page in pages {
        let Some(r) = page.as_lamina_id().obj_ref() else {
            continue;
        };
        let object = doc.object(r.num)?;
        let Some(dict) = object.as_dict() else { continue };
        if let Some(n) = dict.get_int("StructParents") {
            keys.insert(n);
        }
        let annots = match dict.get("Annots") {
            Some(Object::Ref(list)) => (*doc.object(list.num)?).clone(),
            Some(other) => other.clone(),
            None => continue,
        };
        for annot in annots.as_array().unwrap_or_default() {
            let Some(annot) = annot.as_ref() else { continue };
            if let Some(n) = doc.object(annot.num)?.as_dict().and_then(|d| d.get_int("StructParent")) {
                keys.insert(n);
            }
        }
    }
    Ok(keys)
}

/// Removes what the structure tree held only for the deleted pages (`deleted`, by object).
pub(crate) fn clean(
    doc: &mut laminapdf_model::Document,
    references: &[PageReference],
    deleted: &[PageId],
    parent_keys: &BTreeSet<i64>,
) -> Result<Vec<ReferenceChange>> {
    let gone: BTreeSet<ObjRef> = deleted.iter().filter_map(|p| p.as_lamina_id().obj_ref()).collect();
    // Candidates: the elements whose `/Pg` is a deleted page, and, as they go, their parents
    // (a grouping element such as a Table or L has no `/Pg` of its own).
    let mut candidates: Vec<ObjRef> = Vec::new();
    for r in references {
        if let ReferenceKind::StructElem { elem: Some(e) } = &r.kind
            && !candidates.contains(e)
        {
            candidates.push(*e);
        }
    }
    let mut removed: BTreeSet<ObjRef> = BTreeSet::new();
    loop {
        let before = removed.len();
        let mut i = 0;
        while let Some(&elem) = candidates.get(i) {
            i += 1;
            if removed.contains(&elem) || !all_gone(doc, elem, &gone, &removed)? {
                continue;
            }
            removed.insert(elem);
            if let Some(parent) = struct_parent(doc, elem)?
                && !candidates.contains(&parent)
            {
                candidates.push(parent);
            }
        }
        if removed.len() == before {
            break;
        }
    }
    // Per holder (a surviving element or the tree root): the removed elements and the kid
    // indexes to drop, all in one pass, so no index shifts under another drop.
    let mut drops: BTreeMap<ObjRef, (BTreeSet<ObjRef>, BTreeSet<usize>)> = BTreeMap::new();
    for &elem in &removed {
        let parent = doc
            .object(elem.num)?
            .as_dict()
            .and_then(|d| d.get("P"))
            .and_then(Object::as_ref);
        if let Some(parent) = parent.filter(|p| !removed.contains(p)) {
            drops.entry(parent).or_default().0.insert(elem);
        }
    }
    let mut changes = Vec::new();
    for r in references {
        match &r.kind {
            ReferenceKind::StructElem { elem: Some(e) } if removed.contains(e) => {
                changes.push(ReferenceChange::new(r.clone(), ReferenceOutcome::Removed));
            }
            ReferenceKind::StructContentRef {
                elem: Some(e), index, ..
            } if !removed.contains(e) => {
                drops.entry(*e).or_default().1.insert(*index);
                changes.push(ReferenceChange::new(r.clone(), ReferenceOutcome::Removed));
            }
            _ => {}
        }
    }
    for (holder, (elements, indices)) in drops {
        drop_kids(doc, holder, |kid, i| {
            indices.contains(&i) || kid.as_ref().is_some_and(|r| elements.contains(&r))
        })?;
    }
    drop_parent_tree_keys(doc, parent_keys)?;
    Ok(changes)
}

/// `elem`'s parent when it is a structure element (not the tree root, `/Type /StructTreeRoot`).
fn struct_parent(doc: &laminapdf_model::Document, elem: ObjRef) -> Result<Option<ObjRef>> {
    let Some(parent) = doc
        .object(elem.num)?
        .as_dict()
        .and_then(|d| d.get("P"))
        .and_then(Object::as_ref)
    else {
        return Ok(None);
    };
    let is_root = doc.object(parent.num)?.as_dict().and_then(|d| d.get_name("Type")) == Some(b"StructTreeRoot");
    Ok((!is_root).then_some(parent))
}

/// Whether every kid of element `elem` is content on a deleted page or a removed element.
fn all_gone(
    doc: &laminapdf_model::Document,
    elem: ObjRef,
    gone: &BTreeSet<ObjRef>,
    removed: &BTreeSet<ObjRef>,
) -> Result<bool> {
    let object = doc.object(elem.num)?;
    let Some(dict) = object.as_dict() else { return Ok(false) };
    let own_page = dict.get("Pg").and_then(Object::as_ref);
    let on_gone = |pg: Option<ObjRef>| pg.or(own_page).is_some_and(|p| gone.contains(&p));
    let kids = match dict.get("K") {
        None => return Ok(on_gone(None)),
        Some(Object::Array(items)) => items.clone(),
        Some(single) => vec![single.clone()],
    };
    for kid in kids {
        let ok = match &kid {
            Object::Integer(_) => on_gone(None),
            Object::Dict(d) => on_gone(d.get("Pg").and_then(Object::as_ref)),
            Object::Ref(r) => {
                let kid = doc.object(r.num)?;
                match kid.as_dict() {
                    Some(d) if matches!(d.get_name("Type"), Some(b"MCR" | b"OBJR")) => {
                        on_gone(d.get("Pg").and_then(Object::as_ref))
                    }
                    Some(_) => removed.contains(r),
                    None => true,
                }
            }
            _ => true,
        };
        if !ok {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Takes the kids `drop(kid, index)` picks out of `/K` of the dictionary object `holder`.
fn drop_kids(doc: &mut laminapdf_model::Document, holder: ObjRef, drop: impl Fn(&Object, usize) -> bool) -> Result<()> {
    edit_dict(doc, holder, |d| match d.get_mut("K") {
        Some(Object::Array(items)) => {
            let mut i = 0;
            items.retain(|kid| {
                let keep = !drop(kid, i);
                i += 1;
                keep
            });
        }
        Some(single) if drop(single, 0) => {
            d.remove("K");
        }
        _ => {}
    })
}

/// Removes `keys` from the structure tree's `/ParentTree`, writing the number tree again.
fn drop_parent_tree_keys(doc: &mut laminapdf_model::Document, keys: &BTreeSet<i64>) -> Result<()> {
    if keys.is_empty() {
        return Ok(());
    }
    let Some(Object::Ref(root)) = doc.catalog()?.get("StructTreeRoot").cloned() else {
        return Ok(());
    };
    let Some(tree) = doc
        .object(root.num)?
        .as_dict()
        .and_then(|d| d.get("ParentTree").cloned())
    else {
        return Ok(());
    };
    let mut entries: BTreeMap<i64, Object> = doc.number_tree(tree).entries()?.into_iter().collect();
    let count = entries.len();
    entries.retain(|k, _| !keys.contains(k));
    if entries.len() == count {
        return Ok(());
    }
    let new = TreeWriter::new().write_numbers(&mut doc.cos_mut().edit(), &entries)?;
    edit_object(doc, root, |o| {
        if let Object::Dict(d) = o {
            d.insert("ParentTree", Object::Ref(new));
        }
    })
}
