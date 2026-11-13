//! Bringing a source's tags along with its pages (ISO 32000-2 §14.7; E2 plan §2.2): the
//! structure elements that own content on the imported pages, and their ancestors, are copied
//! into the target's structure tree, below the target's top element (or a new `Part` per source
//! on a merge). A source's own top `Document` element isn't copied: its children take its
//! place. Each copied page, annotation and Form XObject gets a fresh `/StructParents` or
//! `/StructParent` key from the target's `/ParentTreeNextKey`, whose `/ParentTree` entry names
//! the copies of its elements; MCIDs are per content stream, so they stay as they are.
//!
//! An element keeps only the kids that came along: marked content on copied pages, object
//! references to copied annotations and child elements that were copied. The source's
//! `/RoleMap`, `/ClassMap` and namespaces join the target's; a role or class name the target
//! already maps another way, and an element ID it already uses, take the next free
//! `<name>#<n>`, reported as [`StructureRename`]s.
//!
//! When the caller drops the tags (an untagged target, or a merge with an untagged source),
//! every `/StructParents` and `/StructParent` the copies carry is removed, and the pages that
//! had them are reported.

use std::collections::{BTreeMap, BTreeSet};

use laminapdf_cos::{Dict, Name, ObjRef, Object, PdfString};
use laminapdf_model::{Copied, NameTree, ObjectCopier, PageId, StructTree, TreeWriter, set_catalog_entry};

use super::cleanup::{edit_dict, edit_object};
use crate::error::Result;

/// What becomes of the tags of pages copied in from another document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Tags {
    /// Copied into the target's structure tree, below its top element.
    Keep,
    /// Copied below a new `Part` element of the target's top element, titled as given.
    KeepInPart(Option<String>),
    /// Dropped: the copies carry no structure keys.
    Drop,
}

impl Tags {
    /// Whether pages copied from `sources` into `doc` keep their tags: when `doc` is tagged,
    /// or has no pages (`empty`) and every source is tagged, so the result is tagged
    /// throughout (ISO 32000-2 §14.8: a tagged file's content is all tagged or artifacts).
    pub(crate) fn kept(
        doc: &laminapdf_model::Document,
        empty: bool,
        sources: &[&laminapdf_model::Document],
    ) -> Result<bool> {
        if doc.is_tagged()? {
            return Ok(true);
        }
        if !empty {
            return Ok(false);
        }
        for src in sources {
            if !src.is_tagged()? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

/// Which structure name a [`StructureRename`] is.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum StructureName {
    /// A structure type the `/RoleMap` maps (§14.7.3).
    Role,
    /// An attribute class of the `/ClassMap` (§14.7.6.2).
    Class,
    /// An element's `/ID`, which the `/IDTree` maps (§14.7.2).
    Id,
}

/// A role, class or element ID that pages copied in from another document brought under
/// another name, because the document already used the name another way.
///
/// Added in 0.3.0.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct StructureRename {
    /// What kind of name it is.
    pub kind: StructureName,
    /// Its name in the source, decoded as UTF-8 (bytes that aren't are replaced).
    pub from: String,
    /// Its name in this document.
    pub to: String,
}

/// What the structure step did.
#[derive(Debug, Default)]
pub(crate) struct TagOutcome {
    /// The copied pages whose tags were dropped.
    pub(crate) dropped: Vec<PageId>,
    /// The names that came in under another name.
    pub(crate) renamed: Vec<StructureRename>,
}

/// An object among the copies that carries a structure key.
struct Keyed {
    object: ObjRef,
    key: &'static str,
    old: i64,
}

/// Brings the tags of `pages` (the copied pages, and of the annotations and Form XObjects copied with them,
/// objects numbered from `first_new`) into `doc`, or drops them, as `tags` says.
pub(crate) fn import_tags(
    doc: &mut laminapdf_model::Document,
    src: &laminapdf_model::Document,
    copier: &mut ObjectCopier<'_>,
    (first_new, pages): (u64, &[ObjRef]),
    tags: &Tags,
) -> Result<TagOutcome> {
    let keyed = keyed_objects(doc, first_new, pages)?;
    if keyed.is_empty() {
        return Ok(TagOutcome::default());
    }
    let source = match (tags, src.structure()?, src_root(src)?) {
        (Tags::Drop, ..) | (_, None, _) | (_, _, None) => None,
        (_, Some(tree), Some(root)) => tree.parent_tree.clone().map(|pt| (tree, root, pt)),
    };
    let Some((tree, src_root_ref, parent_tree)) = source else {
        let dropped = tagged_pages(doc, pages, &keyed)?;
        for k in &keyed {
            edit_object(doc, k.object, |o| {
                if let Some(d) = dict_mut(o) {
                    d.remove(k.key);
                }
            })?;
        }
        return Ok(TagOutcome {
            dropped,
            renamed: Vec::new(),
        });
    };
    let entries: BTreeMap<i64, Object> = src.number_tree(parent_tree).entries()?.into_iter().collect();
    let mut merge = Merge {
        src,
        tree: &tree,
        src_root: src_root_ref,
        needed: BTreeSet::new(),
        made: BTreeMap::new(),
        renamed: Vec::new(),
    };
    for k in &keyed {
        if let Some(value) = entries.get(&k.old) {
            for owner in owners(src, value)? {
                merge.need(owner)?;
            }
        }
    }
    let target = Target::open(doc, tags)?;
    let maps = merge.maps(doc, copier, &target)?;
    let ids = merge.copy_elements(doc, copier, &target, &maps)?;
    renumber(doc, &target, (&keyed, &entries), &merge.made, src)?;
    maps.write(doc, &target, ids)?;
    Ok(TagOutcome {
        dropped: Vec::new(),
        renamed: merge.renamed,
    })
}

/// The copies that carry `/StructParents` or `/StructParent`: the pages, the annotations their
/// `/Annots` list, and any other copied object (a Form XObject).
fn keyed_objects(doc: &laminapdf_model::Document, first_new: u64, copies: &[ObjRef]) -> Result<Vec<Keyed>> {
    let mut listed = BTreeSet::new();
    for &page in copies {
        listed.extend(annots_of(doc, page)?);
    }
    let created: Vec<u32> = doc
        .overlay()
        .iter()
        .map(|(num, _)| num)
        .filter(|&num| u64::from(num) >= first_new)
        .collect();
    let mut out = Vec::new();
    for num in created {
        let object = doc.object(num)?;
        let Some(dict) = dict_of(&object) else { continue };
        let r = ObjRef::new(num, 0);
        // An annotation that no copied page lists (a link removed for going nowhere) is about
        // to be swept away.
        let annot = dict.contains_key("Rect") && dict.contains_key("Subtype") && !matches!(&*object, Object::Stream(_));
        if annot && !listed.contains(&r) {
            continue;
        }
        for key in ["StructParents", "StructParent"] {
            if let Some(old) = dict.get_int(key) {
                out.push(Keyed { object: r, key, old });
            }
        }
    }
    Ok(out)
}

/// The references in `page`'s `/Annots`.
fn annots_of(doc: &laminapdf_model::Document, page: ObjRef) -> Result<Vec<ObjRef>> {
    let object = doc.object(page.num)?;
    let Some(annots) = object.as_dict().and_then(|d| d.get("Annots")) else {
        return Ok(Vec::new());
    };
    Ok(match &*doc.deref(annots)? {
        Object::Array(items) => items.iter().filter_map(Object::as_ref).collect(),
        _ => Vec::new(),
    })
}

/// The copied pages that carry structure keys themselves or through an annotation they list.
fn tagged_pages(doc: &laminapdf_model::Document, copies: &[ObjRef], keyed: &[Keyed]) -> Result<Vec<PageId>> {
    let objects: BTreeSet<ObjRef> = keyed.iter().map(|k| k.object).collect();
    let mut out = Vec::new();
    for &page in copies {
        if objects.contains(&page) || annots_of(doc, page)?.iter().any(|a| objects.contains(a)) {
            out.push(PageId::object(page));
        }
    }
    Ok(out)
}

/// The source's `/StructTreeRoot` object.
fn src_root(src: &laminapdf_model::Document) -> Result<Option<ObjRef>> {
    Ok(src.catalog()?.get("StructTreeRoot").and_then(Object::as_ref))
}

/// The elements a `/ParentTree` value names: an array of them (one per MCID, `null` for none),
/// or one element (an annotation's or XObject's).
fn owners(src: &laminapdf_model::Document, value: &Object) -> Result<Vec<ObjRef>> {
    Ok(match &*src.deref(value)? {
        Object::Array(items) => items.iter().filter_map(Object::as_ref).collect(),
        Object::Dict(_) => value.as_ref().into_iter().collect(),
        _ => Vec::new(),
    })
}

/// The target's structure tree root and the element the copies go under.
struct Target {
    root: ObjRef,
    container: ObjRef,
    /// The target's tree as it was (empty when it had none).
    tree: StructTree,
}

impl Target {
    /// Opens the target's tree, creating a root and a `Document` element when it has none, and
    /// a `Part` for `tags`' title when it asks for one.
    fn open(doc: &mut laminapdf_model::Document, tags: &Tags) -> Result<Self> {
        let existing = doc.catalog()?.get("StructTreeRoot").and_then(Object::as_ref);
        let tree = doc.structure()?.unwrap_or_default();
        let (root, mut container) = if let Some(root) = existing {
            let top = match tree.roots.as_slice() {
                [only] if tree.standard_kind(&only.kind) == b"Document" => only.object,
                _ => None,
            };
            (root, top.unwrap_or(root))
        } else {
            let root = doc.cos_mut().edit().create(Object::Null)?;
            let document: Dict = [
                ("Type", Object::name("StructElem")),
                ("S", Object::name("Document")),
                ("P", Object::Ref(root)),
            ]
            .into_iter()
            .collect();
            let top = doc.cos_mut().edit().create(Object::Dict(document))?;
            let root_dict: Dict = [
                ("Type", Object::name("StructTreeRoot")),
                ("K", Object::Array(vec![Object::Ref(top)])),
            ]
            .into_iter()
            .collect();
            doc.cos_mut().edit().set(root, Object::Dict(root_dict))?;
            set_catalog_entry(&mut doc.cos_mut().edit(), "StructTreeRoot", Some(Object::Ref(root)))?;
            mark_tagged(doc)?;
            (root, top)
        };
        if let Tags::KeepInPart(title) = tags {
            let mut part: Dict = [
                ("Type", Object::name("StructElem")),
                ("S", Object::name("Part")),
                ("P", Object::Ref(container)),
            ]
            .into_iter()
            .collect();
            if let Some(title) = title {
                part.insert("T", crate::outline::title(title));
            }
            let part = doc.cos_mut().edit().create(Object::Dict(part))?;
            append_kids(doc, container, &[part])?;
            container = part;
        }
        Ok(Self { root, container, tree })
    }
}

/// Sets `/MarkInfo /Marked true` on the catalog, keeping the dictionary's other entries.
fn mark_tagged(doc: &mut laminapdf_model::Document) -> Result<()> {
    let mut info = match doc.catalog()?.get("MarkInfo") {
        Some(m) => doc.deref(m)?.as_dict().cloned().unwrap_or_default(),
        None => Dict::new(),
    };
    info.insert("Marked", Object::Bool(true));
    set_catalog_entry(&mut doc.cos_mut().edit(), "MarkInfo", Some(Object::Dict(info)))?;
    Ok(())
}

/// The names the copies use, as the target holds them, and what to add to the target's maps.
#[derive(Default)]
struct Maps {
    /// Source role or class name → the target's, where they differ.
    roles: BTreeMap<Vec<u8>, Vec<u8>>,
    classes: BTreeMap<Vec<u8>, Vec<u8>>,
    /// Source namespace object → the target's.
    namespaces: BTreeMap<ObjRef, Object>,
    /// Entries to add to the target's `/RoleMap`, `/ClassMap` and `/Namespaces`.
    role_map: Vec<(Vec<u8>, Vec<u8>)>,
    class_map: Vec<(Vec<u8>, Object)>,
    new_namespaces: Vec<Object>,
    /// The element IDs the target uses.
    ids: BTreeSet<Vec<u8>>,
}

impl Maps {
    /// Writes the new map entries and namespaces, and `ids` (the copies' IDs) into the
    /// target's `/IDTree`.
    fn write(self, doc: &mut laminapdf_model::Document, target: &Target, ids: BTreeMap<Vec<u8>, Object>) -> Result<()> {
        let root = doc.object(target.root.num)?.as_dict().cloned().unwrap_or_default();
        let id_tree = if ids.is_empty() {
            None
        } else {
            let mut all: BTreeMap<Vec<u8>, Object> = match root.get("IDTree") {
                Some(t) => NameTree::new(doc, t.clone()).entries()?.into_iter().collect(),
                None => BTreeMap::new(),
            };
            all.extend(ids);
            Some(TreeWriter::new().write_names(&mut doc.cos_mut().edit(), &all)?)
        };
        let mut role_map = map_of(doc, root.get("RoleMap"))?;
        let roles_added = !self.role_map.is_empty();
        for (k, v) in self.role_map {
            role_map.insert(Name::new(k), Object::Name(Name::new(v)));
        }
        let mut class_map = map_of(doc, root.get("ClassMap"))?;
        let classes_added = !self.class_map.is_empty();
        for (k, v) in self.class_map {
            class_map.insert(Name::new(k), v);
        }
        let namespaces = if self.new_namespaces.is_empty() {
            None
        } else {
            let mut list = match root.get("Namespaces") {
                Some(ns) => match &*doc.deref(ns)? {
                    Object::Array(items) => items.clone(),
                    _ => Vec::new(),
                },
                None => Vec::new(),
            };
            list.extend(self.new_namespaces);
            Some(list)
        };
        edit_dict(doc, target.root, |d| {
            if roles_added {
                d.insert("RoleMap", Object::Dict(role_map));
            }
            if classes_added {
                d.insert("ClassMap", Object::Dict(class_map));
            }
            if let Some(list) = namespaces {
                d.insert("Namespaces", Object::Array(list));
            }
            if let Some(tree) = id_tree {
                d.insert("IDTree", Object::Ref(tree));
            }
        })
    }
}

/// A dictionary value (perhaps indirect), resolved; empty when absent or not a dictionary.
fn map_of(doc: &laminapdf_model::Document, value: Option<&Object>) -> Result<Dict> {
    Ok(match value {
        Some(v) => doc.deref(v)?.as_dict().cloned().unwrap_or_default(),
        None => Dict::new(),
    })
}

/// The first `<name>#<n>`, `n` from 2, that `taken` refuses.
fn unique(name: &[u8], taken: impl Fn(&[u8]) -> bool) -> Vec<u8> {
    (2..=u32::MAX)
        .map(|n| [name, format!("#{n}").as_bytes()].concat())
        .find(|candidate| !taken(candidate))
        .unwrap_or_else(|| name.to_vec())
}

/// The elements being copied from one source.
struct Merge<'a> {
    src: &'a laminapdf_model::Document,
    tree: &'a StructTree,
    src_root: ObjRef,
    /// Source elements to copy: the owners and their ancestors below the root.
    needed: BTreeSet<ObjRef>,
    /// Source element → its copy.
    made: BTreeMap<ObjRef, ObjRef>,
    renamed: Vec<StructureRename>,
}

impl Merge<'_> {
    /// Marks `elem` and its ancestors (below the root) as needed.
    fn need(&mut self, elem: ObjRef) -> Result<()> {
        let mut current = Some(elem);
        for _ in 0..256 {
            let Some(r) = current else { break };
            if r == self.src_root || self.needed.contains(&r) {
                break;
            }
            let object = self.src.object(r.num)?;
            let Some(dict) = object.as_dict() else { break };
            self.needed.insert(r);
            current = dict.get("P").and_then(Object::as_ref);
        }
        Ok(())
    }

    /// Works out the names the copies take in the target, and the map entries to add.
    fn maps(
        &mut self,
        doc: &mut laminapdf_model::Document,
        copier: &mut ObjectCopier<'_>,
        target: &Target,
    ) -> Result<Maps> {
        let mut maps = Maps::default();
        let theirs = &target.tree;
        let ours = self.tree;
        for (k, v) in &ours.role_map {
            if theirs.role_map.get(k).is_some_and(|t| t != v) {
                let to = unique(k, |c| theirs.role_map.contains_key(c) || ours.role_map.contains_key(c));
                self.rename(StructureName::Role, k, &to);
                maps.roles.insert(k.clone(), to);
            }
        }
        for (k, v) in &ours.role_map {
            let to = maps.roles.get(k).unwrap_or(k).clone();
            let value = maps.roles.get(v).unwrap_or(v).clone();
            if theirs.role_map.get(&to) != Some(&value) {
                maps.role_map.push((to, value));
            }
        }
        for (k, v) in &ours.class_map {
            let value = self.src.deref(v)?.into_owned();
            let to = match theirs.class_map.get(k) {
                Some(t) if *doc.deref(t)? == value => continue,
                Some(_) => {
                    let to = unique(k, |c| {
                        theirs.class_map.contains_key(c) || ours.class_map.contains_key(c)
                    });
                    self.rename(StructureName::Class, k, &to);
                    maps.classes.insert(k.clone(), to.clone());
                    to
                }
                None => k.clone(),
            };
            let class = copier.copy_value(&mut doc.cos_mut().edit(), v)?;
            maps.class_map.push((to, class));
        }
        let mut uris = BTreeMap::new();
        for ns in &theirs.namespaces {
            if let Some(uri) = ns_uri(doc, ns)? {
                uris.insert(uri, ns.clone());
            }
        }
        for ns in &ours.namespaces {
            let Some(r) = ns.as_ref() else { continue };
            let mapped = if let Some(existing) = ns_uri(self.src, ns)?.and_then(|u| uris.get(&u).cloned()) {
                existing
            } else {
                let namespace = copier.copy_value(&mut doc.cos_mut().edit(), ns)?;
                maps.new_namespaces.push(namespace.clone());
                namespace
            };
            maps.namespaces.insert(r, mapped);
        }
        if let Some(t) = &theirs.id_tree {
            maps.ids = NameTree::new(doc, t.clone())
                .entries()?
                .into_iter()
                .map(|(k, _)| k)
                .collect();
        }
        Ok(maps)
    }

    fn rename(&mut self, kind: StructureName, from: &[u8], to: &[u8]) {
        self.renamed.push(StructureRename {
            kind,
            from: String::from_utf8_lossy(from).into_owned(),
            to: String::from_utf8_lossy(to).into_owned(),
        });
    }

    /// Whether `elem` is a top `Document` element of the source, whose children take its
    /// place in the target.
    fn unwrapped(&self, elem: ObjRef) -> Result<bool> {
        let object = self.src.object(elem.num)?;
        let Some(dict) = object.as_dict() else { return Ok(false) };
        let top = dict
            .get("P")
            .and_then(Object::as_ref)
            .is_none_or(|p| p == self.src_root);
        let kind = dict.get_name("S").unwrap_or_default();
        Ok(top && self.tree.standard_kind(kind) == b"Document")
    }

    /// Copies the needed elements, puts the top ones below the target's container in the
    /// source's order, and returns the copies' IDs for the target's `/IDTree`.
    fn copy_elements(
        &mut self,
        doc: &mut laminapdf_model::Document,
        copier: &mut ObjectCopier<'_>,
        target: &Target,
        maps: &Maps,
    ) -> Result<BTreeMap<Vec<u8>, Object>> {
        let mut skip = BTreeSet::new();
        for &e in &self.needed {
            if self.unwrapped(e)? {
                skip.insert(e);
            }
        }
        for &e in &self.needed {
            if !skip.contains(&e) {
                let copy = doc.cos_mut().edit().create(Object::Null)?;
                self.made.insert(e, copy);
            }
        }
        let mut taken = maps.ids.clone();
        let mut ids = BTreeMap::new();
        let elems: Vec<(ObjRef, ObjRef)> = self.made.iter().map(|(&s, &c)| (s, c)).collect();
        for (source, copy) in elems {
            let object = self.src.object(source.num)?;
            let Some(dict) = object.as_dict() else { continue };
            let parent = dict
                .get("P")
                .and_then(Object::as_ref)
                .and_then(|p| self.made.get(&p).copied())
                .unwrap_or(target.container);
            let mut out = self.element(doc, copier, dict, maps)?;
            out.insert("P", Object::Ref(parent));
            if let Some(Object::String(id)) = dict.get("ID").map(|i| self.src.deref(i)).transpose()?.as_deref() {
                let mut id = id.bytes.clone();
                if taken.contains(&id) {
                    let to = unique(&id, |c| taken.contains(c));
                    self.rename(StructureName::Id, &id, &to);
                    id = to;
                }
                taken.insert(id.clone());
                out.insert("ID", Object::String(PdfString::literal(id.clone())));
                ids.insert(id, Object::Ref(copy));
            }
            doc.cos_mut().edit().set(copy, Object::Dict(out))?;
        }
        // The top elements, in the source's order: the root's kids, a skipped `Document`'s kids
        // in its place.
        let mut tops = Vec::new();
        let root = self.src.object(self.src_root.num)?;
        let kids = match root.as_dict().and_then(|d| d.get("K")) {
            Some(k) => kid_list(self.src, k)?,
            None => Vec::new(),
        };
        for kid in kids {
            let Some(r) = kid.as_ref() else { continue };
            if skip.contains(&r) {
                let object = self.src.object(r.num)?;
                let inner = match object.as_dict().and_then(|d| d.get("K")) {
                    Some(k) => kid_list(self.src, k)?,
                    None => Vec::new(),
                };
                tops.extend(
                    inner
                        .iter()
                        .filter_map(Object::as_ref)
                        .filter_map(|c| self.made.get(&c).copied()),
                );
            } else if let Some(&copy) = self.made.get(&r) {
                tops.push(copy);
            }
        }
        append_kids(doc, target.container, &tops)?;
        Ok(ids)
    }

    /// The copy of the element `dict`, without `/P` and `/ID`: its type and classes under the
    /// target's names, `/Pg` and kids pointed at the copies (kids that didn't come along left
    /// out), its other entries copied.
    fn element(
        &self,
        doc: &mut laminapdf_model::Document,
        copier: &mut ObjectCopier<'_>,
        dict: &Dict,
        maps: &Maps,
    ) -> Result<Dict> {
        let page = dict.get("Pg").and_then(Object::as_ref);
        let mut out = Dict::new();
        for (key, value) in dict.iter() {
            match key.as_bytes() {
                b"P" | b"ID" | b"K" => {}
                b"Pg" => {
                    if let Some(c) = page.and_then(|p| copied(copier, p)) {
                        out.insert("Pg", Object::Ref(c));
                    }
                }
                b"S" => {
                    if let Object::Name(n) = value {
                        let name = maps
                            .roles
                            .get(n.as_bytes())
                            .cloned()
                            .unwrap_or_else(|| n.as_bytes().to_vec());
                        out.insert("S", Object::Name(Name::new(name)));
                    }
                }
                b"C" => out.insert("C", rename_classes(&self.src.deref(value)?.into_owned(), &maps.classes)),
                b"Ref" => {
                    let refs: Vec<Object> = match &*self.src.deref(value)? {
                        Object::Array(items) => items
                            .iter()
                            .filter_map(Object::as_ref)
                            .filter_map(|r| self.made.get(&r))
                            .map(|&c| Object::Ref(c))
                            .collect(),
                        _ => Vec::new(),
                    };
                    if !refs.is_empty() {
                        out.insert("Ref", Object::Array(refs));
                    }
                }
                b"NS" => {
                    if let Some(ns) = value.as_ref().and_then(|r| maps.namespaces.get(&r)) {
                        out.insert("NS", ns.clone());
                    }
                }
                _ => {
                    let entry = copier.copy_value(&mut doc.cos_mut().edit(), value)?;
                    out.insert(key.clone(), entry);
                }
            }
        }
        if let Some(k) = dict.get("K") {
            let mut kids = Vec::new();
            for kid in kid_list(self.src, k)? {
                if let Some(mapped) = self.kid(doc, copier, &kid, page)? {
                    kids.push(mapped);
                }
            }
            if kids.len() > 1 {
                out.insert("K", Object::Array(kids));
            } else if let Some(only) = kids.pop() {
                out.insert("K", only);
            }
        }
        Ok(out)
    }

    /// A kid of an element on `page`, as the copy holds it; `None` when what it names didn't
    /// come along.
    fn kid(
        &self,
        doc: &mut laminapdf_model::Document,
        copier: &ObjectCopier<'_>,
        kid: &Object,
        page: Option<ObjRef>,
    ) -> Result<Option<Object>> {
        match kid {
            Object::Integer(_) => Ok(page.and_then(|p| copied(copier, p)).map(|_| kid.clone())),
            Object::Ref(r) => {
                if let Some(&c) = self.made.get(r) {
                    return Ok(Some(Object::Ref(c)));
                }
                let object = self.src.object(r.num)?;
                let Some(d) = object.as_dict() else { return Ok(None) };
                match content_ref(copier, d, page) {
                    Some(mapped) => Ok(Some(Object::Ref(doc.cos_mut().edit().create(Object::Dict(mapped))?))),
                    None => Ok(None),
                }
            }
            Object::Dict(d) => Ok(content_ref(copier, d, page).map(Object::Dict)),
            _ => Ok(None),
        }
    }
}

/// What source object `r` became in the target, when it was copied as an object.
fn copied(copier: &ObjectCopier<'_>, r: ObjRef) -> Option<ObjRef> {
    match copier.copy_of(r) {
        Some(Copied::Object(c)) => Some(c),
        _ => None,
    }
}

/// A marked-content reference (`/Type /MCR`) or object reference (`/Type /OBJR`) pointed at the
/// copies of its page, stream and object; `None` for anything else, or one whose page, stream or
/// object didn't come along.
fn content_ref(copier: &ObjectCopier<'_>, d: &Dict, page: Option<ObjRef>) -> Option<Dict> {
    let pg = d.get("Pg").and_then(Object::as_ref).or(page)?;
    let pg_copy = copied(copier, pg)?;
    let mut out = d.clone();
    if d.contains_key("Pg") {
        out.insert("Pg", Object::Ref(pg_copy));
    }
    match d.get_name("Type") {
        Some(b"OBJR") => {
            let obj = d.get("Obj").and_then(Object::as_ref)?;
            out.insert("Obj", Object::Ref(copied(copier, obj)?));
        }
        Some(b"MCR") => {
            for key in ["Stm", "StmOwn"] {
                if let Some(stm) = d.get(key).and_then(Object::as_ref) {
                    out.insert(key, Object::Ref(copied(copier, stm)?));
                }
            }
        }
        _ => return None,
    }
    Some(out)
}

/// `/C` with each class renamed as `classes` says.
fn rename_classes(value: &Object, classes: &BTreeMap<Vec<u8>, Vec<u8>>) -> Object {
    let rename = |o: &Object| match o {
        Object::Name(n) => match classes.get(n.as_bytes()) {
            Some(to) => Object::Name(Name::new(to.clone())),
            None => o.clone(),
        },
        other => other.clone(),
    };
    match value {
        Object::Array(items) => Object::Array(items.iter().map(rename).collect()),
        other => rename(other),
    }
}

/// The namespace URI (`/NS`) of a namespace dictionary.
fn ns_uri(doc: &laminapdf_model::Document, ns: &Object) -> Result<Option<Vec<u8>>> {
    let resolved = doc.deref(ns)?;
    let Some(d) = resolved.as_dict() else { return Ok(None) };
    Ok(match d.get("NS").map(|v| doc.deref(v)).transpose()?.as_deref() {
        Some(Object::String(s)) => Some(s.bytes.clone()),
        _ => None,
    })
}

/// The items of a `/K`: an array (perhaps indirect), else the one value.
fn kid_list(doc: &laminapdf_model::Document, k: &Object) -> Result<Vec<Object>> {
    Ok(match k {
        Object::Array(items) => items.clone(),
        Object::Ref(_) => match &*doc.deref(k)? {
            Object::Array(items) => items.clone(),
            _ => vec![k.clone()],
        },
        other => vec![other.clone()],
    })
}

/// Appends `items` to `parent`'s `/K` (into the array object `/K` refers to, when it does).
fn append_kids(doc: &mut laminapdf_model::Document, parent: ObjRef, items: &[ObjRef]) -> Result<()> {
    if items.is_empty() {
        return Ok(());
    }
    let refs: Vec<Object> = items.iter().map(|&r| Object::Ref(r)).collect();
    let indirect = match doc.object(parent.num)?.as_dict().and_then(|d| d.get("K")) {
        Some(Object::Ref(r)) if doc.object(r.num)?.as_array().is_some() => Some(*r),
        _ => None,
    };
    if let Some(list) = indirect {
        return edit_object(doc, list, |o| {
            if let Object::Array(existing) = o {
                existing.extend(refs);
            }
        });
    }
    edit_dict(doc, parent, |d| {
        let mut existing = match d.get("K") {
            Some(Object::Array(existing)) => existing.clone(),
            Some(single) => vec![single.clone()],
            None => Vec::new(),
        };
        existing.extend(refs);
        d.insert("K", Object::Array(existing));
    })
}

/// Gives each keyed copy a fresh key whose `/ParentTree` entry names the copies of its source
/// entry's elements (`null` where an element wasn't copied); one whose entry names nothing that
/// was copied loses its key.
fn renumber(
    doc: &mut laminapdf_model::Document,
    target: &Target,
    (keyed, entries): (&[Keyed], &BTreeMap<i64, Object>),
    made: &BTreeMap<ObjRef, ObjRef>,
    src: &laminapdf_model::Document,
) -> Result<()> {
    let root = doc.object(target.root.num)?.as_dict().cloned().unwrap_or_default();
    let mut tree: BTreeMap<i64, Object> = match root.get("ParentTree") {
        Some(t) => doc.number_tree(t.clone()).entries()?.into_iter().collect(),
        None => BTreeMap::new(),
    };
    let after_last = tree.keys().next_back().map_or(0, |k| k.saturating_add(1));
    let mut next = root.get_int("ParentTreeNextKey").unwrap_or(0).max(after_last);
    for k in keyed {
        let stored = entries.get(&k.old);
        let value = match stored.map(|v| src.deref(v)).transpose()?.as_deref() {
            Some(Object::Array(items)) => {
                let mapped: Vec<Object> = items
                    .iter()
                    .map(|i| {
                        i.as_ref()
                            .and_then(|r| made.get(&r))
                            .map_or(Object::Null, |&c| Object::Ref(c))
                    })
                    .collect();
                mapped
                    .iter()
                    .any(|o| matches!(o, Object::Ref(_)))
                    .then_some(Object::Array(mapped))
            }
            Some(Object::Dict(_)) => stored
                .and_then(Object::as_ref)
                .and_then(|r| made.get(&r))
                .map(|&c| Object::Ref(c)),
            _ => None,
        };
        let key = value.map(|v| {
            tree.insert(next, v);
            next += 1;
            next - 1
        });
        edit_object(doc, k.object, |o| {
            if let Some(d) = dict_mut(o) {
                match key {
                    Some(key) => d.insert(k.key, Object::Integer(key)),
                    None => {
                        d.remove(k.key);
                    }
                }
            }
        })?;
    }
    let written = TreeWriter::new().write_numbers(&mut doc.cos_mut().edit(), &tree)?;
    edit_dict(doc, target.root, |d| {
        d.insert("ParentTree", Object::Ref(written));
        d.insert("ParentTreeNextKey", Object::Integer(next));
    })
}

/// The dictionary of a dictionary or a stream.
fn dict_of(object: &Object) -> Option<&Dict> {
    match object {
        Object::Stream(s) => Some(&s.dict),
        Object::Dict(d) => Some(d),
        _ => None,
    }
}

/// The dictionary of a dictionary or a stream, to change.
fn dict_mut(object: &mut Object) -> Option<&mut Dict> {
    match object {
        Object::Stream(s) => Some(&mut s.dict),
        Object::Dict(d) => Some(d),
        _ => None,
    }
}
