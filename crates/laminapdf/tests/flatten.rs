//! `Document::flatten_annotations` over `fixtures/annotations/flatten.pdf` and the marked-up
//! report: the scopes, the report's warnings, the save plan's `flatten-untagged` on a PDF/UA-1
//! file, and one undo step.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf::{Document, FlattenIntent, FlattenScope, OpenOptions, Pt, RectPt, RichText, SaveOptions};
use laminapdf_test_fixtures::fixture;

fn open(name: &str) -> Document {
    Document::open_bytes(fixture(name), OpenOptions::new()).unwrap()
}

fn ids(doc: &Document, page: u32) -> Vec<String> {
    doc.page_annotations(page).unwrap().into_iter().map(|a| a.id).collect()
}

fn codes(warnings: &[laminapdf::Warning]) -> Vec<&'static str> {
    warnings.iter().map(laminapdf::Warning::code).collect()
}

#[test]
fn every_comment_is_flattened_and_what_cannot_be_is_reported() {
    let mut doc = open("annotations/flatten.pdf");
    let edited = doc
        .flatten_annotations(&FlattenScope::All, FlattenIntent::View)
        .unwrap();
    // Left on page 1: hidden, print-only, a layer that is off, an attachment with no appearance,
    // and the widget, which the scope leaves out.
    assert_eq!(
        (ids(&doc, 0), ids(&doc, 1), codes(&edited.warnings)),
        (
            ["a12.0", "a13.0", "a15.0", "a17.0", "a18.0"].map(String::from).to_vec(),
            Vec::new(),
            vec!["flatten-no-appearance"]
        )
    );
}

#[test]
fn a_page_scope_leaves_the_other_pages_alone() {
    let mut doc = open("annotations/flatten.pdf");
    doc.flatten_annotations(&FlattenScope::Pages(vec![1]), FlattenIntent::View)
        .unwrap();
    assert_eq!((ids(&doc, 0).len(), ids(&doc, 1).len()), (13, 0));
}

#[test]
fn a_page_past_the_end_is_refused() {
    let mut doc = open("annotations/flatten.pdf");
    let err = doc
        .flatten_annotations(&FlattenScope::Pages(vec![2]), FlattenIntent::View)
        .unwrap_err();
    assert_eq!(err.code(), "page-index-out-of-range");
}

#[test]
fn naming_a_widget_is_refused() {
    let mut doc = open("annotations/flatten.pdf");
    let scope = FlattenScope::Annotations(vec!["a18.0".parse().unwrap()]);
    let err = doc.flatten_annotations(&scope, FlattenIntent::View).unwrap_err();
    assert_eq!(err.code(), "flatten-widget");
}

#[test]
fn flattening_is_one_undo_step() {
    let mut doc = open("annotations/flatten.pdf");
    let before = ids(&doc, 0);
    doc.flatten_annotations(&FlattenScope::All, FlattenIntent::Print)
        .unwrap();
    doc.undo().unwrap();
    assert_eq!(ids(&doc, 0), before);
}

#[test]
fn a_pdf_ua_file_is_told_its_comments_left_the_structure_tree() {
    let mut doc = open("golden/examples/mark_up-pdfua.pdf");
    doc.flatten_annotations(&FlattenScope::All, FlattenIntent::Print)
        .unwrap();
    let plan = doc.plan(&SaveOptions::new()).unwrap();
    assert!(codes(&plan.conformance_warnings).contains(&"flatten-untagged"));
}

#[test]
fn undoing_a_flattening_takes_its_pdf_ua_warning_back() {
    let mut doc = open("golden/examples/mark_up-pdfua.pdf");
    doc.flatten_annotations(&FlattenScope::All, FlattenIntent::Print)
        .unwrap();
    doc.undo().unwrap();
    let plan = doc.plan(&SaveOptions::new()).unwrap();
    assert!(!codes(&plan.conformance_warnings).contains(&"flatten-untagged"));
}

#[test]
fn a_pdf_a_file_is_told_a_flattened_box_brought_a_font_that_isnt_embedded() {
    let mut doc = open("golden/examples/pdf_a_archive-2b.pdf");
    let area = RectPt::new(Pt::new(72.0), Pt::new(72.0), Pt::new(272.0), Pt::new(122.0));
    let op = doc.text_box_at(0, area, RichText::plain("Checked")).unwrap();
    doc.apply(vec![op.into()]).unwrap();
    doc.flatten_annotations(&FlattenScope::All, FlattenIntent::Print)
        .unwrap();
    let plan = doc.plan(&SaveOptions::new()).unwrap();
    assert!(codes(&plan.conformance_warnings).contains(&"pdfa-font-not-embedded"));
}

#[test]
fn a_comment_whose_reply_is_locked_is_left_out_of_the_whole_document() {
    let mut doc = open("annotations/flatten-cases.pdf");
    doc.flatten_annotations(&FlattenScope::All, FlattenIntent::View)
        .unwrap();
    // Left: the square a membership hides, the locked one, and the square whose reply is locked.
    assert_eq!(
        ids(&doc, 0),
        ["a11.0", "a14.0", "a18.0", "a19.0"].map(String::from).to_vec()
    );
}
