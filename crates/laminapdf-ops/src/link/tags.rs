//! A link in the structure tree (ISO 32000-2 §14.7.5.3 and §14.8.4.4.2; ISO 14289-1 §7.18.5).
//! A tagged link is a `Link` element that holds the marked content of its text and an object
//! reference (`OBJR`) to the annotation. The page's parent tree entry names the `Link` element
//! as the owner of that content, and the annotation's `/StructParent` key names it as the
//! annotation's owner.
//!
//! [`tag`] puts a new link there, taking the content the link covers out of the element that
//! held it; [`untag`] puts that content back before a link is deleted.

use laminapdf_cos::{Dict, ObjRef, Object};

use crate::annot::object_dict;
use crate::error::{Error, Result};
use crate::pages::edit_dict;
use crate::parent_tree::ParentTree;

/// The element a new link joins, and the covered content of it that moves into the link.
struct Place {
    parent: ObjRef,
    /// The parent's kid the new `Link` element goes after, when it doesn't take the covered
    /// content's place (a link on text another link already covers).
    after: Option<ObjRef>,
    /// The MCIDs whose content moves into the new `Link` element.
    covered: Vec<u32>,
}

/// Puts the new link `annot` on `page` into the structure tree, as a `Link` element under the
/// element that owns the first of `marked` (the MCIDs of the page's content the link covers),
/// else under the document element. The content among `marked` that this element owns moves into
/// the `Link` element, which takes its place; content another element owns stays where it is.
/// Text another link already covers stays in that link, and the new `Link` element goes after it,
/// since a link can't hold a link. The page gets `/Tabs /S`, the tab order a tagged page with
/// annotations needs (ISO 14289-1 §7.18.3).
///
/// An untagged document is left alone, unless it claims PDF/UA-1:
/// [`Error::LinkNotTagged`].
pub(super) fn tag(doc: &mut laminapdf_model::Document, page: ObjRef, annot: ObjRef, marked: &[u32]) -> Result<()> {
    let Some(mut tree) = ParentTree::read(doc)? else {
        return if crate::metadata::read_metadata(doc)?.pdfua.is_some() {
            Err(Error::LinkNotTagged)
        } else {
            Ok(())
        };
    };
    let page_dict = object_dict(doc, page)?.unwrap_or_default();
    let page_key = page_dict.get_int("StructParents");
    let mut owners = owners(doc, &tree, page_key)?;
    let place = place(doc, &tree, &owners, marked)?;
    let link = doc.cos_mut().edit().create(Object::Null)?;
    let mut moved = take_covered(doc, &place, page, link)?;
    for kid in &moved {
        if let Some(m) = mcid_on(doc, kid, Some(page), page)?
            && let Some(slot) = owners.get_mut(m as usize)
        {
            *slot = Object::Ref(link);
        }
    }
    moved.push(Object::Dict(
        [
            ("Type", Object::name("OBJR")),
            ("Obj", Object::Ref(annot)),
            ("Pg", Object::Ref(page)),
        ]
        .into_iter()
        .collect(),
    ));
    let elem: Dict = [
        ("Type", Object::name("StructElem")),
        ("S", Object::name("Link")),
        ("P", Object::Ref(place.parent)),
        ("Pg", Object::Ref(page)),
        ("K", Object::Array(moved)),
    ]
    .into_iter()
    .collect();
    doc.cos_mut().edit().set(link, Object::Dict(elem))?;

    if let Some(k) = page_key
        && !place.covered.is_empty()
    {
        tree.entries.insert(k, Object::Array(owners));
    }
    let key = tree.add(Object::Ref(link));
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

/// Where a new link on the content `marked` goes.
fn place(doc: &laminapdf_model::Document, tree: &ParentTree, owners: &[Object], marked: &[u32]) -> Result<Place> {
    let owner = |mcid: u32| owners.get(mcid as usize).and_then(Object::as_ref);
    let Some(first) = marked.iter().find_map(|&m| owner(m)) else {
        return Ok(Place {
            parent: document_element(tree),
            after: None,
            covered: Vec::new(),
        });
    };
    let first_dict = object_dict(doc, first)?.unwrap_or_default();
    if is_link(&first_dict) {
        // Beside that link, in its parent, or under the document element when it names none.
        let parent = first_dict
            .get("P")
            .and_then(Object::as_ref)
            .unwrap_or_else(|| document_element(tree));
        return Ok(Place {
            parent,
            after: Some(first),
            covered: Vec::new(),
        });
    }
    let covered = marked.iter().copied().filter(|&m| owner(m) == Some(first)).collect();
    Ok(Place {
        parent: first,
        after: None,
        covered,
    })
}

/// Takes the covered content out of the parent's `/K`, the new `Link` element `link` in its place
/// (or after `place.after`, or last); returns that content, in order.
fn take_covered(doc: &mut laminapdf_model::Document, place: &Place, page: ObjRef, link: ObjRef) -> Result<Vec<Object>> {
    let mut parent_dict = object_dict(doc, place.parent)?.unwrap_or_default();
    let parent_page = parent_dict.get("Pg").and_then(Object::as_ref);
    let mut kept = Vec::new();
    let mut moved = Vec::new();
    let mut placed = false;
    for kid in kids_of(doc, &parent_dict)? {
        if !place.covered.is_empty()
            && mcid_on(doc, &kid, parent_page, page)?.is_some_and(|m| place.covered.contains(&m))
        {
            if !placed {
                kept.push(Object::Ref(link));
                placed = true;
            }
            moved.push(kid);
            continue;
        }
        let after = place.after.is_some() && kid.as_ref() == place.after;
        kept.push(kid);
        if after {
            kept.push(Object::Ref(link));
            placed = true;
        }
    }
    if !placed {
        kept.push(Object::Ref(link));
    }
    parent_dict.insert("K", Object::Array(kept));
    doc.cos_mut().edit().set(place.parent, Object::Dict(parent_dict))?;
    Ok(moved)
}

/// Takes the `Link` element of the link `annot` out of the structure tree before the link is
/// deleted: the content it held goes back to its parent, in its place (an element among it names
/// that parent as its own), and the page's parent tree names that parent as its owner again.
/// Anything else (an untagged link, a link whose owner isn't a `Link` element) is left to the
/// annotation delete, which removes the `OBJR`.
pub(super) fn untag(doc: &mut laminapdf_model::Document, annot: ObjRef) -> Result<()> {
    let Some(mut tree) = ParentTree::read(doc)? else {
        return Ok(());
    };
    let Some(key) = object_dict(doc, annot)?.and_then(|d| d.get_int("StructParent")) else {
        return Ok(());
    };
    let Some(elem) = tree.entries.get(&key).and_then(Object::as_ref) else {
        return Ok(());
    };
    let Some(elem_dict) = object_dict(doc, elem)?.filter(is_link) else {
        return Ok(());
    };
    let Some(parent) = elem_dict.get("P").and_then(Object::as_ref) else {
        return Ok(());
    };
    let Some(mut parent_dict) = object_dict(doc, parent)? else {
        return Ok(());
    };
    let page = elem_dict.get("Pg").and_then(Object::as_ref);
    let parent_page = parent_dict.get("Pg").and_then(Object::as_ref);
    let mut content = Vec::new();
    for kid in kids_of(doc, &elem_dict)? {
        if is_objr(doc, &kid)? {
            continue;
        }
        if let Some(child) = kid.as_ref()
            && object_dict(doc, child)?.is_some_and(|d| d.get("S").is_some())
        {
            edit_dict(doc, child, |d| {
                d.insert("P", Object::Ref(parent));
            })?;
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
    for kid in kids_of(doc, &parent_dict)? {
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
        let mut owners = owners(doc, &tree, Some(page_key))?;
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

/// The owner of each MCID on the page whose `/StructParents` is `key`.
fn owners(doc: &laminapdf_model::Document, tree: &ParentTree, key: Option<i64>) -> Result<Vec<Object>> {
    let Some(value) = key.and_then(|k| tree.entries.get(&k)) else {
        return Ok(Vec::new());
    };
    Ok(doc.deref(value)?.as_array().map(<[Object]>::to_vec).unwrap_or_default())
}

/// The element a link with no content of its own joins: the document element (the root's first
/// child element), else the root.
fn document_element(tree: &ParentTree) -> ObjRef {
    match tree.root_dict.get("K") {
        Some(Object::Ref(elem)) => *elem,
        Some(Object::Array(kids)) => kids.iter().find_map(Object::as_ref).unwrap_or(tree.root),
        _ => tree.root,
    }
}

/// Whether `dict` is a `Link` element.
fn is_link(dict: &Dict) -> bool {
    dict.get_name("S") == Some(b"Link".as_slice())
}

/// An element's `/K` as a list, an indirect array read through.
fn kids_of(doc: &laminapdf_model::Document, dict: &Dict) -> Result<Vec<Object>> {
    Ok(match dict.get("K") {
        Some(Object::Array(kids)) => kids.clone(),
        Some(Object::Ref(r)) => match doc.deref(&Object::Ref(*r))?.as_array() {
            Some(kids) => kids.to_vec(),
            None => vec![Object::Ref(*r)],
        },
        Some(kid) => vec![kid.clone()],
        None => Vec::new(),
    })
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
