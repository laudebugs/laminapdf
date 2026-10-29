//! ADR 0005 §5: `LaminaId`s for pages and outline items survive reopen and an incremental
//! save; a moved page keeps its `PageId`; a direct page dictionary gets a path ID that is
//! stable for the same bytes.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

mod common;

use common::{classic, corpus, fixture, open};
use lamina_cos::{ObjRef, Object};
use lamina_save::{XrefEol, incremental_update};
use lamina_model::{Document, Kind, LaminaId};

/// Every page and outline-item ID of `doc`, in document order.
fn ids(doc: &Document) -> Vec<String> {
    let mut out: Vec<String> = doc.pages().unwrap().iter().map(|p| p.id().to_string()).collect();
    out.extend(doc.outline().unwrap().iter().map(|i| i.id().unwrap().to_string()));
    out
}

#[test]
fn ids_are_identical_after_reopen() {
    for (name, bytes) in corpus() {
        assert_eq!(ids(&open(&bytes).unwrap()), ids(&open(&bytes).unwrap()), "{name}");
    }
}

#[test]
fn ids_are_identical_after_an_incremental_no_op_save() {
    let bytes = fixture("corpus/outline.pdf");
    let doc = open(&bytes).unwrap();
    let saved = incremental_update(doc.cos(), &[], XrefEol::CrLf).unwrap();
    assert_eq!(ids(&open(&saved).unwrap()), ids(&doc));
}

#[test]
fn ids_are_identical_after_rewriting_the_catalog_incrementally() {
    let bytes = fixture("corpus/outline.pdf");
    let doc = open(&bytes).unwrap();
    let catalog = Object::Dict(doc.catalog().unwrap());
    let saved = incremental_update(doc.cos(), &[(ObjRef::new(1, 0), catalog)], XrefEol::CrLf).unwrap();
    let reopened = open(&saved).unwrap();
    assert_eq!((reopened.revisions(), ids(&reopened)), (2, ids(&doc)));
}

#[test]
fn a_moved_page_keeps_its_id() {
    let bytes = fixture("corpus/outline.pdf");
    let doc = open(&bytes).unwrap();
    let before: Vec<String> = doc.pages().unwrap().iter().map(|p| p.id().to_string()).collect();
    // Move the last page to the front: only the page tree changes, as a page op would.
    let mut pages = doc.resolve(ObjRef::new(2, 0)).unwrap().as_dict().unwrap().clone();
    pages.insert(
        "Kids",
        vec![
            ObjRef::new(6, 0).into(),
            ObjRef::new(4, 0).into(),
            ObjRef::new(5, 0).into(),
        ],
    );
    let moved = incremental_update(doc.cos(), &[(ObjRef::new(2, 0), pages.into())], XrefEol::CrLf).unwrap();
    let after = open(&moved).unwrap();
    let after_pages = after.pages().unwrap();
    let order: Vec<String> = after_pages.iter().map(|p| p.id().to_string()).collect();
    assert_eq!(order, [before[2].clone(), before[0].clone(), before[1].clone()]);
    let moved_page = after_pages.by_id(&before[2].parse().unwrap()).unwrap();
    assert_eq!(
        doc_text(&after, &moved_page),
        doc_text(&doc, &doc.pages().unwrap().get(2).unwrap())
    );
}

fn doc_text(doc: &Document, page: &lamina_model::Page<'_>) -> Vec<u8> {
    doc.page_content(page.cos()).unwrap()
}

/// Pages: object 2 holds a direct page dictionary as its second kid.
fn with_direct_page() -> Vec<u8> {
    classic(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R << /Type /Page /MediaBox [0 0 100 100] >>] /Count 2 >>",
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>",
        ],
        "",
    )
}

#[test]
fn a_direct_page_gets_a_path_id() {
    let doc = open(&with_direct_page()).unwrap();
    let ids: Vec<String> = doc.pages().unwrap().iter().map(|p| p.id().to_string()).collect();
    assert_eq!(ids, ["p3.0", "p2.0/Kids/1"]);
}

#[test]
fn a_path_id_is_stable_for_the_same_bytes() {
    let bytes = with_direct_page();
    assert_eq!(ids(&open(&bytes).unwrap()), ids(&open(&bytes).unwrap()));
}

#[test]
fn a_nested_direct_node_extends_the_path() {
    let doc = open(&classic(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [<< /Type /Pages /Kids [3 0 R << /Type /Page >>] >>] /Count 2 >>",
            "<< /Type /Page >>",
        ],
        "",
    ))
    .unwrap();
    let ids: Vec<String> = doc.pages().unwrap().iter().map(|p| p.id().to_string()).collect();
    assert_eq!(ids, ["p3.0", "p2.0/Kids/0/Kids/1"]);
}

#[test]
fn a_direct_page_tree_root_is_anchored_at_the_catalog() {
    let doc = open(&classic(
        &["<< /Type /Catalog /Pages << /Type /Pages /Kids [<< /Type /Page >>] /Count 1 >> >>"],
        "",
    ))
    .unwrap();
    let ids: Vec<String> = doc.pages().unwrap().iter().map(|p| p.id().to_string()).collect();
    assert_eq!(ids, ["p1.0/Pages/Kids/0"]);
}

#[test]
fn every_id_parses_back_to_itself() {
    for (name, bytes) in corpus() {
        for id in ids(&open(&bytes).unwrap()) {
            assert_eq!(id.parse::<LaminaId>().unwrap().to_string(), id, "{name}");
        }
    }
}

#[test]
fn page_and_outline_ids_are_unique_document_wide() {
    let doc = open(&fixture("corpus/outline.pdf")).unwrap();
    let all = ids(&doc);
    let unique: std::collections::BTreeSet<_> = all.iter().collect();
    assert_eq!(unique.len(), all.len());
}

#[test]
fn allocated_ids_never_collide_with_existing_objects() {
    let doc = open(&fixture("corpus/outline.pdf")).unwrap();
    let existing: Vec<u32> = doc.object_numbers().collect();
    let (r, _) = doc.ids().allocate(Kind::Annot).unwrap();
    assert!(doc.ids().is_allocated(&LaminaId::object(Kind::Annot, r)));
    assert!(existing.iter().all(|&n| n < r.num));
}

#[test]
fn existing_ids_are_not_allocated_however_often_they_are_read() {
    let doc = open(&fixture("corpus/outline.pdf")).unwrap();
    for _ in 0..3 {
        let pages = doc.pages().unwrap();
        assert!(!doc.ids().is_allocated(pages.get(0).unwrap().id().as_lamina_id()));
    }
}

#[test]
fn new_numbers_start_above_the_trailer_size() {
    let bytes = String::from_utf8(classic(
        &["<< /Type /Catalog /Pages 2 0 R >>", "<< /Type /Pages /Kids [] >>"],
        "",
    ))
    .unwrap()
    .replace("/Size 3", "/Size 50");
    let doc = open(bytes.as_bytes()).unwrap();
    assert_eq!(doc.ids().allocate(Kind::Annot).unwrap().0.num, 50);
}

#[test]
fn allocation_stops_at_the_documents_max_object_number() {
    let bytes = classic(
        &["<< /Type /Catalog /Pages 2 0 R >>", "<< /Type /Pages /Kids [] >>"],
        "",
    );
    let limits = lamina_model::Limits::default().with_max_object_number(3);
    let doc = Document::open(bytes, lamina_model::OpenOptions::default().with_limits(limits)).unwrap();
    assert!(doc.ids().allocate(Kind::Annot).is_ok() && doc.ids().allocate(Kind::Annot).is_err());
}

#[test]
fn a_direct_kid_of_an_indirect_kids_array_is_anchored_at_the_array() {
    let doc = open(&classic(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids 3 0 R /Count 1 >>",
            "[<< /Type /Page >>]",
        ],
        "",
    ))
    .unwrap();
    let ids: Vec<String> = doc.pages().unwrap().iter().map(|p| p.id().to_string()).collect();
    assert_eq!(ids, ["p3.0/0"]);
}

#[test]
fn outline_items_under_a_direct_root_are_anchored_at_the_trailer() {
    let bytes = String::from_utf8(classic(&["<< /Type /Pages /Kids [] /Count 0 >>"], ""))
        .unwrap()
        .replace(
            "/Root 1 0 R",
            "/Root << /Type /Catalog /Pages 1 0 R /Outlines << /First << /Title (a) >> >> >>",
        );
    let doc = open(bytes.as_bytes()).unwrap();
    let ids: Vec<String> = doc
        .outline()
        .unwrap()
        .iter()
        .map(|i| i.id().unwrap().to_string())
        .collect();
    assert_eq!(ids, ["ot/Root/Outlines/First"]);
}
