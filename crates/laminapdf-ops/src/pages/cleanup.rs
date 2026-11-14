//! What a page deletion does to the destinations that named the deleted pages (E2 plan §11):
//! each is retargeted to the nearest surviving page with `/Fit` (an explicit destination must
//! name a page object, ISO 32000-2 §12.3.2.2, and coordinates on another page mean nothing), or
//! removed. Outline items, links and `GoTo` actions on surviving pages, named destinations (the
//! `/Names /Dests` tree, rewritten whole, and the PDF 1.1 `/Dests` dictionary) and
//! `/OpenAction`.

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_cos::{Dict, ObjRef, Object};
use laminapdf_model::{NameStore, PageId, PageReference, ReferenceKind, TreeWriter, Via, set_catalog_entry};

use super::delete::ReferencePolicy;
use crate::error::Result;

/// What happened to one reference to a deleted page.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReferenceOutcome {
    /// It now names this page, with the view `/Fit`.
    Retargeted {
        /// The surviving page it names.
        to: PageId,
    },
    /// It was removed: an outline item's destination, a link annotation, a named destination,
    /// `/OpenAction`, or a structure element or kid whose content was on deleted pages.
    Removed,
}

/// A reference to a deleted page and what the deletion did to it.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ReferenceChange {
    /// The reference, as the document held it before the deletion.
    pub reference: PageReference,
    /// What happened to it.
    pub outcome: ReferenceOutcome,
}

impl ReferenceChange {
    pub(crate) const fn new(reference: PageReference, outcome: ReferenceOutcome) -> Self {
        Self { reference, outcome }
    }
}

/// The nearest page to `page` that survives: the next one in `before`, else the one before.
fn nearest<'a>(before: &'a [PageId], deleted: &BTreeSet<&PageId>, page: &PageId) -> Option<&'a PageId> {
    let at = before.iter().position(|p| p == page)?;
    let alive = |p: &&PageId| !deleted.contains(p);
    before
        .iter()
        .skip(at + 1)
        .find(alive)
        .or_else(|| before.iter().take(at).rev().find(alive))
}

/// `[page /Fit]`.
fn fit(page: &PageId) -> Option<Object> {
    let r = page.as_lamina_id().obj_ref()?;
    Some(Object::Array(vec![Object::Ref(r), Object::name("Fit")]))
}

/// Retargets or removes the destinations in `references` (all naming pages in `deleted`).
pub(crate) fn destinations(
    doc: &mut laminapdf_model::Document,
    references: &[PageReference],
    before: &[PageId],
    deleted: &[PageId],
    policy: ReferencePolicy,
) -> Result<Vec<ReferenceChange>> {
    let deleted: BTreeSet<&PageId> = deleted.iter().collect();
    let mut changes = Vec::new();
    let mut names: BTreeMap<(Vec<u8>, NameStoreKey), (PageReference, Option<Object>)> = BTreeMap::new();
    for reference in references {
        let target = nearest(before, &deleted, &reference.page);
        let new = match policy {
            ReferencePolicy::Retarget => target.and_then(fit),
            ReferencePolicy::Remove => None,
        };
        let done = match &reference.kind {
            ReferenceKind::OutlineItem { item, via } if named(via).is_none() => {
                let Some(holder) = item.as_lamina_id().obj_ref() else {
                    continue;
                };
                write_dest(doc, Holder::Object(holder), via, new.clone(), &["Dest", "A"])?;
                true
            }
            ReferenceKind::Link { annot, on, via } if named(via).is_none() && !deleted.contains(on) => {
                // A removed destination leaves the link on its page, inert, so the structure
                // tree's OBJR and parent-tree entry for it stay right.
                write_dest(doc, Holder::Object(*annot), via, new.clone(), &["Dest", "A"])?;
                true
            }
            ReferenceKind::OpenAction { via } if named(via).is_none() => {
                write_dest(doc, Holder::Catalog, via, new.clone(), &[])?;
                true
            }
            ReferenceKind::NamedDestination { name, store } => {
                names.insert(
                    (name.clone(), NameStoreKey::from(*store)),
                    (reference.clone(), new.clone()),
                );
                false
            }
            _ => false,
        };
        if done {
            changes.push(ReferenceChange::new(reference.clone(), outcome(new.as_ref(), target)));
        }
    }
    let mut tree: BTreeMap<Vec<u8>, Option<Object>> = BTreeMap::new();
    for ((name, store), (reference, new)) in names {
        match store {
            NameStoreKey::NameTree => {
                tree.insert(name, new.clone());
            }
            NameStoreKey::DestsDict => set_old_destination(doc, &name, new.as_ref())?,
        }
        let target = nearest(before, &deleted, &reference.page);
        changes.push(ReferenceChange::new(reference, outcome(new.as_ref(), target)));
    }
    set_tree_destinations(doc, &tree)?;
    Ok(changes)
}

fn outcome(new: Option<&Object>, target: Option<&PageId>) -> ReferenceOutcome {
    match (new, target) {
        (Some(_), Some(to)) => ReferenceOutcome::Retargeted { to: to.clone() },
        _ => ReferenceOutcome::Removed,
    }
}

/// The named destination a destination goes through, if any (that entry is retargeted on its
/// own, so links from other files keep working).
pub(super) fn named(via: &Via) -> Option<&[u8]> {
    match via {
        Via::Dest { name } | Via::GoTo { name, .. } => name.as_deref(),
        _ => None,
    }
}

/// `NameStore` with an order, for the map key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum NameStoreKey {
    NameTree,
    DestsDict,
}

impl From<NameStore> for NameStoreKey {
    fn from(store: NameStore) -> Self {
        match store {
            NameStore::DestsDict => Self::DestsDict,
            _ => Self::NameTree,
        }
    }
}

/// Where a destination is held: an object, or the catalog (`/OpenAction`).
#[derive(Clone, Copy)]
pub(super) enum Holder {
    Object(ObjRef),
    Catalog,
}

/// Writes `new` (or removes the destination, with `None`) where `via` says the holder keeps
/// it. On removal, `remove_keys` are taken off the holder (an outline item's `/Dest` and `/A`).
pub(super) fn write_dest(
    doc: &mut laminapdf_model::Document,
    holder: Holder,
    via: &Via,
    new: Option<Object>,
    remove_keys: &[&str],
) -> Result<()> {
    match (holder, via, new) {
        (Holder::Catalog, Via::Dest { .. }, new) => set_catalog(doc, "OpenAction", new),
        (
            Holder::Catalog,
            Via::GoTo {
                action: Some(action), ..
            },
            Some(dest),
        ) => edit_dict(doc, *action, |d| d.insert("D", dest)),
        (Holder::Catalog, Via::GoTo { action: None, .. }, Some(dest)) => {
            let current = doc.catalog()?.get("OpenAction").cloned();
            match current {
                Some(Object::Dict(mut action)) => {
                    action.insert("D", dest);
                    set_catalog(doc, "OpenAction", Some(Object::Dict(action)))
                }
                _ => Ok(()),
            }
        }
        (Holder::Catalog, _, None) => set_catalog(doc, "OpenAction", None),
        (Holder::Object(r), Via::Dest { .. }, Some(dest)) => edit_dict(doc, r, |d| d.insert("Dest", dest)),
        (
            Holder::Object(_),
            Via::GoTo {
                action: Some(action), ..
            },
            Some(dest),
        ) => edit_dict(doc, *action, |d| d.insert("D", dest)),
        (Holder::Object(r), Via::GoTo { action: None, .. }, Some(dest)) => edit_dict(doc, r, |d| {
            if let Some(Object::Dict(action)) = d.get_mut("A") {
                action.insert("D", dest);
            }
        }),
        (Holder::Object(r), _, None) => edit_dict(doc, r, |d| {
            for key in remove_keys {
                d.remove(key);
            }
        }),
        _ => Ok(()),
    }
}

/// Sets each named destination of `changes` in the `/Names /Dests` tree to its new
/// destination, or removes it with `None`, writing the tree again once. A destination
/// dictionary (`<< /D [...] >>`) keeps its other entries.
pub(super) fn set_tree_destinations(
    doc: &mut laminapdf_model::Document,
    changes: &BTreeMap<Vec<u8>, Option<Object>>,
) -> Result<()> {
    if changes.is_empty() {
        return Ok(());
    }
    let Some(tree) = doc.name_tree("Dests")? else {
        return Ok(());
    };
    let mut entries: BTreeMap<Vec<u8>, Object> = tree.entries()?.into_iter().collect();
    for (name, new) in changes {
        match (entries.get(name).cloned(), new) {
            (Some(old), Some(dest)) => {
                let value = with_dest(doc, &old, dest.clone())?;
                entries.insert(name.clone(), value);
            }
            (Some(_), None) => {
                entries.remove(name);
            }
            (None, _) => {}
        }
    }
    let root = TreeWriter::new().write_names(&mut doc.cos_mut().edit(), &entries)?;
    set_names_entry(doc, "Dests", Object::Ref(root))
}

/// Sets the named destination `name` in the catalog's PDF 1.1 `/Dests` dictionary to `new`,
/// or removes it with `None`. Names are matched byte for byte (a name need not be UTF-8), and
/// a destination dictionary keeps its other entries.
pub(super) fn set_old_destination(
    doc: &mut laminapdf_model::Document,
    name: &[u8],
    new: Option<&Object>,
) -> Result<()> {
    let Some(dests) = doc.catalog()?.get("Dests").cloned() else {
        return Ok(());
    };
    let (holder, dict) = match dests {
        Object::Ref(r) => match doc.object(r.num)?.as_dict() {
            Some(d) => (Some(r), d.clone()),
            None => return Ok(()),
        },
        Object::Dict(d) => (None, d),
        _ => return Ok(()),
    };
    let mut out = Dict::new();
    for (key, value) in dict.iter() {
        if key.0 != name {
            out.insert(key.clone(), value.clone());
            continue;
        }
        if let Some(dest) = new {
            let value = with_dest(doc, value, dest.clone())?;
            out.insert(key.clone(), value);
        }
    }
    match holder {
        Some(r) => edit_object(doc, r, |o| *o = Object::Dict(out)),
        None => set_catalog(doc, "Dests", Some(Object::Dict(out))),
    }
}

/// `old` (a destination array or a `<< /D … >>` dictionary, maybe indirect) with its
/// destination replaced by `dest`.
fn with_dest(doc: &laminapdf_model::Document, old: &Object, dest: Object) -> Result<Object> {
    Ok(match resolve(doc, Some(old.clone()))? {
        Some(Object::Dict(mut holder)) => {
            holder.insert("D", dest);
            Object::Dict(holder)
        }
        _ => dest,
    })
}

/// `value` with one reference followed.
pub(super) fn resolve(doc: &laminapdf_model::Document, value: Option<Object>) -> Result<Option<Object>> {
    Ok(match value {
        Some(Object::Ref(r)) => Some((*doc.object(r.num)?).clone()),
        other => other,
    })
}

/// Sets `/Names`'s `key` (the catalog's `/Names` dictionary, direct or indirect).
fn set_names_entry(doc: &mut laminapdf_model::Document, key: &str, value: Object) -> Result<()> {
    match doc.catalog()?.get("Names").cloned() {
        Some(Object::Ref(r)) => edit_dict(doc, r, |d| d.insert(key, value)),
        Some(Object::Dict(mut names)) => {
            names.insert(key, value);
            set_catalog(doc, "Names", Some(Object::Dict(names)))
        }
        _ => Ok(()),
    }
}

fn set_catalog(doc: &mut laminapdf_model::Document, key: &str, value: Option<Object>) -> Result<()> {
    Ok(set_catalog_entry(&mut doc.cos_mut().edit(), key, value)?)
}

/// Changes the dictionary object `r` with `f`; anything else is left alone.
pub(crate) fn edit_dict(doc: &mut laminapdf_model::Document, r: ObjRef, f: impl FnOnce(&mut Dict)) -> Result<()> {
    edit_object(doc, r, |o| {
        if let Object::Dict(d) = o {
            f(d);
        }
    })
}

/// Changes object `r` with `f` and writes it when it changed.
pub(crate) fn edit_object(doc: &mut laminapdf_model::Document, r: ObjRef, f: impl FnOnce(&mut Object)) -> Result<()> {
    let mut cos = doc.cos_mut().edit();
    let Some(current) = cos.get(r)? else { return Ok(()) };
    let mut object = (*current).clone();
    f(&mut object);
    if object != *current {
        cos.set(r, object)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use laminapdf_cos::ObjRef;

    use super::*;

    fn ids(nums: &[u32]) -> Vec<PageId> {
        nums.iter().map(|&n| PageId::object(ObjRef::new(n, 0))).collect()
    }

    #[test]
    fn the_nearest_page_is_the_next_survivor_else_the_one_before() {
        let before = ids(&[1, 2, 3, 4, 5]);
        let deleted = ids(&[2, 3, 5]);
        let set = deleted.iter().collect();
        assert_eq!(
            [&deleted[0], &deleted[1], &deleted[2]].map(|p| nearest(&before, &set, p).cloned()),
            [
                Some(before[3].clone()),
                Some(before[3].clone()),
                Some(before[3].clone())
            ]
        );
    }
}
