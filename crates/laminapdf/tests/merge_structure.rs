//! Tags kept through page imports and merges: the pages of one tagged golden inserted into
//! another, saved and reopened, keep both structure trees, with every `/StructParents` key
//! distinct and every `/ParentTree` entry naming elements on its own page; a custom role mapped
//! two ways is renamed; undo takes the whole tree change back.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "tests: a panic is a test failure"
)]

use std::collections::BTreeSet;

use laminapdf::{
    BuildOptions, Builder, Document, InsertPagesFrom, OpenOptions, PagePosition, PageSize, PointPt, Pt, SaveOptions,
    Standard14, StructureName, Tag, TagKind,
};
use laminapdf_cos::{ObjRef, Object};
use laminapdf_model::{StructElem, StructKid};
use laminapdf_test_fixtures::fixture;

fn open(bytes: Vec<u8>) -> Document {
    Document::open_bytes(bytes, OpenOptions::default()).unwrap()
}

fn reopened(doc: &mut Document) -> Document {
    open(doc.save(&SaveOptions::new()).unwrap().bytes)
}

/// Inserts every page of `source` at the end of `doc`.
fn insert_all(doc: &mut Document, source: Vec<u8>) -> laminapdf::Edited {
    let id = doc.add_source(source, None).unwrap();
    let pages = doc.source_pages(&id).unwrap();
    doc.apply(vec![InsertPagesFrom::new(id, pages, PagePosition::End).into()])
        .unwrap()
}

/// How many elements the tree holds.
fn element_count(doc: &Document) -> usize {
    let ops = doc.ops();
    ops.structure().unwrap().map_or(0, |t| t.elements().len())
}

/// The pages each element's content is on: its `/Pg`, and the pages its marked-content and
/// object references name.
fn pages_of(elem: &StructElem) -> BTreeSet<laminapdf::PageId> {
    let mut out: BTreeSet<_> = elem.page.iter().cloned().collect();
    for kid in &elem.kids {
        match kid {
            StructKid::Mcid { page: Some(p), .. } | StructKid::Object { page: Some(p), .. } => {
                out.insert(p.clone());
            }
            _ => {}
        }
    }
    out
}

#[test]
fn inserted_tagged_pages_keep_both_trees_with_distinct_keys_on_their_own_pages() {
    let mut doc = open(fixture("golden/tagged-across-pages.pdf"));
    let target_elements = element_count(&doc);
    let source = open(fixture("golden/tagged-report.pdf"));
    // The source's Document element isn't copied: its children go under the target's.
    let source_elements = element_count(&source) - 1;
    let edited = insert_all(&mut doc, fixture("golden/tagged-report.pdf"));
    assert_eq!(edited.tags_dropped, []);
    let again = reopened(&mut doc);
    let ops = again.ops();
    let tree = ops.structure().unwrap().unwrap();
    assert_eq!(tree.elements().len(), target_elements + source_elements);
    // Every page's key and every annotation's key is used once.
    let mut keys = BTreeSet::new();
    let mut keyed_pages = Vec::new();
    for page in ops.pages().unwrap().iter() {
        let dict = &page.cos().dict;
        if let Some(k) = dict.get_int("StructParents") {
            assert!(keys.insert(k), "key {k} is shared");
            keyed_pages.push((k, page.id().clone()));
        }
        if let Some(Object::Array(annots)) = dict.get("Annots") {
            for a in annots.iter().filter_map(Object::as_ref) {
                let annot = ops.object(a.num).unwrap();
                if let Some(k) = annot.as_dict().and_then(|d| d.get_int("StructParent")) {
                    assert!(keys.insert(k), "key {k} is shared");
                }
            }
        }
    }
    // Each page's entry names elements that have content on that page.
    let entries: std::collections::BTreeMap<i64, Object> = ops
        .number_tree(tree.parent_tree.clone().unwrap())
        .entries()
        .unwrap()
        .into_iter()
        .collect();
    let by_object: std::collections::BTreeMap<ObjRef, &StructElem> = tree
        .elements()
        .into_iter()
        .filter_map(|e| e.object.map(|o| (o, e)))
        .collect();
    for (key, page) in keyed_pages {
        let owners = match ops.deref(&entries[&key]).unwrap().into_owned() {
            Object::Array(items) => items,
            other => panic!("page {page} has entry {other:?}"),
        };
        for owner in owners.iter().filter_map(Object::as_ref) {
            let elem = by_object
                .get(&owner)
                .unwrap_or_else(|| panic!("{owner:?} isn't in the tree"));
            assert!(pages_of(elem).contains(&page), "{owner:?} has no content on {page}");
        }
    }
}

/// A tagged one-page document of one paragraph tagged `tag`.
fn tagged_page(tag: Tag) -> Vec<u8> {
    let mut b = Builder::new(BuildOptions::new().with_tagged(true).with_id(*b"merge-struct-001"));
    let font = b.standard_font(Standard14::Helvetica).unwrap();
    let mut page = b.page(PageSize::A4);
    page.surface()
        .text("A note", &font, Pt::new(12.0), PointPt::from_points(72.0, 72.0))
        .unwrap()
        .tag(tag);
    page.finish().unwrap();
    b.finish().unwrap().save(&SaveOptions::new()).unwrap().bytes
}

#[test]
fn a_custom_role_mapped_another_way_comes_in_renamed() {
    let mut doc = open(tagged_page(Tag::custom("Remark", TagKind::P)));
    let edited = insert_all(&mut doc, tagged_page(Tag::custom("Remark", TagKind::H1)));
    let renames: Vec<(StructureName, &str, &str)> = edited
        .structure_renamed
        .iter()
        .map(|r| (r.kind, r.from.as_str(), r.to.as_str()))
        .collect();
    assert!(
        renames.contains(&(StructureName::Role, "Remark", "Remark#2")),
        "{renames:?}"
    );
    let again = reopened(&mut doc);
    let ops = again.ops();
    let tree = ops.structure().unwrap().unwrap();
    assert_eq!(tree.role_map.get(&b"Remark"[..]).map(Vec::as_slice), Some(&b"P"[..]));
    assert_eq!(tree.role_map.get(&b"Remark#2"[..]).map(Vec::as_slice), Some(&b"H1"[..]));
    let kinds: Vec<&[u8]> = tree.elements().iter().map(|e| e.kind.as_slice()).collect();
    assert!(kinds.contains(&&b"Remark#2"[..]), "{kinds:?}");
}

#[test]
fn an_element_id_already_used_comes_in_renamed() {
    // The builder names each Note element (`Note1`, …), so both documents use `Note1`.
    let mut doc = open(tagged_page(Tag::new(TagKind::Note)));
    let edited = insert_all(&mut doc, tagged_page(Tag::new(TagKind::Note)));
    let renames: Vec<(StructureName, &str, &str)> = edited
        .structure_renamed
        .iter()
        .map(|r| (r.kind, r.from.as_str(), r.to.as_str()))
        .collect();
    assert_eq!(renames, [(StructureName::Id, "Note1", "Note1#2")]);
    let again = reopened(&mut doc);
    let ops = again.ops();
    let tree = ops.structure().unwrap().unwrap();
    let ids: Vec<&[u8]> = tree.elements().iter().filter_map(|e| e.id.as_deref()).collect();
    assert_eq!(ids, [&b"Note1"[..], b"Note1#2"]);
    // The IDTree maps both.
    let entries = laminapdf_model::NameTree::new(&ops, tree.id_tree.clone().unwrap())
        .entries()
        .unwrap();
    let keys: Vec<&[u8]> = entries.iter().map(|(k, _)| k.as_slice()).collect();
    assert_eq!(keys, [&b"Note1"[..], b"Note1#2"]);
}

#[test]
fn undo_takes_the_tree_back() {
    let mut doc = open(fixture("golden/tagged-across-pages.pdf"));
    let before = element_count(&doc);
    insert_all(&mut doc, fixture("golden/tagged-report.pdf"));
    assert!(element_count(&doc) > before);
    doc.undo().unwrap();
    assert_eq!(element_count(&doc), before);
    assert!(!doc.ops().is_dirty());
}
