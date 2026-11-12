//! `Document::page_references` on `fixtures/pages/references.pdf`: every reference to page 3,
//! and nothing that points elsewhere, `MCR`/`OBJR` kids with their own `/Pg` and fields whose
//! kids mix widgets and fields among them.
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

/// The `/Parent` of object `r`.
fn parent(doc: &Document, r: ObjRef) -> ObjRef {
    doc.resolve(r)
        .unwrap()
        .as_dict()
        .unwrap()
        .get("Parent")
        .and_then(Object::as_ref)
        .unwrap()
}

/// The annotation at `index` in the `/Annots` of the page at `page`.
fn annot(doc: &Document, page: usize, index: usize) -> ObjRef {
    doc.pages().unwrap().get(page).unwrap().annots().unwrap()[index]
}

/// The outline, link, named-destination and `/OpenAction` references to page 3, in order.
fn navigation_kinds(doc: &Document, ids: &[PageId]) -> Vec<ReferenceKind> {
    let outline = doc.outline().unwrap();
    let items: Vec<OutlineItemId> = outline.iter().map(|i| i.id().unwrap().clone()).collect();
    let goto = doc
        .resolve(annot(doc, 1, 0))
        .unwrap()
        .as_dict()
        .unwrap()
        .get("A")
        .and_then(Object::as_ref);
    vec![
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
            annot: annot(doc, 0, 0),
            on: ids[0].clone(),
            via: Via::Dest { name: None },
        },
        ReferenceKind::Link {
            annot: annot(doc, 1, 0),
            on: ids[1].clone(),
            via: Via::GoTo {
                action: goto,
                name: None,
            },
        },
        ReferenceKind::Link {
            annot: annot(doc, 3, 0),
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
    ]
}

/// The structure and widget references to page 3, in order.
fn structure_and_form_kinds(doc: &Document, ids: &[PageId]) -> Vec<ReferenceKind> {
    let page3 = &ids[2];
    let city = parent(doc, annot(doc, 2, 1));
    let structure = doc.structure().unwrap().unwrap();
    let elem_on = |page: &PageId| {
        structure
            .elements()
            .iter()
            .find(|e| e.page.as_ref() == Some(page))
            .unwrap()
            .object
    };
    let (para2, para3) = (elem_on(&ids[1]), elem_on(page3));
    let k = doc.resolve(para2.unwrap()).unwrap();
    let objr = k.as_dict().unwrap().get("K").unwrap().as_array().unwrap()[2]
        .as_ref()
        .unwrap();
    vec![
        ReferenceKind::StructElem { elem: para3 },
        ReferenceKind::StructContentRef {
            elem: para2,
            index: 1,
            kid: None,
        },
        ReferenceKind::StructContentRef {
            elem: para2,
            index: 2,
            kid: Some(objr),
        },
        ReferenceKind::Widget {
            widget: annot(doc, 2, 0),
            field: annot(doc, 2, 0),
            by: WidgetLink::P,
        },
        ReferenceKind::Widget {
            widget: annot(doc, 2, 1),
            field: city,
            by: WidgetLink::Annots,
        },
        // A widget kid beside a field kid belongs to its own field, and a kid without /T that
        // holds a widget is a field.
        ReferenceKind::Widget {
            widget: annot(doc, 2, 2),
            field: parent(doc, annot(doc, 2, 2)),
            by: WidgetLink::P,
        },
        ReferenceKind::Widget {
            widget: annot(doc, 2, 3),
            field: parent(doc, annot(doc, 2, 3)),
            by: WidgetLink::P,
        },
    ]
}

#[test]
fn every_reference_to_page_three_is_found_and_nothing_else() {
    let doc = open();
    let ids = page_ids(&doc);
    let page3 = ids[2].clone();
    let refs = doc.page_references(std::slice::from_ref(&page3)).unwrap();
    let mut kinds = navigation_kinds(&doc, &ids);
    kinds.extend(structure_and_form_kinds(&doc, &ids));
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
    assert_eq!(refs.to_page(&page3).count(), 15);
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
