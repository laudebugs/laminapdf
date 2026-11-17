//! The structure tree kept in step with content edits (ADR 0022 §4), the conformance warnings
//! content edits raise, and `RemoveConformanceClaims`.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{classic, fixture, open, stream_body};
use laminapdf_content::{ContentOps, ContentOwner, interpret_page};
use laminapdf_cos::{ObjRef, Object};
use laminapdf_model::PageId;
use laminapdf_model::xmp::PacketState;
use laminapdf_ops::{
    ConformanceWarning, ContentEdit, Document, EditContent, EditText, ObjectRef, Op, Origin, RemoveConformanceClaims,
    pending_conformance_warnings, text_runs,
};

/// Two tagged paragraphs and a tagged image, each in marked content of its own.
const CONTENT: &[u8] = b"/P <</MCID 0>> BDC BT /F1 12 Tf 20 150 Td (Hello) Tj ET EMC\n\
/P <</MCID 1>> BDC BT /F1 12 Tf 20 120 Td (World) Tj ET EMC\n\
/Figure <</MCID 2>> BDC q 40 0 0 40 100 20 cm /Im1 Do Q EMC\n";

/// The objects: 1 catalog, 2 pages, 3 page, 4 content, 5 font, 6 image, 7 root, 8 Document,
/// 9 and 10 the paragraphs, 11 the figure, 12 the parent tree, 13 the XMP packet claiming `claim`.
fn tagged(claim: &str) -> Document {
    build(claim, b"<< /Nums [0 [9 0 R 10 0 R 11 0 R]] >>")
}

/// [`tagged`] with `parent_tree` as object 12.
fn build(claim: &str, parent_tree: &[u8]) -> Document {
    let content = stream_body("", CONTENT);
    let image = stream_body(
        "/Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceGray /BitsPerComponent 8",
        b"\x80",
    );
    let xmp = format!(
        r#"<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:pdfuaid="http://www.aiim.org/pdfua/ns/id/" xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/" {claim}><dc:title><rdf:Alt><rdf:li xml:lang="x-default">Tagged</rdf:li></rdf:Alt></dc:title></rdf:Description></rdf:RDF></x:xmpmeta>"#
    );
    let metadata = stream_body("/Type /Metadata /Subtype /XML", xmp.as_bytes());
    let objects: [&[u8]; 13] = [
        b"<< /Type /Catalog /Pages 2 0 R /StructTreeRoot 7 0 R /MarkInfo << /Marked true >> /Metadata 13 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R /StructParents 0 /Resources << /Font << /F1 5 0 R >> /XObject << /Im1 6 0 R >> >> >>",
        &content,
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        &image,
        b"<< /Type /StructTreeRoot /K [8 0 R] /ParentTree 12 0 R /ParentTreeNextKey 1 >>",
        b"<< /Type /StructElem /S /Document /P 7 0 R /K [9 0 R 10 0 R 11 0 R] >>",
        b"<< /Type /StructElem /S /P /P 8 0 R /Pg 3 0 R /K 0 /ActualText (Hello) >>",
        b"<< /Type /StructElem /S /P /P 8 0 R /Pg 3 0 R /K [1] >>",
        b"<< /Type /StructElem /S /Figure /P 8 0 R /Pg 3 0 R /K 2 /Alt (A grey square) >>",
        parent_tree,
        &metadata,
    ];
    open(&classic(&objects, "<< /Size 14 /Root 1 0 R >>")).with_images(laminapdf_build::ImageXObjectWriter)
}

fn page(doc: &Document) -> PageId {
    doc.pages().unwrap().get(0).unwrap().id().clone()
}

/// The page's object at `ordinal`, by its lasting name.
fn object(doc: &Document, ordinal: usize) -> ObjectRef {
    let pages = doc.pages().unwrap();
    let page = pages.get(0).unwrap();
    let mut ir = ContentOps::read_page(doc, page.cos(), ContentOwner::Page(page.id().clone())).unwrap();
    interpret_page(doc, &page, &mut ir.recorder(page.frame().unwrap())).unwrap();
    let id = ir.objects()[ordinal].id();
    ObjectRef::from(&ir.reference(id).unwrap())
}

fn dict(doc: &Document, num: u32) -> laminapdf_cos::Dict {
    doc.object(num).unwrap().as_dict().unwrap().clone()
}

/// The page's slots in the parent tree.
fn slots(doc: &Document) -> Vec<Object> {
    let root = dict(doc, 7);
    let entries = doc
        .number_tree(root.get("ParentTree").unwrap().clone())
        .entries()
        .unwrap();
    match entries.into_iter().find(|(k, _)| *k == 0) {
        Some((_, Object::Array(items))) => items,
        other => panic!("no array for the page: {other:?}"),
    }
}

fn kids(doc: &Document, num: u32) -> Vec<Object> {
    match dict(doc, num).get("K") {
        Some(Object::Array(items)) => items.clone(),
        Some(one) => vec![one.clone()],
        None => Vec::new(),
    }
}

fn apply(doc: &mut Document, op: impl Into<Op>) {
    doc.apply_all(Origin::Local, [op.into()]).unwrap();
}

fn warnings(doc: &Document) -> Vec<ConformanceWarning> {
    pending_conformance_warnings(doc, doc.pending_changes()).unwrap()
}

fn delete(doc: &mut Document, ordinal: usize) {
    let edit = ContentEdit::Delete {
        object: object(doc, ordinal),
    };
    let page = page(doc);
    apply(doc, EditContent::new(page, vec![edit]));
}

fn r(num: u32) -> Object {
    Object::Ref(ObjRef { num, gen_nr: 0 })
}

#[test]
fn deleting_a_tagged_object_should_take_its_element_out_of_the_tree() {
    let mut doc = tagged(r#"pdfuaid:part="1""#);
    delete(&mut doc, 1);
    assert_eq!(slots(&doc), [r(9), Object::Null, r(11)]);
    assert_eq!(kids(&doc, 8), [r(9), r(11)]);
    let content = doc.page_content(doc.pages().unwrap().get(0).unwrap().cos()).unwrap();
    assert!(!String::from_utf8_lossy(&content).contains("MCID 1"));
    assert_eq!(warnings(&doc), Vec::new());
}

#[test]
fn deleting_every_tagged_object_should_drop_the_pages_key() {
    let mut doc = tagged(r#"pdfuaid:part="1""#);
    for _ in 0..3 {
        delete(&mut doc, 0);
    }
    assert!(dict(&doc, 3).get("StructParents").is_none());
    assert_eq!(kids(&doc, 8), Vec::<Object>::new());
    assert_eq!(kids(&doc, 7), Vec::<Object>::new());
}

#[test]
fn deleting_a_tagged_run_should_take_its_mcid_out_of_the_tree() {
    let mut doc = tagged(r#"pdfuaid:part="1""#);
    let run = text_runs(&doc, &page(&doc))
        .unwrap()
        .into_iter()
        .find(|r| r.text == "World")
        .unwrap()
        .run;
    let op = EditText::new(page(&doc), run, String::new());
    apply(&mut doc, op);
    assert_eq!(slots(&doc), [r(9), Object::Null, r(11)]);
    assert_eq!(kids(&doc, 8), [r(9), r(11)]);
}

#[test]
fn rewriting_a_tagged_run_should_update_its_actual_text() {
    let mut doc = tagged(r#"pdfuaid:part="1""#);
    let run = text_runs(&doc, &page(&doc))
        .unwrap()
        .into_iter()
        .find(|r| r.text == "Hello")
        .unwrap()
        .run;
    let op = EditText::new(page(&doc), run, "Howdy".into());
    apply(&mut doc, op);
    let actual = dict(&doc, 9).get("ActualText").cloned();
    assert!(matches!(actual, Some(Object::String(s)) if s.bytes == b"Howdy"));
    assert_eq!(slots(&doc), [r(9), r(10), r(11)]);
}

#[test]
fn replacing_a_figures_image_should_ask_for_new_alternative_text() {
    let mut doc = tagged(r#"pdfuaid:part="1""#);
    let blob = doc.add_blob(fixture("images/photo-rgb.jpg"));
    let edit = ContentEdit::ReplaceImage {
        object: object(&doc, 2),
        blob,
    };
    let op = EditContent::new(page(&doc), vec![edit]);
    apply(&mut doc, op);
    assert!(dict(&doc, 11).get("Alt").is_none());
    assert_eq!(
        warnings(&doc),
        [ConformanceWarning::FigureWithoutAlt {
            pages: vec![page(&doc).to_string()]
        }]
    );
    doc.undo().unwrap();
    assert_eq!(warnings(&doc), Vec::new());
}

#[test]
fn tags_the_parent_tree_does_not_map_should_be_reported_as_broken() {
    // A parent tree with no slot for the image's MCID or the second paragraph's: deleting the
    // second paragraph flags the page, which still draws the untagged image.
    let mut doc = build(r#"pdfuaid:part="1""#, b"<< /Nums [0 [9 0 R]] >>");
    delete(&mut doc, 1);
    let codes: Vec<&str> = warnings(&doc).iter().map(ConformanceWarning::code).collect();
    assert_eq!(codes, ["content-tags-broken"]);
}

#[test]
fn removing_the_claims_should_clear_the_pdfa_and_pdfua_identification() {
    let mut doc = tagged(r#"pdfuaid:part="1" pdfaid:part="2" pdfaid:conformance="B""#);
    apply(&mut doc, RemoveConformanceClaims::new(false, true));
    let PacketState::Parsed(packet) = doc.xmp().unwrap() else {
        panic!()
    };
    assert_eq!((packet.pdfua_part(), packet.pdfa_part()), (None, Some(2)));
    apply(&mut doc, RemoveConformanceClaims::new(true, false));
    let PacketState::Parsed(packet) = doc.xmp().unwrap() else {
        panic!()
    };
    assert_eq!((packet.pdfa_part(), packet.pdfa_conformance()), (None, None));
}

#[test]
fn removing_no_claim_should_be_refused() {
    let mut doc = tagged("");
    let result = doc.apply_all(Origin::Local, [Op::from(RemoveConformanceClaims::new(false, false))]);
    assert_eq!(result.map(drop).map_err(|e| e.code()), Err("invalid-op"));
}

#[test]
fn removing_the_claim_should_silence_the_warnings_it_raised() {
    let mut doc = tagged(r#"pdfuaid:part="1""#);
    let blob = doc.add_blob(fixture("images/photo-rgb.jpg"));
    let edit = ContentEdit::ReplaceImage {
        object: object(&doc, 2),
        blob,
    };
    let op = EditContent::new(page(&doc), vec![edit]);
    apply(&mut doc, op);
    apply(&mut doc, RemoveConformanceClaims::new(false, true));
    assert_eq!(warnings(&doc), Vec::new());
}

#[test]
fn the_frozen_sample_removes_both_claims() {
    let op: Op = serde_json::from_slice(&fixture("ops/v1/remove-conformance-claims.json")).unwrap();
    assert_eq!(op, Op::from(RemoveConformanceClaims::new(true, true)));
    let mut doc = tagged(r#"pdfuaid:part="1" pdfaid:part="2" pdfaid:conformance="B""#);
    apply(&mut doc, op);
    let PacketState::Parsed(packet) = doc.xmp().unwrap() else {
        panic!()
    };
    assert_eq!((packet.pdfua_part(), packet.pdfa_part()), (None, None));
}
