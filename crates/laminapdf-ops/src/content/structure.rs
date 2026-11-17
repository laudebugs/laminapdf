//! The structure tree kept in step with content edits (ADR 0022 §4; E7 plan §3). An MCID an edit
//! took out of the page leaves its slot in the page's `/ParentTree` array and its element's `/K`,
//! and an element left with no content leaves its parent. Text an edit rewrote updates the
//! `/ActualText` and `/Alt` of the element it belongs to; a figure whose image was replaced loses
//! its alternative text, which described the old image, so the save asks for new text.

use std::collections::BTreeSet;

use laminapdf_cos::{Dict, ObjRef, Object};
use laminapdf_model::PageId;
use laminapdf_syntax::PdfString;
use laminapdf_syntax::text::{decode_text_string, encode_text_string};

use crate::annot::object_dict;
use crate::error::Result;
use crate::pages::edit_dict;
use crate::parent_tree::ParentTree;

/// How deep an element may sit before pruning stops climbing: a guard against a `/P` cycle.
const MAX_DEPTH: usize = 64;

/// How many role-map steps lead to a standard type before the search gives up.
const MAX_ROLE_STEPS: usize = 8;

/// The MCIDs of `before` that `after` doesn't hold: what an edit took out of the page.
pub(super) fn gone(before: &BTreeSet<i64>, after: &BTreeSet<i64>) -> BTreeSet<i64> {
    let mut out = BTreeSet::new();
    for &mcid in before {
        if !after.contains(&mcid) {
            out.insert(mcid);
        }
    }
    out
}

/// Takes the MCIDs `gone`, which the page's content no longer draws, out of the structure tree:
/// each one's `/ParentTree` slot is cleared and its element no longer names it, and elements
/// left empty leave their parents. The page's key goes when no slot names an element any more.
///
/// Returns whether the tags couldn't be kept whole: a gone MCID that the parent tree doesn't map
/// to an element, or a page with tagged content and no `/StructParents`.
pub(super) fn untag(doc: &mut laminapdf_model::Document, page: &PageId, gone: &BTreeSet<i64>) -> Result<bool> {
    if gone.is_empty() {
        return Ok(false);
    }
    let Some(PageSlots {
        page: page_ref,
        key,
        mut slots,
        mut tree,
    }) = page_slots(doc, page)?
    else {
        return Ok(true);
    };
    let mut broken = false;
    let mut owners = Vec::new();
    for &mcid in gone {
        match usize::try_from(mcid).ok().and_then(|i| slots.get_mut(i)) {
            Some(slot @ Object::Ref(_)) => {
                if let Object::Ref(elem) = std::mem::take(slot) {
                    owners.push((elem, mcid));
                }
            }
            _ => broken = true,
        }
    }
    if slots.iter().all(|s| matches!(s, Object::Null)) {
        tree.entries.remove(&key);
        edit_dict(doc, page_ref, |d| {
            d.remove("StructParents");
        })?;
    } else {
        tree.entries.insert(key, Object::Array(slots));
    }
    tree.write(doc)?;
    for (elem, mcid) in owners {
        drop_mcid(doc, elem, page_ref, mcid)?;
    }
    Ok(broken)
}

/// The element the page's parent tree says owns `mcid`.
pub(super) fn owner(doc: &laminapdf_model::Document, page: &PageId, mcid: i64) -> Result<Option<ObjRef>> {
    let Some(PageSlots { slots, .. }) = page_slots(doc, page)? else {
        return Ok(None);
    };
    Ok(usize::try_from(mcid)
        .ok()
        .and_then(|i| slots.get(i))
        .and_then(Object::as_ref))
}

/// Puts `new` where `old` was in the element's `/ActualText` and `/Alt`, so a span whose text
/// changed says the new text. Text the element doesn't hold is left as it is.
pub(super) fn retext(doc: &mut laminapdf_model::Document, elem: ObjRef, old: &str, new: &str) -> Result<()> {
    if old.is_empty() || old == new {
        return Ok(());
    }
    edit_dict(doc, elem, |d| {
        for key in ["ActualText", "Alt"] {
            let Some(Object::String(s)) = d.get(key) else {
                continue;
            };
            let text = decode_text_string(&s.bytes);
            if text.contains(old) {
                let replaced = text.replacen(old, new, 1);
                d.insert(key, Object::String(PdfString::literal(encode_text_string(&replaced))));
            }
        }
    })
}

/// Whether the element is a `Figure` (through the role map), and if so removes its `/Alt` and
/// `/ActualText`: they described the image an edit replaced.
pub(super) fn clear_figure_alt(doc: &mut laminapdf_model::Document, elem: ObjRef) -> Result<bool> {
    let Some(dict) = object_dict(doc, elem)? else {
        return Ok(false);
    };
    if standard_type(doc, &dict)?.as_deref() != Some(b"Figure".as_slice()) {
        return Ok(false);
    }
    edit_dict(doc, elem, |d| {
        d.remove("Alt");
        d.remove("ActualText");
    })?;
    Ok(true)
}

/// A page's entry in the parent tree.
struct PageSlots {
    /// The page's object.
    page: ObjRef,
    /// Its `/StructParents` key.
    key: i64,
    /// The key's slots: the element that owns each MCID, `null` for none.
    slots: Vec<Object>,
    /// The parent tree.
    tree: ParentTree,
}

/// The page's entry in the parent tree; `None` when the page has no key or the tree no array
/// for it.
fn page_slots(doc: &laminapdf_model::Document, page: &PageId) -> Result<Option<PageSlots>> {
    let Some(page_ref) = page.as_lamina_id().obj_ref() else {
        return Ok(None);
    };
    let Some(key) = object_dict(doc, page_ref)?.and_then(|d| d.get_int("StructParents")) else {
        return Ok(None);
    };
    let Some(tree) = ParentTree::read(doc)? else {
        return Ok(None);
    };
    let slots = match tree.entries.get(&key) {
        Some(Object::Array(items)) => items.clone(),
        Some(Object::Ref(r)) => match laminapdf_cos::tolerant(doc.cos().resolve(*r))?.as_deref() {
            Some(Object::Array(items)) => items.clone(),
            _ => return Ok(None),
        },
        _ => return Ok(None),
    };
    Ok(Some(PageSlots {
        page: page_ref,
        key,
        slots,
        tree,
    }))
}

/// Takes `mcid` on `page` out of the element's `/K`, then the element out of its parent's when
/// that left it empty, climbing while parents empty too.
fn drop_mcid(doc: &mut laminapdf_model::Document, elem: ObjRef, page: ObjRef, mcid: i64) -> Result<()> {
    let Some(dict) = object_dict(doc, elem)? else {
        return Ok(());
    };
    let own_page = dict.get("Pg").and_then(Object::as_ref);
    let names_page = |pg: Option<ObjRef>| pg.or(own_page).is_none_or(|p| p == page);
    let mut drops = Vec::new();
    for kid in kids(&dict) {
        let names_it = match &kid {
            Object::Integer(n) => *n == mcid && names_page(None),
            Object::Dict(d) => is_mcr(d, mcid) && names_page(d.get("Pg").and_then(Object::as_ref)),
            Object::Ref(r) => object_dict(doc, *r)?
                .is_some_and(|d| is_mcr(&d, mcid) && names_page(d.get("Pg").and_then(Object::as_ref))),
            _ => false,
        };
        drops.push(names_it);
    }
    let emptied = remove_kids(doc, elem, &drops)?;
    if emptied {
        prune(doc, elem, 0)?;
    }
    Ok(())
}

/// Whether `d` is a marked-content reference to `mcid` in a page's content (not a stream's).
fn is_mcr(d: &Dict, mcid: i64) -> bool {
    d.get_int("MCID") == Some(mcid) && d.get("Stm").is_none()
}

/// Takes element `elem`, which has no content left, out of its parent's `/K`; a parent element
/// that is left empty goes the same way. The tree's root keeps its `/K`.
fn prune(doc: &mut laminapdf_model::Document, elem: ObjRef, depth: usize) -> Result<()> {
    if depth > MAX_DEPTH {
        return Ok(());
    }
    let Some(parent) = object_dict(doc, elem)?.and_then(|d| d.get("P").and_then(Object::as_ref)) else {
        return Ok(());
    };
    let Some(parent_dict) = object_dict(doc, parent)? else {
        return Ok(());
    };
    let drops: Vec<bool> = kids(&parent_dict).iter().map(|k| k.as_ref() == Some(elem)).collect();
    let emptied = remove_kids(doc, parent, &drops)?;
    let is_root = parent_dict.get_name("Type") == Some(b"StructTreeRoot");
    if emptied && !is_root {
        prune(doc, parent, depth + 1)?;
    }
    Ok(())
}

/// The kids `/K` lists, one or many.
fn kids(dict: &Dict) -> Vec<Object> {
    match dict.get("K") {
        None => Vec::new(),
        Some(Object::Array(items)) => items.clone(),
        Some(single) => vec![single.clone()],
    }
}

/// Removes the kids `drops` marks from `/K` of `holder`; whether that left it with none. Nothing
/// is written when no kid is marked.
fn remove_kids(doc: &mut laminapdf_model::Document, holder: ObjRef, drops: &[bool]) -> Result<bool> {
    if !drops.iter().any(|&d| d) {
        return Ok(false);
    }
    let mut emptied = false;
    edit_dict(doc, holder, |d| {
        let kept: Vec<Object> = kids(d)
            .into_iter()
            .zip(drops.iter().copied().chain(std::iter::repeat(false)))
            .filter_map(|(kid, drop)| (!drop).then_some(kid))
            .collect();
        emptied = kept.is_empty();
        match kept.len() {
            0 => {
                d.remove("K");
            }
            _ => {
                d.insert("K", Object::Array(kept));
            }
        }
    })?;
    Ok(emptied)
}

/// The element's standard structure type: its `/S`, through the root's `/RoleMap`.
fn standard_type(doc: &laminapdf_model::Document, dict: &Dict) -> Result<Option<Vec<u8>>> {
    let Some(mut kind) = dict.get_name("S").map(<[u8]>::to_vec) else {
        return Ok(None);
    };
    let role_map = match doc.catalog()?.get("StructTreeRoot").and_then(Object::as_ref) {
        Some(root) => object_dict(doc, root)?
            .and_then(|d| d.get("RoleMap").cloned())
            .map(|m| doc.deref(&m).map(|o| o.as_dict().cloned()))
            .transpose()?
            .flatten(),
        None => None,
    };
    if let Some(map) = role_map {
        for _ in 0..MAX_ROLE_STEPS {
            match map.get_name(&String::from_utf8_lossy(&kind)) {
                Some(next) if next != kind.as_slice() => kind = next.to_vec(),
                _ => break,
            }
        }
    }
    Ok(Some(kind))
}

/// Whether the page draws tagged content that its `/ParentTree` slots don't map to an element.
pub(crate) fn tags_broken(doc: &laminapdf_model::Document, page: &PageId) -> Result<bool> {
    let ir = {
        let pages = doc.pages()?;
        let Some(model_page) = pages.by_id(page) else {
            return Ok(false);
        };
        laminapdf_content::ContentOps::read_page(
            doc,
            model_page.cos(),
            laminapdf_content::ContentOwner::Page(page.clone()),
        )?
    };
    let drawn = ir.drawn_mcids();
    if drawn.is_empty() {
        return Ok(false);
    }
    let Some(PageSlots { slots, .. }) = page_slots(doc, page)? else {
        return Ok(true);
    };
    Ok(drawn.iter().any(|&mcid| {
        usize::try_from(mcid)
            .ok()
            .and_then(|i| slots.get(i))
            .and_then(Object::as_ref)
            .is_none()
    }))
}
