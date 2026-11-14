//! `SetPageBoxes` and `CropPages` over `fixtures/pages/boxes.pdf` (four pages, one per rotation,
//! all five boxes, `/UserUnit 2` on the second): what each writes, what it refuses, the pages
//! whose content a crop hides, the widgets it reports and undo.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{fixture, open, page_ids, snapshot};
use laminapdf_model::{PageFrame, PointPt, Pt, UserSpaceRect};
use laminapdf_ops::{CropPages, Document, Error, Margins, Op, Origin, PageBoxes, SetPageBoxes, WidgetChange};

fn pages(name: &str) -> Document {
    open(&fixture(&format!("pages/{name}")))
}

fn apply(doc: &mut Document, op: impl Into<Op>) -> Result<laminapdf_ops::Applied, Error> {
    doc.apply_all(Origin::Local, [op.into()])
}

fn frames(doc: &Document) -> Vec<PageFrame> {
    doc.pages().unwrap().iter().map(|p| p.frame().unwrap()).collect()
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> UserSpaceRect {
    UserSpaceRect::from_corners(x0, y0, x1, y1)
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn one_set_of_margins_crops_every_rotation_by_the_same_visible_amount() {
    let mut doc = pages("boxes.pdf");
    let ids = page_ids(&doc);
    let before = frames(&doc);
    let margins = Margins::new(10.0, 20.0, 30.0, 40.0);
    let applied = apply(&mut doc, CropPages::new(ids.clone(), margins)).unwrap();
    let after = frames(&doc);
    for (i, (old, new)) in before.iter().zip(&after).enumerate() {
        // The page shows 60 points less across and 40 less down, whatever its rotation or unit.
        assert!(close(new.width().get(), old.width().get() - 60.0), "page {i} width");
        assert!(close(new.height().get(), old.height().get() - 40.0), "page {i} height");
        // Its new top-left corner is the point that showed 40 in from the left, 10 down.
        let corner = new.to_user_space(PointPt::new(Pt::ZERO, Pt::ZERO));
        let expected = old.to_user_space(PointPt::new(Pt::new(40.0), Pt::new(10.0)));
        assert!(
            close(corner.x, expected.x) && close(corner.y, expected.y),
            "page {i} corner"
        );
    }
    assert_eq!((applied.touched_pages.len(), applied.cropped_content_remains), (4, ids));
}

#[test]
fn margins_that_leave_nothing_or_are_negative_are_refused() {
    let mut doc = pages("boxes.pdf");
    let ids = page_ids(&doc);
    let before = snapshot(&doc);
    for margins in [
        Margins::new(0.0, 300.0, 0.0, 300.0),
        Margins::all(-1.0),
        Margins::new(f64::NAN, 0.0, 0.0, 0.0),
    ] {
        let err = apply(&mut doc, CropPages::new(ids.clone(), margins)).unwrap_err();
        assert!(matches!(err, Error::InvalidOp(_)), "{margins:?}: {err:?}");
    }
    assert_eq!(snapshot(&doc), before);
}

#[test]
fn zero_margins_hide_nothing() {
    let mut doc = pages("boxes.pdf");
    let ids = page_ids(&doc);
    let applied = apply(&mut doc, CropPages::new(ids, Margins::default())).unwrap();
    assert_eq!(applied.cropped_content_remains, Vec::<laminapdf_model::PageId>::new());
}

#[test]
fn set_page_boxes_writes_and_removes_boxes() {
    let mut doc = pages("boxes.pdf");
    let ids = page_ids(&doc);
    let boxes = PageBoxes::default()
        .with_crop(Some(rect(0.0, 0.0, 612.0, 792.0)))
        .with_trim(Some(rect(40.0, 40.0, 572.0, 752.0)))
        .with_art(None);
    let applied = apply(&mut doc, SetPageBoxes::new(vec![ids[0].clone()], boxes)).unwrap();
    let pages = doc.pages().unwrap();
    let page = pages.by_id(&ids[0]).unwrap();
    assert_eq!(
        (
            page.crop_box().unwrap(),
            page.trim_box().unwrap(),
            page.art_box().unwrap(),
            page.bleed_box().unwrap(),
            applied.cropped_content_remains.len()
        ),
        (
            Some(rect(0.0, 0.0, 612.0, 792.0)),
            Some(rect(40.0, 40.0, 572.0, 752.0)),
            // No art box of its own: the crop box (§14.11.2).
            Some(rect(0.0, 0.0, 612.0, 792.0)),
            Some(rect(27.0, 27.0, 585.0, 765.0)),
            // A larger crop box hides nothing.
            0
        )
    );
}

#[test]
fn a_smaller_crop_box_is_reported_and_removing_one_shows_the_media_box() {
    let mut doc = pages("boxes.pdf");
    let ids = page_ids(&doc);
    let smaller = PageBoxes::default().with_crop(Some(rect(100.0, 100.0, 500.0, 700.0)));
    let applied = apply(&mut doc, SetPageBoxes::new(vec![ids[1].clone()], smaller)).unwrap();
    assert_eq!(applied.cropped_content_remains, vec![ids[1].clone()]);
    let removed = PageBoxes::default().with_crop(None);
    let applied = apply(&mut doc, SetPageBoxes::new(vec![ids[1].clone()], removed)).unwrap();
    let pages = doc.pages().unwrap();
    assert_eq!(
        (
            pages.by_id(&ids[1]).unwrap().crop_box().unwrap(),
            applied.cropped_content_remains.len()
        ),
        (Some(rect(0.0, 0.0, 612.0, 792.0)), 0)
    );
}

#[test]
fn a_new_media_box_bounds_the_other_boxes() {
    let mut doc = pages("boxes.pdf");
    let ids = page_ids(&doc);
    // The page's crop box [18 18 594 774] is outside a 300-point media box, so the op must set
    // one inside it too.
    let media = PageBoxes::default()
        .with_media(rect(0.0, 0.0, 300.0, 300.0))
        .with_crop(Some(rect(0.0, 0.0, 300.0, 300.0)));
    let applied = apply(&mut doc, SetPageBoxes::new(vec![ids[0].clone()], media)).unwrap();
    let pages = doc.pages().unwrap();
    assert_eq!(
        (
            pages.by_id(&ids[0]).unwrap().media_box().unwrap(),
            applied.cropped_content_remains,
        ),
        (Some(rect(0.0, 0.0, 300.0, 300.0)), vec![ids[0].clone()])
    );
}

#[test]
fn boxes_outside_the_media_box_or_without_area_are_refused() {
    let mut doc = pages("boxes.pdf");
    let ids = page_ids(&doc);
    let before = snapshot(&doc);
    for boxes in [
        PageBoxes::default().with_trim(Some(rect(-1.0, 0.0, 100.0, 100.0))),
        PageBoxes::default().with_bleed(Some(rect(0.0, 0.0, 613.0, 100.0))),
        PageBoxes::default().with_crop(Some(rect(10.0, 10.0, 10.0, 100.0))),
        PageBoxes::default().with_media(rect(0.0, 0.0, f64::INFINITY, 10.0)),
    ] {
        let err = apply(&mut doc, SetPageBoxes::new(ids.clone(), boxes.clone())).unwrap_err();
        assert!(matches!(err, Error::InvalidOp(_)), "{boxes:?}: {err:?}");
    }
    assert_eq!(snapshot(&doc), before);
}

#[test]
fn undo_puts_every_box_back() {
    let mut doc = pages("boxes.pdf");
    let ids = page_ids(&doc);
    let before = frames(&doc);
    apply(&mut doc, CropPages::new(ids.clone(), Margins::all(36.0))).unwrap();
    apply(
        &mut doc,
        SetPageBoxes::new(ids, PageBoxes::default().with_art(None).with_trim(None)),
    )
    .unwrap();
    doc.undo().unwrap();
    doc.undo().unwrap();
    let pages = doc.pages().unwrap();
    let art: Vec<_> = pages.iter().map(|p| p.art_box().unwrap()).collect();
    assert_eq!(
        (frames(&doc), art),
        (before, vec![Some(rect(54.0, 54.0, 558.0, 738.0)); 4])
    );
}

#[test]
fn both_ops_reshow_the_widgets_on_their_pages() {
    let mut doc = pages("forms.pdf");
    let ids = page_ids(&doc);
    let cropped = apply(&mut doc, CropPages::new(vec![ids[0].clone()], Margins::all(10.0))).unwrap();
    let boxed = apply(
        &mut doc,
        SetPageBoxes::new(vec![ids[1].clone()], PageBoxes::default().with_trim(None)),
    )
    .unwrap();
    let changes = |a: &laminapdf_ops::Applied| a.widgets.iter().map(|w| w.change).collect::<Vec<_>>();
    assert_eq!(
        (changes(&cropped), changes(&boxed)),
        (vec![WidgetChange::Reshown; 2], vec![WidgetChange::Reshown; 2])
    );
    let op: Op = CropPages::new(vec![ids[0].clone()], Margins::all(10.0)).into();
    assert_eq!(op.page_changes().len(), 1);
}
