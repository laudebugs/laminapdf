//! A link in the structure tree (ISO 32000-2 §14.7.5.3 and §14.8.4.4.2; ISO 14289-1 §7.18.5).
//! A tagged link is a `Link` element that holds the marked content of its text and an object
//! reference (`OBJR`) to the annotation. The page's parent tree entry names the `Link` element
//! as the owner of that content, and the annotation's `/StructParent` key names it as the
//! annotation's owner.
//!
//! [`tag`] puts a new link there, taking the content the link covers out of the element that
//! held it; [`untag`] puts that content back before a link is deleted.

use std::collections::BTreeMap;

use laminapdf_cos::{Dict, ObjRef, Object};
use laminapdf_model::TreeWriter;

use crate::annot::object_dict;
use crate::error::{Error, Result};
use crate::pages::edit_dict;

/// The structure tree's root and its parent tree, read once per op.
struct Tree {
    root: ObjRef,
    root_dict: Dict,
    entries: BTreeMap<i64, Object>,
    /// The next free parent tree key.
    next: i64,
}

impl Tree {
    fn read(doc: &laminapdf_model::Document) -> Result<Option<Self>> {
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

    /// The owner of each MCID on the page whose `/StructParents` is `key`.
    fn owners(&self, doc: &laminapdf_model::Document, key: Option<i64>) -> Result<Vec<Object>> {
        let Some(value) = key.and_then(|k| self.entries.get(&k)) else {
            return Ok(Vec::new());
        };
        Ok(doc.deref(value)?.as_array().map(<[Object]>::to_vec).unwrap_or_default())
    }

    /// The element a link with no content of its own joins: the document element (the
    /// root's first child element), else the root.
    fn document_element(&self) -> ObjRef {
        match self.root_dict.get("K") {
            Some(Object::Ref(elem)) => *elem,
            Some(Object::Array(kids)) => kids.iter().find_map(Object::as_ref).unwrap_or(self.root),
            _ => self.root,
        }
    }

    fn write(self, doc: &mut laminapdf_model::Document) -> Result<()> {
        let tree = TreeWriter::new().write_numbers(&mut doc.cos_mut().edit(), &self.entries)?;
        edit_dict(doc, self.root, |d| {
            d.insert("ParentTree", Object::Ref(tree));
            d.insert("ParentTreeNextKey", Object::Integer(self.next));
        })
    }
}

/// Puts the new link `annot` on `page` into the structure tree, as a `Link` element under the
/// element that owns the first of `marked` (the MCIDs of the page's content the link covers),
/// else under the document element. The covered content that element holds moves into the
/// `Link` element, which takes its place. The page gets `/Tabs /S`, the tab order a tagged
/// page with annotations needs (ISO 14289-1 §7.18.3).
///
/// An untagged document is left alone, unless it claims PDF/UA-1:
/// [`Error::LinkNotTagged`].
pub(super) fn tag(doc: &mut laminapdf_model::Document, page: ObjRef, annot: ObjRef, marked: &[u32]) -> Result<()> {
    let Some(mut tree) = Tree::read(doc)? else {
        return if crate::metadata::read_metadata(doc)?.pdfua.is_some() {
            Err(Error::LinkNotTagged)
        } else {
            Ok(())
        };
    };
    let page_dict = object_dict(doc, page)?.unwrap_or_default();
    let page_key = page_dict.get_int("StructParents");
    let mut owners = tree.owners(doc, page_key)?;
    let owner = |mcid: u32| owners.get(mcid as usize).and_then(Object::as_ref);
    let parent = marked
        .iter()
        .find_map(|&m| owner(m))
        .unwrap_or_else(|| tree.document_element());
    let covered: Vec<u32> = marked.iter().copied().filter(|&m| owner(m) == Some(parent)).collect();

    let link = doc.cos_mut().edit().create(Object::Null)?;
    let mut parent_dict = object_dict(doc, parent)?.unwrap_or_default();
    let parent_page = parent_dict.get("Pg").and_then(Object::as_ref);
    let mut kept = Vec::new();
    let mut moved = Vec::new();
    for kid in kids_of(&parent_dict) {
        if mcid_on(doc, &kid, parent_page, page)?.is_some_and(|m| covered.contains(&m)) {
            if moved.is_empty() {
                kept.push(Object::Ref(link));
            }
            moved.push(kid);
        } else {
            kept.push(kid);
        }
    }
    if moved.is_empty() {
        kept.push(Object::Ref(link));
    }
    parent_dict.insert("K", Object::Array(kept));
    doc.cos_mut().edit().set(parent, Object::Dict(parent_dict))?;

    for kid in &moved {
        if let Some(m) = mcid_on(doc, kid, Some(page), page)?
            && let Some(slot) = owners.get_mut(m as usize)
        {
            *slot = Object::Ref(link);
        }
    }
    let objr: Dict = [
        ("Type", Object::name("OBJR")),
        ("Obj", Object::Ref(annot)),
        ("Pg", Object::Ref(page)),
    ]
    .into_iter()
    .collect();
    moved.push(Object::Dict(objr));
    let elem: Dict = [
        ("Type", Object::name("StructElem")),
        ("S", Object::name("Link")),
        ("P", Object::Ref(parent)),
        ("Pg", Object::Ref(page)),
        ("K", Object::Array(moved)),
    ]
    .into_iter()
    .collect();
    doc.cos_mut().edit().set(link, Object::Dict(elem))?;

    if let Some(k) = page_key
        && !covered.is_empty()
    {
        tree.entries.insert(k, Object::Array(owners));
    }
    let key = tree.next;
    tree.next += 1;
    tree.entries.insert(key, Object::Ref(link));
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

/// Takes the `Link` element of the link `annot` out of the structure tree before the link is
/// deleted: the content it held goes back to its parent, in its place, and the page's parent
/// tree names that parent as its owner again. Anything else (an untagged link, a link whose
/// owner isn't a `Link` element) is left to the annotation delete, which removes the `OBJR`.
pub(super) fn untag(doc: &mut laminapdf_model::Document, annot: ObjRef) -> Result<()> {
    let Some(mut tree) = Tree::read(doc)? else {
        return Ok(());
    };
    let Some(key) = object_dict(doc, annot)?.and_then(|d| d.get_int("StructParent")) else {
        return Ok(());
    };
    let Some(elem) = tree.entries.get(&key).and_then(Object::as_ref) else {
        return Ok(());
    };
    let Some(elem_dict) = object_dict(doc, elem)? else {
        return Ok(());
    };
    let Some(parent) = elem_dict.get("P").and_then(Object::as_ref) else {
        return Ok(());
    };
    if elem_dict.get_name("S") != Some(b"Link".as_slice()) {
        return Ok(());
    }
    let Some(mut parent_dict) = object_dict(doc, parent)? else {
        return Ok(());
    };
    let page = elem_dict.get("Pg").and_then(Object::as_ref);
    let parent_page = parent_dict.get("Pg").and_then(Object::as_ref);
    let mut content = Vec::new();
    for kid in kids_of(&elem_dict) {
        if is_objr(doc, &kid)? {
            continue;
        }
        // An MCID kid takes its element's page; one that moves to a parent on another page
        // names its own.
        content.push(match (&kid, page) {
            (Object::Integer(m), Some(pg)) if parent_page != Some(pg) => Object::Dict(
                [
                    ("Type", Object::name("MCR")),
                    ("MCID", Object::Integer(*m)),
                    ("Pg", Object::Ref(pg)),
                ]
                .into_iter()
                .collect(),
            ),
            _ => kid,
        });
    }
    let mut kids = Vec::new();
    for kid in kids_of(&parent_dict) {
        if kid.as_ref() == Some(elem) {
            kids.extend(content.iter().cloned());
        } else {
            kids.push(kid);
        }
    }
    parent_dict.insert("K", Object::Array(kids));
    doc.cos_mut().edit().set(parent, Object::Dict(parent_dict))?;

    if let Some(page) = page
        && let Some(page_key) = object_dict(doc, page)?.and_then(|d| d.get_int("StructParents"))
    {
        let mut owners = tree.owners(doc, Some(page_key))?;
        for slot in &mut owners {
            if slot.as_ref() == Some(elem) {
                *slot = Object::Ref(parent);
            }
        }
        tree.entries.insert(page_key, Object::Array(owners));
    }
    tree.entries.remove(&key);
    doc.cos_mut().edit().delete(elem)?;
    edit_dict(doc, annot, |d| {
        d.remove("StructParent");
    })?;
    tree.write(doc)
}

/// An element's `/K` as a list.
fn kids_of(dict: &Dict) -> Vec<Object> {
    match dict.get("K") {
        Some(Object::Array(kids)) => kids.clone(),
        Some(kid) => vec![kid.clone()],
        None => Vec::new(),
    }
}

/// The MCID of marked-content kid `kid` when its content is on `page`: an integer on an
/// element whose page is `element_page`, or an `MCR` (direct or indirect) on its own `/Pg` or
/// the element's. An `MCR` into a form XObject's stream (`/Stm`) is not the page's content.
fn mcid_on(
    doc: &laminapdf_model::Document,
    kid: &Object,
    element_page: Option<ObjRef>,
    page: ObjRef,
) -> Result<Option<u32>> {
    let mcr = match kid {
        Object::Integer(m) => return Ok((element_page == Some(page)).then(|| u32::try_from(*m).ok()).flatten()),
        Object::Dict(d) => d.clone(),
        Object::Ref(r) => match object_dict(doc, *r)? {
            Some(d) => d,
            None => return Ok(None),
        },
        _ => return Ok(None),
    };
    let on_page = mcr.get("Pg").and_then(Object::as_ref).or(element_page) == Some(page);
    let is_mcr = mcr.get_name("Type") == Some(b"MCR".as_slice()) && mcr.get("Stm").is_none();
    Ok(if on_page && is_mcr {
        mcr.get_int("MCID").and_then(|m| u32::try_from(m).ok())
    } else {
        None
    })
}

/// Whether `kid` is an object reference (`OBJR`).
fn is_objr(doc: &laminapdf_model::Document, kid: &Object) -> Result<bool> {
    let dict = match kid {
        Object::Dict(d) => Some(d.clone()),
        Object::Ref(r) => object_dict(doc, *r)?,
        _ => None,
    };
    Ok(dict.is_some_and(|d| d.get_name("Type") == Some(b"OBJR".as_slice())))
}
