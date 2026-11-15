//! What the page-setup dialogs read: a page's boxes in user space, and the label ranges in the
//! shape the labels op takes back.
#![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf::{Document, LabelStyle, OpenOptions, SetPageLabels, UserSpaceRect};
use laminapdf_test_fixtures::fixture;

fn open(name: &str) -> Document {
    Document::open_bytes(fixture(name), OpenOptions::new()).unwrap()
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> UserSpaceRect {
    UserSpaceRect::from_corners(x0, y0, x1, y1)
}

#[test]
fn page_boxes_reads_each_box_and_whether_the_page_sets_it() {
    let doc = open("pages/boxes.pdf");
    let first = doc.page_boxes(0).unwrap();
    let second = doc.page_boxes(1).unwrap();
    assert_eq!(
        (
            first.media,
            first.crop,
            first.bleed,
            first.trim,
            first.art,
            first.own.bleed && first.own.trim && first.own.art,
            first.user_unit,
            second.user_unit,
        ),
        (
            rect(0.0, 0.0, 612.0, 792.0),
            rect(18.0, 18.0, 594.0, 774.0),
            rect(27.0, 27.0, 585.0, 765.0),
            rect(36.0, 36.0, 576.0, 756.0),
            rect(54.0, 54.0, 558.0, 738.0),
            true,
            1.0,
            2.0,
        )
    );
}

#[test]
fn page_boxes_defaults_unset_boxes_to_the_crop_box() {
    let doc = open("pages/labels.pdf");
    let boxes = doc.page_boxes(0).unwrap();
    assert_eq!(
        (boxes.own.bleed, boxes.own.trim, boxes.own.art, boxes.bleed, boxes.art),
        (false, false, false, boxes.crop, boxes.crop)
    );
}

#[test]
fn page_boxes_refuses_a_page_past_the_end() {
    let doc = open("pages/boxes.pdf");
    assert_eq!(doc.page_boxes(4).unwrap_err().code(), "page-index-out-of-range");
}

#[test]
fn label_ranges_name_each_range_by_its_first_page() {
    let doc = open("pages/set-labels/mixed.pdf");
    let ids: Vec<_> = doc.page_infos().unwrap().into_iter().map(|p| p.id).collect();
    let ranges = doc.label_ranges().unwrap();
    let read: Vec<_> = ranges
        .iter()
        .map(|r| {
            (
                ids.iter().position(|id| *id == r.page),
                r.style,
                r.prefix.as_str(),
                r.first,
            )
        })
        .collect();
    assert_eq!(
        read,
        [
            (Some(0), Some(LabelStyle::LowerRoman), "", 1),
            (Some(2), Some(LabelStyle::Decimal), "", 5),
            (Some(8), Some(LabelStyle::LowerLetters), "App. ", 1),
            (Some(9), None, "Back cover", 1),
        ]
    );
}

#[test]
fn label_ranges_written_back_change_nothing() {
    for name in ["alpha", "mixed", "prefixed", "restarts", "roman"] {
        let mut doc = open(&format!("pages/set-labels/{name}.pdf"));
        let before: Vec<_> = doc.page_infos().unwrap().into_iter().map(|p| p.label).collect();
        let ranges = doc.label_ranges().unwrap();
        let edited = doc.apply(vec![SetPageLabels::new(ranges).into()]).unwrap();
        let after: Vec<_> = doc.page_infos().unwrap().into_iter().map(|p| p.label).collect();
        assert_eq!((edited.changed, after), (false, before), "{name}");
    }
}

#[test]
fn a_document_without_labels_has_no_ranges() {
    assert_eq!(open("pages/boxes.pdf").label_ranges().unwrap(), []);
}
