//! Writing the edited page tree: pruning empty nodes, rebuilding past the thresholds,
//! materialising inherited attributes, copying pages listed twice, and the node and page
//! dictionaries that changed.

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_cos::{Dict, ObjRef, Object, Result, Warning};

use super::PageTreeEditor;
use super::load::{Attrs, INHERITABLE, Kid, Node, Tree, overlay, own};
use crate::document::Document;
use crate::tree_writer::even_chunks;

/// Writes `tree` (edited from the document's page tree) into the overlay.
pub(super) fn finish(doc: &mut Document, mut tree: Tree) -> Result<()> {
    prune(&mut tree);
    if needs_rebuild(&tree) {
        rebuild(&mut tree);
    }
    let refs = node_refs(doc, &tree)?;
    let inherited = inherited(doc, &tree)?;
    let mut writes: Vec<(ObjRef, Object)> = Vec::new();
    write_pages(doc, &mut tree, &refs, &inherited, &mut writes)?;
    write_nodes(&tree, &refs, &mut writes);
    let dead: Vec<ObjRef> = tree.nodes.iter().filter(|n| !n.alive).filter_map(|n| n.r).collect();
    let mut cos = doc.cos_mut().edit();
    for (r, object) in writes {
        cos.set(r, object)?;
    }
    for r in dead {
        if cos.get(r)?.is_some() {
            cos.delete(r)?;
        }
    }
    Ok(())
}

/// Removes nodes (other than the root) left without kids, bottom up. A node's children come
/// after it in the arena, so one backward pass reaches every child before its parent.
fn prune(tree: &mut Tree) {
    for index in (1..tree.nodes.len()).rev() {
        let Some(node) = tree.nodes.get(index) else { continue };
        if !node.alive || !node.kids.is_empty() {
            continue;
        }
        let parent = node.parent;
        if let Some(node) = tree.nodes.get_mut(index) {
            node.alive = false;
        }
        if let Some(parent) = parent.and_then(|p| tree.nodes.get_mut(p)) {
            parent.kids.retain(|k| *k != Kid::Node(index));
            parent.touched = true;
        }
    }
}

/// Whether the tree passes a threshold ([`PageTreeEditor::MAX_DEPTH`],
/// [`PageTreeEditor::MAX_KIDS`] on a touched node) and can be rebuilt (no kid kept as is).
fn needs_rebuild(tree: &Tree) -> bool {
    let wide = tree
        .nodes
        .iter()
        .any(|n| n.alive && n.touched && n.kids.len() > PageTreeEditor::MAX_KIDS);
    let deep = depths(tree).values().any(|&d| d > PageTreeEditor::MAX_DEPTH);
    (wide || deep) && !tree.has_other_kids()
}

/// Each live node's depth: 1 for the root.
fn depths(tree: &Tree) -> BTreeMap<usize, usize> {
    let mut out = BTreeMap::from([(0, 1)]);
    for (node, depth) in top_down(tree) {
        out.insert(node, depth);
    }
    out
}

/// The live nodes from the root down (each after its parent), with their depths.
fn top_down(tree: &Tree) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut stack = vec![(0usize, 1usize)];
    while let Some((node, depth)) = stack.pop() {
        out.push((node, depth));
        if let Some(n) = tree.nodes.get(node) {
            stack.extend(n.kids.iter().filter_map(|k| match k {
                Kid::Node(child) => Some((*child, depth + 1)),
                _ => None,
            }));
        }
    }
    out
}

/// Replaces every node under the root with a balanced tree of [`PageTreeEditor::FAN_OUT`]
/// kids per node over the pages in order. The root keeps its object and attributes; every
/// page counts as moved, so what it inherited from the old nodes is written on it.
fn rebuild(tree: &mut Tree) {
    let order = tree.order();
    for node in tree.nodes.iter_mut().skip(1) {
        node.alive = false;
    }
    let mut level: Vec<Kid> = order.iter().map(|&s| Kid::Page(s)).collect();
    while level.len() > PageTreeEditor::FAN_OUT {
        let mut next = Vec::new();
        for group in even_chunks(level, PageTreeEditor::FAN_OUT) {
            tree.nodes.push(Node {
                r: None,
                dict: Dict::new(),
                parent: Some(0),
                kids: group,
                inherited: Attrs::default(),
                touched: true,
                alive: true,
            });
            next.push(Kid::Node(tree.nodes.len() - 1));
        }
        level = next;
    }
    if let Some(root) = tree.nodes.first_mut() {
        root.kids = level;
        root.touched = true;
    }
    // Parents follow the new kids.
    for index in 0..tree.nodes.len() {
        let Some(node) = tree.nodes.get(index).filter(|n| n.alive) else {
            continue;
        };
        let kids = node.kids.clone();
        for kid in kids {
            match kid {
                Kid::Node(child) => {
                    if let Some(c) = tree.nodes.get_mut(child) {
                        c.parent = Some(index);
                    }
                }
                Kid::Page(slot) => {
                    if let Some(s) = tree.slots.get_mut(slot) {
                        s.parent = Some(index);
                        s.moved = true;
                    }
                }
                Kid::Other { .. } => {}
            }
        }
    }
}

/// Every live node's object: its own, or a fresh number for a new node.
fn node_refs(doc: &Document, tree: &Tree) -> Result<BTreeMap<usize, ObjRef>> {
    let mut refs = BTreeMap::new();
    for (index, node) in tree.nodes.iter().enumerate().filter(|(_, n)| n.alive) {
        let r = match node.r {
            Some(r) => r,
            None => doc.reserve_number()?,
        };
        refs.insert(index, r);
    }
    Ok(refs)
}

/// What a page under each live node inherits now.
fn inherited(doc: &Document, tree: &Tree) -> Result<BTreeMap<usize, Attrs>> {
    let mut out: BTreeMap<usize, Attrs> = BTreeMap::new();
    for (index, _) in top_down(tree) {
        let Some(node) = tree.nodes.get(index) else { continue };
        let above = node.parent.and_then(|p| out.get(&p)).cloned().unwrap_or_default();
        out.insert(index, overlay(doc, &above, &node.dict)?);
    }
    Ok(out)
}

/// The page dictionaries to write: pages that moved (attributes materialised), pages whose
/// node changed (`/Parent`), and a copy of each page listed again after its first listing.
fn write_pages(
    doc: &Document,
    tree: &mut Tree,
    refs: &BTreeMap<usize, ObjRef>,
    inherited: &BTreeMap<usize, Attrs>,
    writes: &mut Vec<(ObjRef, Object)>,
) -> Result<()> {
    let mut seen = BTreeSet::new();
    for slot in tree.order() {
        let Some(s) = tree.slots.get(slot) else { continue };
        let Some(parent) = s.parent else { continue };
        let Some(&parent_ref) = refs.get(&parent) else { continue };
        let listed_again = !seen.insert(s.r);
        let parent_touched = tree.nodes.get(parent).is_some_and(|n| n.touched);
        let parent_wrong = s.dict.get("Parent") != Some(&Object::Ref(parent_ref));
        if !(s.moved || parent_touched || parent_wrong || listed_again) {
            continue;
        }
        let mut dict = s.dict.clone();
        if s.moved {
            let now = inherited.get(&parent).cloned().unwrap_or_default();
            materialise(doc, &mut dict, &s.desired, &now)?;
        }
        dict.insert("Parent", Object::Ref(parent_ref));
        if listed_again {
            doc.warn(Warning::InvalidEntry {
                object: Some(parent_ref.num),
                key: "Kids",
            })?;
            let copy = doc.reserve_number()?;
            if let Some(s) = tree.slots.get_mut(slot) {
                s.r = copy;
            }
            if let Some(n) = tree.nodes.get_mut(parent) {
                n.touched = true;
            }
            writes.push((copy, Object::Dict(dict)));
        } else {
            writes.push((s.r, Object::Dict(dict)));
        }
    }
    Ok(())
}

/// Writes on `dict` what the page must show (`desired`) where its new ancestors give
/// something else (`now`): the desired value, or the neutral one when it showed none.
fn materialise(doc: &Document, dict: &mut Dict, desired: &Attrs, now: &Attrs) -> Result<()> {
    let media = desired.get(1).cloned().flatten();
    for ((key, want), have) in INHERITABLE.into_iter().zip(desired).zip(now) {
        if own(doc, dict, key)?.is_some() || want == have {
            continue;
        }
        let value = match (want, key) {
            (Some(value), _) => Some(value.clone()),
            (None, "Rotate") => Some(Object::Integer(0)),
            (None, "Resources") => Some(Object::Dict(Dict::new())),
            // No crop box shows the media box (§14.11.2).
            (None, "CropBox") => media.clone(),
            // A missing media box can't be stated; the page keeps inheriting.
            (None, _) => None,
        };
        if let Some(value) = value {
            dict.insert(key, value);
        }
    }
    Ok(())
}

/// The node dictionaries to write: new and touched nodes (`/Kids`), and any node whose
/// `/Count` or `/Parent` differs from what it stores.
fn write_nodes(tree: &Tree, refs: &BTreeMap<usize, ObjRef>, writes: &mut Vec<(ObjRef, Object)>) {
    let order = top_down(tree);
    let mut counts: BTreeMap<usize, i64> = BTreeMap::new();
    for &(index, _) in order.iter().rev() {
        let Some(node) = tree.nodes.get(index) else { continue };
        let count = node
            .kids
            .iter()
            .map(|k| match k {
                Kid::Page(_) => 1,
                Kid::Node(child) => counts.get(child).copied().unwrap_or(0),
                Kid::Other { pages, .. } => i64::try_from(*pages).unwrap_or(i64::MAX),
            })
            .fold(0i64, i64::saturating_add);
        counts.insert(index, count);
    }
    for (index, _) in order {
        let (Some(node), Some(&r)) = (tree.nodes.get(index), refs.get(&index)) else {
            continue;
        };
        let count = counts.get(&index).copied().unwrap_or(0);
        let parent = node.parent.filter(|_| index != 0).and_then(|p| refs.get(&p)).copied();
        let count_wrong = node.dict.get("Count").and_then(Object::as_int) != Some(count);
        let parent_wrong = parent.is_some_and(|p| node.dict.get("Parent") != Some(&Object::Ref(p)));
        if !(node.touched || node.r.is_none() || count_wrong || parent_wrong) {
            continue;
        }
        let mut dict = node.dict.clone();
        dict.insert("Type", Object::name("Pages"));
        if node.touched || node.r.is_none() {
            dict.insert(
                "Kids",
                Object::Array(node.kids.iter().map(|k| kid_object(tree, refs, k)).collect()),
            );
        }
        dict.insert("Count", Object::Integer(count));
        if let Some(parent) = parent {
            dict.insert("Parent", Object::Ref(parent));
        }
        writes.push((r, Object::Dict(dict)));
    }
}

fn kid_object(tree: &Tree, refs: &BTreeMap<usize, ObjRef>, kid: &Kid) -> Object {
    match kid {
        Kid::Node(node) => refs.get(node).map_or(Object::Null, |r| Object::Ref(*r)),
        Kid::Page(slot) => tree.slots.get(*slot).map_or(Object::Null, |s| Object::Ref(s.r)),
        Kid::Other { value, .. } => value.clone(),
    }
}
