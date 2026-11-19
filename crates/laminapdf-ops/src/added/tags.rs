//! Added objects in a tagged file (E6 plan §3): an object with alternate text is a `Figure`
//! element that owns its drawing's marked content, and anything else is an artifact, which
//! screen readers skip (ISO 32000-2 §14.8.2.2). The marked content sits around the `Do` in the
//! drawing stream, so its MCID is the page's, and the page's `/StructParents` entry in the parent
//! tree lists the figure.

use laminapdf_cos::{Dict, ObjRef, Object, PdfString};
use laminapdf_syntax::text::encode_text_string;

use super::place::Mark;
use super::spec::Alt;
use crate::annot::{insert_kid, object_dict};
use crate::error::Result;
use crate::link::tags::{document_element, kids_of, owners};
use crate::pages::edit_dict;
use crate::parent_tree::ParentTree;

/// How the drawing of an object with `alt` on `page` is marked: untagged in an untagged file, a
/// new `Figure` element last in the document element for alternate text, else an artifact.
pub(super) fn tag(doc: &mut laminapdf_model::Document, page: ObjRef, alt: Option<&Alt>) -> Result<Mark> {
    let Some(mut tree) = ParentTree::read(doc)? else {
        return Ok(Mark::None);
    };
    let Some(Alt::Text(text)) = alt else {
        return Ok(Mark::Artifact);
    };
    let page_dict = object_dict(doc, page)?.unwrap_or_default();
    let key = page_dict.get_int("StructParents");
    let mut owners = owners(doc, &tree, key)?;
    let mcid = i64::try_from(owners.len()).unwrap_or(i64::MAX);
    let parent = document_element(&tree);
    let elem: Dict = [
        ("Type", Object::name("StructElem")),
        ("S", Object::name("Figure")),
        ("P", Object::Ref(parent)),
        ("Pg", Object::Ref(page)),
        ("Alt", Object::String(PdfString::literal(encode_text_string(text)))),
        ("K", Object::Integer(mcid)),
    ]
    .into_iter()
    .collect();
    let elem = doc.cos_mut().edit().create(Object::Dict(elem))?;
    insert_kid(doc, parent, elem, None)?;
    owners.push(Object::Ref(elem));
    if let Some(key) = key {
        tree.entries.insert(key, Object::Array(owners));
    } else {
        let key = tree.add(Object::Array(owners));
        edit_dict(doc, page, |d| {
            d.insert("StructParents", Object::Integer(key));
        })?;
    }
    tree.write(doc)?;
    Ok(Mark::Figure(mcid))
}

/// Takes the `Figure` that owns `mcid` on `page` out of the structure tree: out of its parent's
/// `/K`, and its place in the page's parent-tree entry left null so the page's other MCIDs keep
/// their owners.
pub(super) fn untag(doc: &mut laminapdf_model::Document, page: ObjRef, mcid: i64) -> Result<()> {
    let Some(mut tree) = ParentTree::read(doc)? else {
        return Ok(());
    };
    let key = object_dict(doc, page)?.unwrap_or_default().get_int("StructParents");
    let mut owners = owners(doc, &tree, key)?;
    let (Some(key), Ok(at)) = (key, usize::try_from(mcid)) else {
        return Ok(());
    };
    let Some(slot) = owners.get_mut(at) else {
        return Ok(());
    };
    let elem = slot.as_ref();
    *slot = Object::Null;
    tree.entries.insert(key, Object::Array(owners));
    if let Some(elem) = elem
        && let Some(parent) = object_dict(doc, elem)?.and_then(|d| d.get("P").and_then(Object::as_ref))
        && let Some(mut parent_dict) = object_dict(doc, parent)?
    {
        let kids: Vec<Object> = kids_of(doc, &parent_dict)?
            .into_iter()
            .filter(|k| k.as_ref() != Some(elem))
            .collect();
        parent_dict.insert("K", Object::Array(kids));
        doc.cos_mut().edit().set(parent, Object::Dict(parent_dict))?;
    }
    tree.write(doc)
}

/// Sets the alternate text of the `Figure` that owns `mcid` on `page`.
pub(super) fn set_alt(doc: &mut laminapdf_model::Document, page: ObjRef, mcid: i64, text: &str) -> Result<bool> {
    let Some(tree) = ParentTree::read(doc)? else {
        return Ok(false);
    };
    let key = object_dict(doc, page)?.unwrap_or_default().get_int("StructParents");
    let owners = owners(doc, &tree, key)?;
    let Some(elem) = usize::try_from(mcid)
        .ok()
        .and_then(|at| owners.get(at))
        .and_then(Object::as_ref)
    else {
        return Ok(false);
    };
    edit_dict(doc, elem, |d| {
        d.insert("Alt", Object::String(PdfString::literal(encode_text_string(text))));
    })?;
    Ok(true)
}
