//! A new annotation in the structure tree (ISO 32000-2 §14.7.5.3; ISO 14289-1 §7.18.1): an
//! `Annot` element holding an object reference (`OBJR`) to the annotation, which names the
//! element back through its `/StructParent` key in the parent tree.
//!
//! Unlike a link, an annotation takes no content: its element goes beside the content it marks,
//! right after the element that owns that content, so a screen reader meets the comment after
//! the text it is about (E4 plan §3). With no marked content (a note on a picture), it goes last
//! in the document element. A Popup is never tagged: it belongs to its parent.

use laminapdf_cos::{Dict, ObjRef, Object};

use super::object_dict;
use crate::error::Result;
use crate::link::tags::{document_element, kids_of, owners};
use crate::pages::edit_dict;
use crate::parent_tree::ParentTree;

/// Puts the new annotation `annot` on `page` into the structure tree, after the element that owns
/// the first of `marked`, else last in the document element, and gives the page `/Tabs /S` when
/// it has no tab order (ISO 14289-1 §7.18.3). An untagged document is left alone: on one that
/// claims PDF/UA-1 the conformance check reports `annot-not-tagged`.
pub(super) fn tag(doc: &mut laminapdf_model::Document, page: ObjRef, annot: ObjRef, marked: &[u32]) -> Result<()> {
    let Some(mut tree) = ParentTree::read(doc)? else {
        return Ok(());
    };
    let page_dict = object_dict(doc, page)?.unwrap_or_default();
    let owners = owners(doc, &tree, page_dict.get_int("StructParents"))?;
    let owner = marked
        .iter()
        .find_map(|&m| owners.get(m as usize).and_then(Object::as_ref));
    let (parent, after) = match owner {
        Some(owner) => {
            let parent = object_dict(doc, owner)?
                .and_then(|d| d.get("P").and_then(Object::as_ref))
                .unwrap_or_else(|| document_element(&tree));
            (parent, Some(owner))
        }
        None => (document_element(&tree), None),
    };
    let elem: Dict = [
        ("Type", Object::name("StructElem")),
        ("S", Object::name("Annot")),
        ("P", Object::Ref(parent)),
        ("Pg", Object::Ref(page)),
        (
            "K",
            Object::Dict(
                [
                    ("Type", Object::name("OBJR")),
                    ("Obj", Object::Ref(annot)),
                    ("Pg", Object::Ref(page)),
                ]
                .into_iter()
                .collect(),
            ),
        ),
    ]
    .into_iter()
    .collect();
    let elem = doc.cos_mut().edit().create(Object::Dict(elem))?;
    insert_kid(doc, parent, elem, after)?;
    let key = tree.add(Object::Ref(elem));
    edit_dict(doc, annot, |d| {
        d.insert("StructParent", Object::Integer(key));
    })?;
    if page_dict.get("Tabs").is_none() {
        edit_dict(doc, page, |d| {
            d.insert("Tabs", Object::name("S"));
        })?;
    }
    tree.write(doc)
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
