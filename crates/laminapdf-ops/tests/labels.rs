//! `SetPageLabels` over `fixtures/pages/labels.pdf` (ten pages: i–iii, 1–4, A-1, A-2, then
//! "Back cover"): writing, validation, removal, undo and the wire form.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::{open, page_ids, snapshot};
use laminapdf_model::{LabelStyle, PageId};
use laminapdf_ops::{Document, Error, ModificationClass, Op, Origin, PageLabelRange, SetPageLabels, UndoOutcome};
use laminapdf_test_fixtures::fixture;

fn labelled() -> Document {
    open(&fixture("pages/labels.pdf"))
}

fn labels(doc: &Document) -> Vec<Option<String>> {
    let labels = doc.page_labels().unwrap();
    (0..doc.pages().unwrap().len()).map(|i| labels.label(i)).collect()
}

fn set(doc: &mut Document, ranges: Vec<PageLabelRange>) -> laminapdf_ops::Result<()> {
    doc.apply_all(Origin::Local, [Op::from(SetPageLabels::new(ranges))])
        .map(|_| ())
}

fn range(page: &PageId, style: Option<LabelStyle>) -> PageLabelRange {
    PageLabelRange::new(page.clone(), style)
}

fn invalid(result: laminapdf_ops::Result<()>) -> Option<&'static str> {
    match result {
        Err(Error::InvalidLabelRanges { what }) => Some(what),
        _ => None,
    }
}

#[test]
fn ranges_label_their_pages_in_order() {
    let mut doc = labelled();
    let pages = page_ids(&doc);
    set(
        &mut doc,
        vec![
            range(&pages[0], Some(LabelStyle::UpperRoman)),
            range(&pages[2], Some(LabelStyle::Decimal)).with_first(5),
            range(&pages[8], Some(LabelStyle::LowerLetters)).with_prefix("App. "),
        ],
    )
    .unwrap();
    let shown = labels(&doc);
    assert_eq!(
        shown.iter().map(|l| l.as_deref().unwrap_or("")).collect::<Vec<_>>(),
        ["I", "II", "5", "6", "7", "8", "9", "10", "App. a", "App. b"]
    );
}

#[test]
fn a_range_without_a_style_shows_its_prefix_alone() {
    let mut doc = labelled();
    let pages = page_ids(&doc);
    set(&mut doc, vec![range(&pages[0], None).with_prefix("Cover")]).unwrap();
    assert_eq!(labels(&doc)[3].as_deref(), Some("Cover"));
}

#[test]
fn no_ranges_remove_the_labels() {
    let mut doc = labelled();
    set(&mut doc, Vec::new()).unwrap();
    assert_eq!(
        (doc.catalog().unwrap().get("PageLabels").is_none(), labels(&doc)[0].clone()),
        (true, None)
    );
}

#[test]
fn the_labels_the_document_already_shows_write_nothing() {
    let mut doc = labelled();
    let pages = page_ids(&doc);
    set(
        &mut doc,
        vec![
            range(&pages[0], Some(LabelStyle::LowerRoman)),
            range(&pages[3], Some(LabelStyle::Decimal)),
            range(&pages[7], Some(LabelStyle::Decimal)).with_prefix("A-"),
            range(&pages[9], None).with_prefix("Back cover"),
        ],
    )
    .unwrap();
    assert!(doc.overlay().is_empty());
}

#[test]
fn the_first_range_must_be_on_the_first_page() {
    let mut doc = labelled();
    let pages = page_ids(&doc);
    assert_eq!(
        invalid(set(&mut doc, vec![range(&pages[1], Some(LabelStyle::Decimal))])),
        Some("first-page")
    );
}

#[test]
fn ranges_out_of_page_order_are_refused() {
    let mut doc = labelled();
    let pages = page_ids(&doc);
    let ranges = vec![
        range(&pages[0], Some(LabelStyle::LowerRoman)),
        range(&pages[5], Some(LabelStyle::Decimal)),
        range(&pages[3], Some(LabelStyle::Decimal)),
    ];
    assert_eq!(invalid(set(&mut doc, ranges)), Some("order"));
}

#[test]
fn two_ranges_on_one_page_are_refused() {
    let mut doc = labelled();
    let pages = page_ids(&doc);
    let ranges = vec![
        range(&pages[0], Some(LabelStyle::LowerRoman)),
        range(&pages[0], Some(LabelStyle::Decimal)),
    ];
    assert_eq!(invalid(set(&mut doc, ranges)), Some("order"));
}

#[test]
fn numbering_from_zero_is_refused() {
    let mut doc = labelled();
    let pages = page_ids(&doc);
    let ranges = vec![range(&pages[0], Some(LabelStyle::Decimal)).with_first(0)];
    assert_eq!(invalid(set(&mut doc, ranges)), Some("first-number"));
}

#[test]
fn a_page_of_another_document_is_unknown() {
    let mut doc = labelled();
    let stranger: PageId = "p999.0".parse().unwrap();
    assert!(matches!(
        set(&mut doc, vec![range(&stranger, Some(LabelStyle::Decimal))]),
        Err(Error::UnknownPage(_))
    ));
}

#[test]
fn a_refused_op_changes_nothing() {
    let mut doc = labelled();
    let pages = page_ids(&doc);
    let before = snapshot(&doc);
    let _refused = set(&mut doc, vec![range(&pages[2], Some(LabelStyle::Decimal))]);
    assert_eq!(snapshot(&doc), before);
}

#[test]
fn undo_restores_the_labels_and_leaves_the_overlay_empty() {
    let mut doc = labelled();
    let pages = page_ids(&doc);
    let before = labels(&doc);
    set(&mut doc, vec![range(&pages[0], Some(LabelStyle::Decimal))]).unwrap();
    let outcome = doc.undo().unwrap().unwrap().outcome;
    assert_eq!(
        (outcome, labels(&doc), doc.overlay().is_empty()),
        (UndoOutcome::Applied, before, true)
    );
}

#[test]
fn redo_writes_the_same_entries_again() {
    let mut doc = labelled();
    let pages = page_ids(&doc);
    set(&mut doc, vec![range(&pages[0], Some(LabelStyle::UpperLetters))]).unwrap();
    let applied = snapshot(&doc);
    doc.undo().unwrap();
    doc.redo().unwrap();
    assert_eq!(snapshot(&doc), applied);
}

#[test]
fn labels_are_a_document_change_that_needs_no_rewrite() {
    let op = Op::from(SetPageLabels::new(Vec::new()));
    assert_eq!((op.class(), op.requires_rewrite()), (ModificationClass::Document, false));
}

/// The frozen v1 sample (`fixtures/ops/v1/set-page-labels.json`) reads and writes back the
/// same, defaults left out.
#[test]
fn the_v1_sample_round_trips() {
    let text = String::from_utf8(fixture("ops/v1/set-page-labels.json")).unwrap();
    let op: Op = serde_json::from_str(&text).unwrap();
    let again: serde_json::Value = serde_json::to_value(&op).unwrap();
    let frozen: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(again, frozen);
}

#[test]
fn an_unknown_style_is_refused_on_the_wire() {
    let json = r#"{ "op": "setPageLabels", "args": { "ranges": [{ "page": "p3.0", "style": "hex" }] } }"#;
    assert!(serde_json::from_str::<Op>(json).is_err());
}
