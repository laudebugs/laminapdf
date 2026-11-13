//! `Document::page_annotations` in page space, and what `apply` reports for the annotation ops.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf::{AnnotationGeometry, DeleteAnnot, Document, Op, OpenOptions, PointPt, Pt, RectPt};
use laminapdf_test_fixtures::fixture;

fn open(name: &str) -> Document {
    Document::open_bytes(fixture(&format!("annotations/{name}")), OpenOptions::new()).unwrap()
}

#[test]
fn rectangles_and_quads_are_in_page_space() {
    let annots = open("every-subtype.pdf").page_annotations(0).unwrap();
    let highlight = annots.iter().find(|a| a.id == "a10.0").unwrap();
    let at = PointPt::from_points;
    assert_eq!(
        (highlight.rect, &highlight.geometry),
        (
            Some(RectPt::new(Pt::new(20.0), Pt::new(25.0), Pt::new(120.0), Pt::new(40.0))),
            &AnnotationGeometry::Quads(vec![[at(20.0, 40.0), at(120.0, 40.0), at(120.0, 25.0), at(20.0, 25.0)]])
        )
    );
}

#[test]
fn the_records_name_their_links_by_id() {
    let annots = open("threads.pdf").page_annotations(0).unwrap();
    let note = annots.iter().find(|a| a.id == "a10.0").unwrap();
    let caret = annots.iter().find(|a| a.id == "a17.0").unwrap();
    assert_eq!(
        (note.popup.as_deref(), caret.in_reply_to.as_deref(), caret.grouped),
        (Some("a11.0"), Some("a16.0"), true)
    );
}

#[test]
fn a_page_out_of_range_is_refused() {
    assert_eq!(
        open("flags.pdf").page_annotations(3).unwrap_err().code(),
        "page-index-out-of-range"
    );
}

#[test]
fn a_delete_reports_what_went_with_it() {
    let mut doc = open("threads.pdf");
    let edited = doc
        .apply(vec![Op::from(DeleteAnnot::new("a16.0".parse().unwrap()))])
        .unwrap();
    assert_eq!(
        (edited.also_deleted, edited.still_in_earlier_revisions, edited.changed),
        (vec!["a17.0".to_owned()], true, true)
    );
}
