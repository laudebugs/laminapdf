//! Content edits (todo E7d) change a page only where the edited objects were and now are: each
//! edit's page, rendered before and after, differs only inside the old and new boxes of the
//! objects it named (the render diff mask).
#![cfg(feature = "hayro")]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "tests: a panic is a test failure; pixel coordinates are small"
)]

use std::sync::Arc;

use laminapdf::{
    ContentEdit, ContentObjectKind, Document, EditContent, Matrix, OpenOptions, PageObject, Pt, RectPt, SaveOptions,
    StackPosition,
};
use laminapdf_render::{ByteSource, CancelToken, HayroBackend, RenderBackend, RenderOpenOptions, RenderRequest};
use laminapdf_test_fixtures::fixture;

/// Page 1 at 72 dpi, one pixel a point: width, height and straight RGBA.
fn render(bytes: Vec<u8>) -> (u32, u32, Vec<u8>) {
    let source: Arc<dyn ByteSource> = Arc::new(bytes);
    let doc = HayroBackend::new().open(source, &RenderOpenOptions::new()).unwrap();
    let req = RenderRequest::page(0, 1.0);
    let plan = doc.plan(&req).unwrap();
    let mut out = vec![0; plan.byte_len()];
    doc.render(&req, &mut out, &CancelToken::new()).unwrap();
    (plan.tile.width, plan.tile.height, out)
}

/// The pixels that differ by more than a rounding step, as (x, y).
fn changed(before: &(u32, u32, Vec<u8>), after: &(u32, u32, Vec<u8>)) -> Vec<(u32, u32)> {
    assert_eq!((before.0, before.1), (after.0, after.1));
    let mut out = Vec::new();
    for y in 0..before.1 {
        for x in 0..before.0 {
            let i = ((y * before.0 + x) * 4) as usize;
            let differs = (0..4).any(|c| before.2[i + c].abs_diff(after.2[i + c]) > 2);
            if differs {
                out.push((x, y));
            }
        }
    }
    out
}

/// Whether the pixel lies in one of `boxes`, grown by two pixels for anti-aliasing.
fn inside(boxes: &[RectPt], (x, y): (u32, u32)) -> bool {
    let (x, y) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
    boxes
        .iter()
        .any(|b| x >= b.x0.get() - 2.0 && x <= b.x1.get() + 2.0 && y >= b.y0.get() - 2.0 && y <= b.y1.get() + 2.0)
}

fn moved(bounds: RectPt, dx: f64, dy: f64) -> RectPt {
    RectPt::new(
        Pt::new(bounds.x0.get() + dx),
        Pt::new(bounds.y0.get() + dy),
        Pt::new(bounds.x1.get() + dx),
        Pt::new(bounds.y1.get() + dy),
    )
}

/// Applies `edits` to page 1 of `images_and_vectors.pdf` and checks that what changed lies in
/// `boxes` (given the page's objects).
fn check(edits: impl Fn(&[PageObject]) -> (Vec<ContentEdit>, Vec<RectPt>)) {
    let bytes = fixture("golden/examples/images_and_vectors.pdf");
    let mut doc = Document::open_bytes(bytes.clone(), OpenOptions::new()).unwrap();
    let (edits, boxes) = edits(&doc.content_objects(0).unwrap());
    let page = doc.page_info(0).unwrap().id;
    doc.apply(vec![EditContent::new(page, edits).into()]).unwrap();
    let after = doc.save(&SaveOptions::new()).unwrap().bytes;
    let changed = changed(&render(bytes), &render(after));
    assert!(!changed.is_empty(), "the edit changed nothing");
    let outside: Vec<_> = changed.iter().filter(|&&p| !inside(&boxes, p)).take(5).collect();
    assert_eq!(outside, Vec::<&(u32, u32)>::new(), "pixels changed outside {boxes:?}");
}

fn paths(objects: &[PageObject]) -> Vec<&PageObject> {
    objects.iter().filter(|o| o.kind == ContentObjectKind::Path).collect()
}

#[test]
fn moving_a_shape_changes_only_where_it_was_and_now_is() {
    check(|objects| {
        let circle = paths(objects)[1];
        let was = circle.bounds.unwrap();
        let edit = ContentEdit::Transform {
            object: circle.object,
            matrix: Matrix::translate(0.0, 500.0),
        };
        (vec![edit], vec![was, moved(was, 0.0, 500.0)])
    });
}

#[test]
fn deleting_and_moving_an_image_change_only_their_boxes() {
    check(|objects| {
        let square = paths(objects)[0];
        let image = objects
            .iter()
            .rev()
            .find(|o| o.kind == ContentObjectKind::Image)
            .unwrap();
        let (square_box, image_box) = (square.bounds.unwrap(), image.bounds.unwrap());
        let edits = vec![
            ContentEdit::Delete { object: square.object },
            ContentEdit::Transform {
                object: image.object,
                matrix: Matrix::translate(300.0, 0.0),
            },
        ];
        (edits, vec![square_box, image_box, moved(image_box, 300.0, 0.0)])
    });
}

#[test]
fn a_shape_sent_to_the_back_and_moved_under_another_changes_only_its_boxes() {
    check(|objects| {
        // The circle, moved 80 points left, overlaps the rectangle; at the back, it goes under it.
        let circle = paths(objects)[1];
        let was = circle.bounds.unwrap();
        let edits = vec![
            ContentEdit::Reorder {
                object: circle.object,
                to: StackPosition::Back,
            },
            ContentEdit::Transform {
                object: circle.object,
                matrix: Matrix::translate(-80.0, 0.0),
            },
        ];
        (edits, vec![was, moved(was, -80.0, 0.0)])
    });
}
