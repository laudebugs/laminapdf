//! `Document::annotations` over `fixtures/annotations/` (written by
//! `tools/fixtures/annotations.py`): every subtype's entries and geometry, Popup pairing, reply
//! and group links, flags, and duplicate names.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf_model::{Annot, AnnotGeometry, AnnotSubtype, Document, OpenOptions, ReplyType, UserSpacePoint, Warning};
use laminapdf_test_fixtures::fixture;

fn open(name: &str) -> Document {
    Document::open(fixture(&format!("annotations/{name}")), OpenOptions::default()).unwrap()
}

fn page_annots(doc: &Document, index: usize) -> Vec<Annot> {
    let pages = doc.pages().unwrap();
    doc.annotations(&pages.get(index).unwrap()).unwrap()
}

fn by_object(annots: &[Annot], num: u32) -> &Annot {
    annots
        .iter()
        .find(|a| a.id.as_lamina_id().obj_ref().is_some_and(|r| r.num == num))
        .unwrap()
}

fn id(text: &str) -> laminapdf_model::AnnotId {
    text.parse().unwrap()
}

#[test]
fn every_entry_is_read_in_the_order_of_annots() {
    let annots = page_annots(&open("every-subtype.pdf"), 0);
    let subtypes: Vec<AnnotSubtype> = annots.iter().map(|a| a.subtype.clone()).collect();
    assert_eq!(
        subtypes,
        [
            AnnotSubtype::Highlight,
            AnnotSubtype::Underline,
            AnnotSubtype::Text,
            AnnotSubtype::Popup,
            AnnotSubtype::Ink,
            AnnotSubtype::Line,
            AnnotSubtype::Polygon,
            AnnotSubtype::FreeText,
            AnnotSubtype::Widget,
            AnnotSubtype::Square,
        ]
    );
}

#[test]
fn a_highlight_reads_its_common_and_markup_entries() {
    let annots = page_annots(&open("every-subtype.pdf"), 0);
    let highlight = by_object(&annots, 10);
    assert_eq!(
        (
            highlight.name.as_deref(),
            highlight.color.as_slice(),
            highlight.opacity,
            highlight.author.as_deref(),
            highlight.subject.as_deref(),
            highlight.created.as_deref(),
            highlight.modified.as_deref(),
            highlight.flags.0,
            highlight.has_appearance,
        ),
        (
            Some("hl-1"),
            [1.0, 1.0, 0.0].as_slice(),
            Some(0.5),
            Some("Ada"),
            Some("Highlight"),
            Some("D:20261101090000Z"),
            Some("D:20261102090000Z"),
            4,
            true,
        )
    );
}

#[test]
fn quads_are_kept_in_the_order_stored() {
    let annots = page_annots(&open("every-subtype.pdf"), 0);
    let first_corner = |num| match &by_object(&annots, num).geometry {
        AnnotGeometry::Quads(quads) => quads.first().map(|q| q[0]),
        other => panic!("{other:?}"),
    };
    assert_eq!(
        (first_corner(10), first_corner(11)),
        (
            Some(UserSpacePoint::new(20.0, 260.0)),
            Some(UserSpacePoint::new(20.0, 255.0))
        )
    );
}

#[test]
fn each_subtype_reads_its_geometry() {
    let annots = page_annots(&open("every-subtype.pdf"), 0);
    let p = UserSpacePoint::new;
    assert_eq!(
        (
            by_object(&annots, 14).geometry.clone(),
            by_object(&annots, 15).geometry.clone(),
            by_object(&annots, 16).geometry.clone(),
            by_object(&annots, 17).geometry.clone(),
        ),
        (
            AnnotGeometry::Ink(vec![
                vec![p(20.0, 150.0), p(70.0, 200.0), p(120.0, 150.0)],
                vec![p(30.0, 160.0), p(40.0, 170.0)],
            ]),
            AnnotGeometry::Line([p(20.0, 100.0), p(120.0, 140.0)]),
            AnnotGeometry::Vertices(vec![p(150.0, 100.0), p(250.0, 100.0), p(200.0, 160.0)]),
            AnnotGeometry::Callout(vec![p(140.0, 90.0), p(160.0, 70.0), p(170.0, 50.0)]),
        )
    );
}

#[test]
fn a_note_and_its_popup_name_each_other() {
    let annots = page_annots(&open("every-subtype.pdf"), 0);
    assert_eq!(
        (
            by_object(&annots, 12).popup.clone(),
            by_object(&annots, 13).parent.clone()
        ),
        (Some(id("a13.0")), Some(id("a12.0")))
    );
}

#[test]
fn rich_text_is_noticed() {
    let annots = page_annots(&open("every-subtype.pdf"), 0);
    assert!(by_object(&annots, 17).has_rich_text);
}

#[test]
fn a_direct_annotation_gets_a_path_id() {
    let annots = page_annots(&open("every-subtype.pdf"), 0);
    let direct = annots.last().unwrap();
    assert_eq!((direct.id.as_lamina_id().is_path(), direct.index), (true, 9));
}

#[test]
fn a_popup_without_a_link_from_its_parent_is_paired_by_its_parent_entry() {
    let annots = page_annots(&open("threads.pdf"), 0);
    assert_eq!(by_object(&annots, 10).popup, Some(id("a11.0")));
}

#[test]
fn replies_and_groups_read_their_links() {
    let annots = page_annots(&open("threads.pdf"), 0);
    let link = |num| {
        let a = by_object(&annots, num);
        (a.in_reply_to.clone(), a.reply_type)
    };
    assert_eq!(
        [link(12), link(13), link(17)],
        [
            (Some(id("a10.0")), ReplyType::Reply),
            (Some(id("a12.0")), ReplyType::Reply),
            (Some(id("a16.0")), ReplyType::Group),
        ]
    );
}

#[test]
fn an_indirect_annots_array_is_read() {
    let annots = page_annots(&open("threads.pdf"), 1);
    assert_eq!(
        annots.iter().map(|a| a.in_reply_to.clone()).collect::<Vec<_>>(),
        [Some(id("a10.0"))]
    );
}

#[test]
fn two_annotations_with_one_name_keep_both_and_warn() {
    let doc = open("flags.pdf");
    let annots = page_annots(&doc, 0);
    let warned: Vec<Option<u32>> = doc
        .warnings()
        .iter()
        .filter_map(|w| match w {
            Warning::AnnotNameDuplicate { object } => Some(*object),
            _ => None,
        })
        .collect();
    assert_eq!((annots.len(), warned), (5, vec![Some(14)]));
}

/// pdf.js's `getAnnotations` (facts in `fixtures/annotations/pdfjs.json`, from
/// `tools/fixtures/pdfjs_annotations.mjs`) reports the same annotations on every page, by
/// subtype. Order isn't compared, since pdf.js lists widgets after the other annotations.
#[test]
fn pdfjs_reads_the_same_annotations_on_every_page() {
    let facts: serde_json::Value = serde_json::from_slice(&fixture("annotations/pdfjs.json")).unwrap();
    for (file, pages) in facts["files"].as_object().unwrap() {
        let doc = open(file);
        for (index, expected) in pages.as_array().unwrap().iter().enumerate() {
            let mut theirs: Vec<String> = expected
                .as_array()
                .unwrap()
                .iter()
                .map(|a| a["subtype"].as_str().unwrap().to_owned())
                .collect();
            let mut ours: Vec<String> = page_annots(&doc, index)
                .iter()
                .map(|a| String::from_utf8_lossy(a.subtype.name()).into_owned())
                .collect();
            theirs.sort();
            ours.sort();
            assert_eq!(ours, theirs, "{file} page {index}");
        }
    }
}
