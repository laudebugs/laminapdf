//! `Document::page_references` on `fixtures/pages/references.pdf`: every reference to page 3,
//! and nothing that points elsewhere.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

use laminapdf_cos::{ObjRef, Object};
use laminapdf_model::{
    Document, NameStore, OpenOptions, OutlineItemId, PageId, PageReference, ReferenceKind, Via, WidgetLink,
};
use laminapdf_test_fixtures::fixture;

fn open() -> Document {
    Document::open(fixture("pages/references.pdf"), OpenOptions::default()).unwrap()
}

fn page_ids(doc: &Document) -> Vec<PageId> {
    doc.pages().unwrap().iter().map(|p| p.id().clone()).collect()
}

/// The annotation at `index` in the `/Annots` of the page at `page`.
fn annot(doc: &Document, page: usize, index: usize) -> ObjRef {
    doc.pages().unwrap().get(page).unwrap().annots().unwrap()[index]
}

#[test]
fn every_reference_to_page_three_is_found_and_nothing_else() {
    let doc = open();
    let ids = page_ids(&doc);
    let page3 = ids[2].clone();
    let refs = doc.page_references(std::slice::from_ref(&page3)).unwrap();
    let outline = doc.outline().unwrap();
    let items: Vec<OutlineItemId> = outline.iter().map(|i| i.id().unwrap().clone()).collect();
    let goto = doc
        .resolve(annot(&doc, 1, 0))
        .unwrap()
        .as_dict()
        .unwrap()
        .get("A")
        .and_then(Object::as_ref);
    let city = doc
        .resolve(annot(&doc, 2, 1))
        .unwrap()
        .as_dict()
        .unwrap()
        .get("Parent")
        .and_then(Object::as_ref)
        .unwrap();
    let structure = doc.structure().unwrap().unwrap();
    let para3 = structure
        .elements()
        .iter()
        .find(|e| e.page.as_ref() == Some(&page3))
        .unwrap()
        .object;
    let kinds = [
        ReferenceKind::OutlineItem {
            item: items[0].clone(),
            via: Via::Dest { name: None },
        },
        ReferenceKind::OutlineItem {
            item: items[1].clone(),
            via: Via::GoTo {
                action: None,
                name: None,
            },
        },
        ReferenceKind::Link {
            annot: annot(&doc, 0, 0),
            on: ids[0].clone(),
            via: Via::Dest { name: None },
        },
        ReferenceKind::Link {
            annot: annot(&doc, 1, 0),
            on: ids[1].clone(),
            via: Via::GoTo {
                action: goto,
                name: None,
            },
        },
        ReferenceKind::Link {
            annot: annot(&doc, 3, 0),
            on: ids[3].clone(),
            via: Via::Dest {
                name: Some(b"chap3".to_vec()),
            },
        },
        ReferenceKind::NamedDestination {
            name: b"chap3".to_vec(),
            store: NameStore::NameTree,
        },
        ReferenceKind::NamedDestination {
            name: b"old3".to_vec(),
            store: NameStore::DestsDict,
        },
        ReferenceKind::OpenAction {
            via: Via::Dest { name: None },
        },
        ReferenceKind::StructElem { elem: para3 },
        ReferenceKind::Widget {
            widget: annot(&doc, 2, 0),
            field: annot(&doc, 2, 0),
            by: WidgetLink::P,
        },
        ReferenceKind::Widget {
            widget: annot(&doc, 2, 1),
            field: city,
            by: WidgetLink::Annots,
        },
    ];
    let expected: Vec<PageReference> = kinds
        .into_iter()
        .map(|kind| {
            refs.entries()
                .iter()
                .find(|e| e.kind == kind)
                .cloned()
                .unwrap_or_else(|| panic!("missing {kind:?}"))
        })
        .collect();
    assert_eq!(refs.entries(), expected.as_slice());
    assert!(refs.entries().iter().all(|e| e.page == page3));
    assert_eq!(refs.to_page(&page3).count(), 11);
}

#[test]
fn references_to_other_pages_are_found_for_those_pages() {
    let doc = open();
    let ids = page_ids(&doc);
    let refs = doc.page_references(&ids[3..5]).unwrap();
    // Page 4: the name tree's chap4, the zip widget; page 5: the /Dests dictionary's old5.
    let names: Vec<(&PageId, &ReferenceKind)> = refs.entries().iter().map(|e| (&e.page, &e.kind)).collect();
    assert_eq!(names.len(), 3, "{names:?}");
    assert!(
        refs.to_page(&ids[3])
            .any(|e| matches!(&e.kind, ReferenceKind::Widget { by: WidgetLink::P, .. }))
    );
    assert!(refs.to_page(&ids[4]).any(|e| e.kind
        == ReferenceKind::NamedDestination {
            name: b"old5".to_vec(),
            store: NameStore::DestsDict,
        }));
}

#[test]
fn a_page_nothing_points_at_has_no_references() {
    let doc = open();
    let ids = page_ids(&doc);
    // Page 1 has a link on it but nothing points at it.
    assert!(doc.page_references(&ids[..1]).unwrap().is_empty());
}
