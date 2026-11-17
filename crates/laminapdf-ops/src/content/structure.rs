//! The structure tree kept in step with content edits (ADR 0022 §4; E7 plan §3). An MCID an edit
//! took out of the page leaves its slot in the page's `/ParentTree` array and its element's `/K`,
//! and an element left with no content leaves its parent. Text an edit rewrote updates the
//! `/ActualText` and `/Alt` of the element it belongs to; a figure whose image was replaced loses
//! its alternative text, which described the old image, so the save asks for new text.
//!
//! The page's slots are read once per op, by a lookup in the parent tree, and written back where
//! they are: an indirect slot array, or the tree node that holds it. Only a direct root node, or
//! a page whose last tagged content went, has the tree written again whole.

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_cos::{Dict, ObjRef, Object};
use laminapdf_model::PageId;
use laminapdf_syntax::PdfString;
use laminapdf_syntax::text::{decode_text_string, encode_text_string};

use crate::annot::object_dict;
use crate::error::Result;
use crate::pages::{edit_dict, edit_object};
use crate::parent_tree::ParentTree;
use crate::structure::{drop_kids, kids_of, mcid_on, struct_parent};

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

/// Where a page's slot array lives, so an edit writes it back there.
enum Place {
    /// An indirect array: the parent tree's entry is a reference to it.
    Array(ObjRef),
    /// Directly in the `/Nums` of this indirect tree node.
    Node(ObjRef),
    /// In a direct node: the tree is written again whole.
    Tree,
}

/// A page's entry in the parent tree: one slot per MCID, the element that owns it or `null`.
pub(super) struct PageTags {
    page: ObjRef,
    key: i64,
    slots: Vec<Object>,
    place: Place,
}

impl PageTags {
    /// The page's entry; `None` when the page has no `/StructParents` or the tree no array for it.
    pub(super) fn read(doc: &laminapdf_model::Document, page: &PageId) -> Result<Option<Self>> {
        let Some(page_ref) = page.as_lamina_id().obj_ref() else {
            return Ok(None);
        };
        let Some(key) = object_dict(doc, page_ref)?.and_then(|d| d.get_int("StructParents")) else {
            return Ok(None);
        };
        let Some(root) = doc.catalog()?.get("StructTreeRoot").and_then(Object::as_ref) else {
            return Ok(None);
        };
        let Some(tree) = object_dict(doc, root)?.and_then(|d| d.get("ParentTree").cloned()) else {
            return Ok(None);
        };
        let Some((value, node)) = doc.number_tree(tree).locate(key)? else {
            return Ok(None);
        };
        let (slots, place) = match &value {
            Object::Array(items) => (items.clone(), node.as_ref().map_or(Place::Tree, Place::Node)),
            Object::Ref(r) => match doc.deref(&value)?.as_array() {
                Some(items) => (items.to_vec(), Place::Array(*r)),
                None => return Ok(None),
            },
            _ => return Ok(None),
        };
        Ok(Some(Self {
            page: page_ref,
            key,
            slots,
            place,
        }))
    }

    /// The element that owns `mcid`.
    pub(super) fn owner(&self, mcid: i64) -> Option<ObjRef> {
        usize::try_from(mcid)
            .ok()
            .and_then(|i| self.slots.get(i))
            .and_then(Object::as_ref)
    }

    /// Takes the MCIDs `gone`, which the page's content no longer draws, out of the structure
    /// tree: each one's slot is cleared, its element no longer names it (each element edited
    /// once), and elements left empty leave their parents. The page's key and its
    /// `/StructParents` go when no slot names an element any more.
    pub(super) fn untag(mut self, doc: &mut laminapdf_model::Document, gone: &BTreeSet<i64>) -> Result<()> {
        let mut owners: BTreeMap<ObjRef, BTreeSet<u32>> = BTreeMap::new();
        for &mcid in gone {
            let slot = usize::try_from(mcid).ok().and_then(|i| self.slots.get_mut(i));
            if let Some(slot @ Object::Ref(_)) = slot
                && let Object::Ref(elem) = std::mem::take(slot)
                && let Ok(mcid) = u32::try_from(mcid)
            {
                owners.entry(elem).or_default().insert(mcid);
            }
        }
        if owners.is_empty() {
            return Ok(());
        }
        let page = self.page;
        self.write(doc)?;
        for (elem, mcids) in owners {
            drop_mcids(doc, elem, page, &mcids)?;
        }
        Ok(())
    }

    /// Writes the slots back where they were read from.
    fn write(self, doc: &mut laminapdf_model::Document) -> Result<()> {
        let emptied = self.slots.iter().all(|s| matches!(s, Object::Null));
        let key = self.key;
        let slots = Object::Array(self.slots);
        match self.place {
            Place::Array(array) if !emptied => edit_object(doc, array, |o: &mut Object| *o = slots),
            Place::Node(node) if !emptied => set_entry(doc, node, key, &slots),
            _ => {
                let Some(mut tree) = ParentTree::read(doc)? else {
                    return Ok(());
                };
                if emptied {
                    tree.entries.remove(&key);
                    edit_dict(doc, self.page, |d| {
                        d.remove("StructParents");
                    })?;
                } else {
                    tree.entries.insert(key, slots);
                }
                tree.write(doc)
            }
        }
    }
}

/// Puts `value` under `key` in the `/Nums` of tree node `node` (an indirect `/Nums` array is
/// changed where it is).
fn set_entry(doc: &mut laminapdf_model::Document, node: ObjRef, key: i64, value: &Object) -> Result<()> {
    let replace = |items: &mut Vec<Object>| {
        let (pairs, _) = items.as_chunks_mut::<2>();
        if let Some([_, v]) = pairs.iter_mut().find(|[k, _]| k.as_int() == Some(key)) {
            *v = value.clone();
        }
    };
    match object_dict(doc, node)?.and_then(|d| d.get("Nums").cloned()) {
        Some(Object::Ref(nums)) => edit_object(doc, nums, |o: &mut Object| {
            if let Object::Array(items) = o {
                replace(items);
            }
        }),
        _ => edit_dict(doc, node, |d| {
            if let Some(Object::Array(items)) = d.get_mut("Nums") {
                replace(items);
            }
        }),
    }
}

/// Takes the MCIDs `mcids` on `page` out of the element's `/K`, then the element out of its
/// parent's when that left it empty.
fn drop_mcids(doc: &mut laminapdf_model::Document, elem: ObjRef, page: ObjRef, mcids: &BTreeSet<u32>) -> Result<()> {
    let Some(dict) = object_dict(doc, elem)? else {
        return Ok(());
    };
    // The parent tree says the element owns these MCIDs on this page, so an element without a
    // `/Pg` of its own holds its integer kids there.
    let element_page = dict.get("Pg").and_then(Object::as_ref).or(Some(page));
    let mut dropped = BTreeSet::new();
    for (index, kid) in kids_of(doc, &dict)?.iter().enumerate() {
        if mcid_on(doc, kid, element_page, page)?.is_some_and(|m| mcids.contains(&m)) {
            dropped.insert(index);
        }
    }
    if !dropped.is_empty() && drop_kids(doc, elem, |_, i| dropped.contains(&i))? {
        prune(doc, elem)?;
    }
    Ok(())
}

/// Takes element `elem`, which has no content left, out of its parent's `/K`, the tree root's
/// included; a parent element left empty goes the same way. The climb stops at the root, which
/// keeps its `/K` even when it is empty, and after the tree's depth limit (a `/P` cycle).
fn prune(doc: &mut laminapdf_model::Document, elem: ObjRef) -> Result<()> {
    let mut elem = elem;
    for _ in 0..=doc.limits().max_tree_depth() {
        let Some(parent) = object_dict(doc, elem)?.and_then(|d| d.get("P").and_then(Object::as_ref)) else {
            return Ok(());
        };
        let emptied = drop_kids(doc, parent, |kid, _| kid.as_ref() == Some(elem))?;
        match struct_parent(doc, elem)? {
            Some(next) if emptied => elem = next,
            _ => return Ok(()),
        }
    }
    Ok(())
}

/// Puts `new` where `old` was in the element's `/ActualText` and `/Alt`, so a span whose text
/// changed says the new text. When the old text isn't there exactly once, the entry no longer
/// says what the content shows and can't be fixed, so it goes: a screen reader then reads the
/// content itself.
pub(super) fn retext(doc: &mut laminapdf_model::Document, elem: ObjRef, old: &str, new: &str) -> Result<()> {
    if old == new {
        return Ok(());
    }
    edit_dict(doc, elem, |d| {
        for key in ["ActualText", "Alt"] {
            let Some(Object::String(s)) = d.get(key) else {
                continue;
            };
            let text = decode_text_string(&s.bytes);
            if !old.is_empty() && text.matches(old).count() == 1 {
                let replaced = text.replacen(old, new, 1);
                d.insert(key, Object::String(PdfString::literal(encode_text_string(&replaced))));
            } else {
                d.remove(key);
            }
        }
    })
}

/// Whether the element is a `Figure` (through the role map, as `tree` reads it), and if so
/// removes its `/Alt` and `/ActualText`: they described the image an edit replaced.
pub(super) fn clear_figure_alt(
    doc: &mut laminapdf_model::Document,
    tree: &laminapdf_model::StructTree,
    elem: ObjRef,
) -> Result<bool> {
    let Some(kind) = object_dict(doc, elem)?.and_then(|d| d.get_name("S").map(<[u8]>::to_vec)) else {
        return Ok(false);
    };
    if tree.standard_kind(&kind) != b"Figure" {
        return Ok(false);
    }
    edit_dict(doc, elem, |d| {
        d.remove("Alt");
        d.remove("ActualText");
    })?;
    Ok(true)
}

/// The XObjects among `names` (the page's `/Resources` names an object drew) that are tagged
/// themselves: a form with its own `/StructParents`, or an XObject an `OBJR` names through its
/// `/StructParent`. Deleting their drawing leaves their elements pointing at content the page
/// no longer shows.
pub(super) fn tagged_xobjects(
    doc: &laminapdf_model::Document,
    page: &laminapdf_cos::Page,
    names: &BTreeSet<laminapdf_content::ResourceName>,
) -> Result<Vec<ObjRef>> {
    let Some(xobjects) = xobject_dict(doc, page)? else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for name in names.iter().filter(|n| n.category == "XObject") {
        let Some(Object::Ref(r)) = xobjects.get(&String::from_utf8_lossy(name.name.as_bytes())).cloned() else {
            continue;
        };
        let reference = Object::Ref(r);
        let object = doc.deref(&reference)?;
        let dict = match &*object {
            Object::Stream(s) => Some(&s.dict),
            Object::Dict(d) => Some(d),
            _ => None,
        };
        if dict.is_some_and(|d| d.get("StructParents").is_some() || d.get("StructParent").is_some()) {
            out.push(r);
        }
    }
    Ok(out)
}

/// The page's `/XObject` resources.
fn xobject_dict(doc: &laminapdf_model::Document, page: &laminapdf_cos::Page) -> Result<Option<Dict>> {
    let Some(resources) = page.resources_dict() else {
        return Ok(None);
    };
    Ok(match resources.get("XObject") {
        Some(x) => doc.deref(x)?.as_dict().cloned(),
        None => None,
    })
}

/// Whether the tags of a page an edit changed are broken: it draws tagged content its slots
/// don't map to an element, real content outside every marked-content sequence, or no longer
/// draws a tagged XObject of `dropped`.
pub(crate) fn tags_broken(doc: &laminapdf_model::Document, page: &PageId, dropped: &[ObjRef]) -> Result<bool> {
    let pages = doc.pages()?;
    let Some(model_page) = pages.by_id(page) else {
        return Ok(false);
    };
    let ir = laminapdf_content::ContentOps::read_page(
        doc,
        model_page.cos(),
        laminapdf_content::ContentOwner::Page(page.clone()),
    )?;
    if !dropped.is_empty() {
        let drawn = ir.resource_names();
        let xobjects = xobject_dict(doc, model_page.cos())?.unwrap_or_default();
        let mut still = BTreeSet::new();
        for name in drawn.iter().filter(|n| n.category == "XObject") {
            if let Some(r) = xobjects
                .get(&String::from_utf8_lossy(name.name.as_bytes()))
                .and_then(Object::as_ref)
            {
                still.insert(r);
            }
        }
        if dropped.iter().any(|r| !still.contains(r)) {
            return Ok(true);
        }
    }
    let drawn = ir.drawn_mcids();
    let Some(tags) = PageTags::read(doc, page)? else {
        // A page with tagged content and no entry in the parent tree.
        return Ok(!drawn.is_empty());
    };
    Ok(ir.draws_untagged() || drawn.iter().any(|&mcid| tags.owner(mcid).is_none()))
}
