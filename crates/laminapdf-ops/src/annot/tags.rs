//! A new annotation in the structure tree (ISO 32000-2 §14.7.5.3; ISO 14289-1 §7.18.1): an
//! `Annot` element holding an object reference (`OBJR`) to the annotation, which names the
//! element back through its `/StructParent` key in the parent tree.
//!
//! Unlike a link, an annotation takes no content: its element goes beside the content it marks,
//! right after the element that owns that content, so a screen reader meets the comment after
//! the text it is about (E4 plan §3). With no marked content (a note on a picture), it goes after
//! the element that owns the page's last marked content, else last in the document element. A
//! note's Popup gets an `Annot` element of its own right after its note's, as PDF/UA-1 asks of
//! every annotation but widgets, printer's marks and links (ISO 14289-1 §7.18.1).

use laminapdf_cos::{Dict, ObjRef, Object};

use super::object_dict;
use crate::error::Result;
use crate::link::tags::{document_element, kids_of, owners};
use crate::pages::edit_dict;
use crate::parent_tree::ParentTree;

/// Puts the new annotations `annots` on `page` into the structure tree, an `Annot` element each, in
/// order: the first after the element that owns
/// the first of `marked`, else after the element that owns the page's last marked content, else
/// last in the document element; gives the page `/Tabs /S` when it has no tab order (ISO 14289-1
/// §7.18.3). An untagged document is left alone: on one that claims PDF/UA-1 the conformance
/// check reports `annot-not-tagged`.
pub(super) fn tag(doc: &mut laminapdf_model::Document, page: ObjRef, annots: &[ObjRef], marked: &[u32]) -> Result<()> {
    let Some(mut tree) = ParentTree::read(doc)? else {
        return Ok(());
    };
    let page_dict = object_dict(doc, page)?.unwrap_or_default();
    let owners = owners(doc, &tree, page_dict.get_int("StructParents"))?;
    // After the element that owns the marked content, else after the page's last element, so a
    // screen reader meets the mark with its page.
    let owner = marked
        .iter()
        .find_map(|&m| owners.get(m as usize).and_then(Object::as_ref))
        .or_else(|| owners.iter().rev().find_map(Object::as_ref));
    let (parent, after) = match owner {
        Some(owner) => {
            let parent = object_dict(doc, owner)?
                .and_then(|d| d.get("P").and_then(Object::as_ref))
                .unwrap_or_else(|| document_element(&tree));
            (parent, Some(owner))
        }
        None => (document_element(&tree), None),
    };
    let mut after = after;
    for &annot in annots {
        let elem = annot_element(doc, parent, page, annot)?;
        insert_kid(doc, parent, elem, after)?;
        let key = tree.add(Object::Ref(elem));
        edit_dict(doc, annot, |d| {
            d.insert("StructParent", Object::Integer(key));
        })?;
        after = Some(elem);
    }
    if page_dict.get("Tabs").is_none() {
        edit_dict(doc, page, |d| {
            d.insert("Tabs", Object::name("S"));
        })?;
    }
    tree.write(doc)
}

/// A new `Annot` element under `parent` holding an object reference to `annot`.
fn annot_element(doc: &mut laminapdf_model::Document, parent: ObjRef, page: ObjRef, annot: ObjRef) -> Result<ObjRef> {
    let objr: Dict = [
        ("Type", Object::name("OBJR")),
        ("Obj", Object::Ref(annot)),
        ("Pg", Object::Ref(page)),
    ]
    .into_iter()
    .collect();
    let elem: Dict = [
        ("Type", Object::name("StructElem")),
        ("S", Object::name("Annot")),
        ("P", Object::Ref(parent)),
        ("Pg", Object::Ref(page)),
        ("K", Object::Dict(objr)),
    ]
    .into_iter()
    .collect();
    Ok(doc.cos_mut().edit().create(Object::Dict(elem))?)
}

/// Puts `elem` into `parent`'s `/K` after `after`, else last.
fn insert_kid(doc: &mut laminapdf_model::Document, parent: ObjRef, elem: ObjRef, after: Option<ObjRef>) -> Result<()> {
    let mut parent_dict = object_dict(doc, parent)?.unwrap_or_default();
    let mut kids = kids_of(doc, &parent_dict)?;
    let at = after
        .and_then(|a| kids.iter().position(|k| k.as_ref() == Some(a)))
        .map_or(kids.len(), |i| i + 1);
    kids.insert(at, Object::Ref(elem));
    parent_dict.insert("K", Object::Array(kids));
    Ok(doc.cos_mut().edit().set(parent, Object::Dict(parent_dict))?)
}
