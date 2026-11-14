//! `ResizePages` and `SetOrientation` over `fixtures/pages/geometry.pdf` (an A4 page 1 that
//! every kind of destination aims at, and a Letter page 2) and `boxes.pdf` (one page per
//! rotation, the second with `/UserUnit 2`): the content's matrix, the destinations, structure
//! boxes and annotations that follow it, what is left alone, refusals and undo.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{fixture, open, page_ids, snapshot};
use laminapdf_cos::{ObjRef, Object};
use laminapdf_model::{PageSize, Pt, Rotation};
use laminapdf_ops::{
    Anchor, Document, Error, Op, Orientation, OrientationMethod, Origin, ResizePages, SetOrientation, WidgetChange,
};

const A4: (f64, f64) = (595.28, 841.89);

fn pages(name: &str) -> Document {
    open(&fixture(&format!("pages/{name}")))
}

fn apply(doc: &mut Document, op: impl Into<Op>) -> Result<laminapdf_ops::Applied, Error> {
    doc.apply_all(Origin::Local, [op.into()])
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

/// The numbers of an array object (`None` for `null`).
fn numbers(o: &Object) -> Vec<Option<f64>> {
    o.as_array().unwrap().iter().map(Object::as_f64).collect()
}

fn assert_numbers(actual: &[Option<f64>], expected: &[Option<f64>], what: &str) {
    assert_eq!(actual.len(), expected.len(), "{what}: {actual:?}");
    for (a, e) in actual.iter().zip(expected) {
        match (a, e) {
            (Some(a), Some(e)) => assert!(close(*a, *e), "{what}: {actual:?} vs {expected:?}"),
            (None, None) => {}
            _ => panic!("{what}: {actual:?} vs {expected:?}"),
        }
    }
}

/// Object `r`, one reference followed.
fn object(doc: &Document, r: ObjRef) -> Object {
    (*doc.resolve(r).unwrap()).clone()
}

fn get(doc: &Document, o: &Object, key: &str) -> Object {
    let value = o.as_dict().unwrap().get(key).unwrap().clone();
    match value {
        Object::Ref(r) => object(doc, r),
        other => other,
    }
}

/// The catalog.
fn catalog(doc: &Document) -> Object {
    Object::Dict(doc.catalog().unwrap())
}

/// A destination's operands after its page and kind, and its kind.
fn operands(dest: &Object) -> (String, Vec<Option<f64>>) {
    let items = dest.as_array().unwrap();
    let Object::Name(kind) = &items[1] else {
        panic!("no kind: {dest:?}")
    };
    (
        String::from_utf8(kind.as_bytes().to_vec()).unwrap(),
        items[2..].iter().map(Object::as_f64).collect(),
    )
}

/// The first and second outline items' destinations, the named destinations `one` and `two`,
/// `/OpenAction`, and the paragraph's Layout `/BBox`.
struct Places {
    first: Object,
    second: Object,
    one: Object,
    two: Object,
    open: Object,
    bbox: Object,
}

fn places(doc: &Document) -> Places {
    let cat = catalog(doc);
    let outlines = get(doc, &cat, "Outlines");
    let first = get(doc, &outlines, "First");
    let second = get(doc, &first, "Next");
    let tree = doc.name_tree("Dests").unwrap().unwrap();
    let entries = tree.entries().unwrap();
    let named = |n: &[u8]| entries.iter().find(|(k, _)| k == n).unwrap().1.clone();
    let root = get(doc, &cat, "StructTreeRoot");
    let document = get(doc, &root, "K");
    let para = get(doc, &document, "K").as_array().unwrap()[0].clone();
    let para = match para {
        Object::Ref(r) => object(doc, r),
        other => other,
    };
    Places {
        first: get(doc, &first, "Dest"),
        second: get(doc, &second, "Dest"),
        one: named(b"one"),
        two: named(b"two"),
        open: get(doc, &cat, "OpenAction"),
        bbox: get(doc, &get(doc, &para, "A"), "BBox"),
    }
}

/// Page `index`'s annotations, resolved.
fn annots(doc: &Document, index: usize) -> Vec<Object> {
    let page = page_ids(doc)[index].as_lamina_id().obj_ref().unwrap();
    let annots = get(doc, &object(doc, page), "Annots");
    annots
        .as_array()
        .unwrap()
        .iter()
        .map(|a| object(doc, a.as_ref().unwrap()))
        .collect()
}

/// The scale and the shift across of A4 → Letter by `scale`: it fits by height, centred across.
fn a4_to_letter() -> (f64, f64) {
    let s = 792.0 / A4.1;
    (s, (612.0 - s * A4.0) / 2.0)
}

#[test]
fn resizing_a4_to_letter_moves_every_destination_aimed_at_the_page_and_no_other() {
    let mut doc = pages("geometry.pdf");
    let ids = page_ids(&doc);
    let before = places(&doc);
    apply(&mut doc, ResizePages::scale(vec![ids[0].clone()], PageSize::LETTER)).unwrap();
    let (s, ox) = a4_to_letter();
    let after = places(&doc);
    let x = |v: f64| Some(s.mul_add(v, ox));
    let y = |v: f64| Some(s * v);
    let checks = [
        (
            operands(&after.first),
            ("XYZ", vec![x(72.0), y(700.0), Some(0.0)]),
            "outline /XYZ",
        ),
        (operands(&after.one), ("FitH", vec![y(700.0)]), "named /FitH"),
        (operands(&after.open), ("FitV", vec![x(72.0)]), "/OpenAction /FitV"),
    ];
    for ((kind, values), (want_kind, want), what) in checks {
        assert_eq!(kind, want_kind, "{what}");
        assert_numbers(&values, &want, what);
    }
    assert_numbers(
        &numbers(&after.bbox),
        &[x(72.0), y(690.0), x(200.0), y(730.0)],
        "Layout /BBox",
    );
    // Page 2's destinations are left as they were.
    assert_eq!((after.second, after.two), (before.second, before.two));
    // The link's /FitR, held in its GoTo action.
    let link = &annots(&doc, 0)[0];
    let goto = get(&doc, link, "A");
    let (kind, values) = operands(&get(&doc, &goto, "D"));
    assert_eq!(kind, "FitR");
    assert_numbers(&values, &[x(72.0), y(690.0), x(200.0), y(730.0)], "link /FitR");
}

#[test]
fn annotations_and_the_paper_follow_the_resize() {
    let mut doc = pages("geometry.pdf");
    let ids = page_ids(&doc);
    apply(&mut doc, ResizePages::scale(vec![ids[0].clone()], PageSize::LETTER)).unwrap();
    let (s, ox) = a4_to_letter();
    let x = |v: f64| Some(s.mul_add(v, ox));
    let y = |v: f64| Some(s * v);
    let a = annots(&doc, 0);
    assert_numbers(
        &numbers(&get(&doc, &a[0], "Rect")),
        &[x(72.0), y(690.0), x(200.0), y(730.0)],
        "link",
    );
    assert_numbers(
        &numbers(&get(&doc, &a[1], "QuadPoints")),
        &[
            x(72.0),
            y(730.0),
            x(200.0),
            y(730.0),
            x(72.0),
            y(690.0),
            x(200.0),
            y(690.0),
        ],
        "highlight quads",
    );
    let ink = get(&doc, &a[2], "InkList");
    assert_numbers(
        &numbers(&ink.as_array().unwrap()[0]),
        &[x(300.0), y(300.0), x(350.0), y(380.0), x(400.0), y(300.0)],
        "ink",
    );
    let pages = doc.pages().unwrap();
    let page = pages.get(0).unwrap();
    assert_eq!(
        (
            page.media_box().unwrap().map(|r| r.to_array()),
            page.cos().dict.get("CropBox").is_some(),
        ),
        (Some([0.0, 0.0, 612.0, 792.0]), false)
    );
}

/// The matrix of the `cm` the op put first inside the page's wrap.
fn content_matrix(doc: &Document, index: usize) -> [f64; 6] {
    let pages = doc.pages().unwrap();
    let page = pages.get(index).unwrap();
    for r in page.contents().unwrap() {
        let Object::Stream(stream) = object(doc, r) else {
            continue;
        };
        let bytes = doc.decode_stream(&stream).unwrap();
        let text = String::from_utf8_lossy(&bytes).into_owned();
        if let Some(line) = text.lines().find(|l| l.ends_with(" cm")) {
            let v: Vec<f64> = line.split(' ').take(6).map(|n| n.parse().unwrap()).collect();
            return <[f64; 6]>::try_from(v).unwrap();
        }
    }
    panic!("no cm on page {index}");
}

/// `m` applied to `(x, y)`.
fn apply_matrix(m: &[f64; 6], x: f64, y: f64) -> (f64, f64) {
    (m[0].mul_add(x, m[2] * y) + m[4], m[1].mul_add(x, m[3] * y) + m[5])
}

#[test]
fn extending_keeps_the_scale_and_puts_the_old_corner_at_the_anchor_in_every_rotation() {
    let mut doc = pages("boxes.pdf");
    let ids = page_ids(&doc);
    let before: Vec<_> = doc.pages().unwrap().iter().map(|p| p.frame().unwrap()).collect();
    let size = PageSize {
        width: Pt::new(700.0),
        height: Pt::new(900.0),
    };
    apply(&mut doc, ResizePages::extend(ids, size, Anchor::TopLeft)).unwrap();
    let after: Vec<_> = doc.pages().unwrap().iter().map(|p| p.frame().unwrap()).collect();
    for (i, (old, new)) in before.iter().zip(&after).enumerate() {
        let quarter = matches!(old.rotate(), Rotation::Clockwise90 | Rotation::Clockwise270);
        let (w, h) = if quarter { (900.0, 700.0) } else { (700.0, 900.0) };
        assert!(
            close(new.width().get(), w) && close(new.height().get(), h),
            "page {i} size"
        );
        // The old page's top-left corner, as shown, lands on the new one's.
        let corner = old.to_user_space(laminapdf_model::PointPt::new(Pt::ZERO, Pt::ZERO));
        let m = content_matrix(&doc, i);
        let (cx, cy) = apply_matrix(&m, corner.x, corner.y);
        let shown = new.to_page(laminapdf_model::UserSpacePoint::new(cx, cy));
        assert!(
            close(shown.x.get(), 0.0) && close(shown.y.get(), 0.0),
            "page {i} corner: {shown:?}"
        );
        // Extending never scales: the matrix keeps lengths.
        assert!(close(m[0].hypot(m[1]), 1.0), "page {i} scale");
    }
}

#[test]
fn swapping_the_box_turns_the_page_and_everything_on_it_a_quarter_clockwise() {
    let mut doc = pages("geometry.pdf");
    let ids = page_ids(&doc);
    let applied = apply(
        &mut doc,
        SetOrientation::new(ids, Orientation::Landscape, OrientationMethod::SwapBox),
    )
    .unwrap();
    // Both pages were portrait; each turns, keeping /Rotate.
    assert_eq!(applied.touched_pages.len(), 2);
    let frame = doc.pages().unwrap().get(0).unwrap().frame().unwrap();
    assert!(close(frame.width().get(), A4.1) && close(frame.height().get(), A4.0));
    assert_eq!(frame.rotate(), Rotation::None);
    // (x, y) → (y, 595.28 − x): the outline's point, and /FitH becomes /FitV.
    let after = places(&doc);
    let (kind, values) = operands(&after.first);
    assert_eq!(kind, "XYZ");
    assert_numbers(&values, &[Some(700.0), Some(A4.0 - 72.0), Some(0.0)], "turned /XYZ");
    let (kind, values) = operands(&after.one);
    assert_eq!(kind, "FitV");
    assert_numbers(&values, &[Some(700.0)], "turned /FitH");
    // The highlight's appearance turns with it.
    let mark = &annots(&doc, 0)[1];
    let ap = get(&doc, mark, "AP");
    let Object::Ref(stream) = ap.as_dict().unwrap().get("N").unwrap().clone() else {
        panic!("indirect appearance");
    };
    let Object::Stream(s) = object(&doc, stream) else {
        panic!("not a stream")
    };
    assert_numbers(
        &numbers(s.dict.get("Matrix").unwrap()),
        &[Some(0.0), Some(-1.0), Some(1.0), Some(0.0), Some(0.0), Some(0.0)],
        "appearance matrix",
    );
}

#[test]
fn setting_the_orientation_by_rotate_turns_only_the_pages_shown_the_other_way() {
    let mut doc = pages("boxes.pdf");
    let ids = page_ids(&doc);
    // Pages 1 and 3 show portrait (0°, 180°), pages 2 and 4 landscape (90°, 270°).
    let applied = apply(
        &mut doc,
        SetOrientation::new(ids.clone(), Orientation::Landscape, OrientationMethod::Rotate),
    )
    .unwrap();
    let rotations: Vec<_> = doc.pages().unwrap().iter().map(|p| p.rotate().unwrap()).collect();
    assert_eq!(
        (applied.touched_pages.into_iter().collect::<Vec<_>>(), rotations),
        (
            vec![ids[0].clone(), ids[2].clone()],
            vec![
                Rotation::Clockwise90,
                Rotation::Clockwise90,
                Rotation::Clockwise270,
                Rotation::Clockwise270
            ]
        )
    );
}

#[test]
fn bad_sizes_are_refused_and_change_nothing() {
    let mut doc = pages("geometry.pdf");
    let ids = page_ids(&doc);
    let before = snapshot(&doc);
    for (w, h) in [(0.0, 792.0), (612.0, 14_401.0), (f64::NAN, 792.0)] {
        let size = PageSize {
            width: Pt::new(w),
            height: Pt::new(h),
        };
        let err = apply(&mut doc, ResizePages::scale(ids.clone(), size)).unwrap_err();
        assert!(matches!(err, Error::InvalidOp(_)), "{w} × {h}: {err:?}");
    }
    assert_eq!(snapshot(&doc), before);
}

#[test]
fn undo_takes_back_a_resize_and_a_turn() {
    let mut doc = pages("geometry.pdf");
    let ids = page_ids(&doc);
    let before = snapshot(&doc);
    apply(&mut doc, ResizePages::scale(ids.clone(), PageSize::LETTER)).unwrap();
    apply(
        &mut doc,
        SetOrientation::new(ids, Orientation::Landscape, OrientationMethod::SwapBox),
    )
    .unwrap();
    doc.undo().unwrap();
    doc.undo().unwrap();
    assert_eq!(snapshot(&doc), before);
}

#[test]
fn both_ops_reshow_the_widgets_on_their_pages() {
    let mut doc = pages("forms.pdf");
    let ids = page_ids(&doc);
    let resized = apply(&mut doc, ResizePages::scale(vec![ids[0].clone()], PageSize::A4)).unwrap();
    let turned = apply(
        &mut doc,
        SetOrientation::new(vec![ids[1].clone()], Orientation::Landscape, OrientationMethod::SwapBox),
    )
    .unwrap();
    let changes = |a: &laminapdf_ops::Applied| a.widgets.iter().map(|w| w.change).collect::<Vec<_>>();
    assert_eq!(
        (changes(&resized), changes(&turned)),
        (vec![WidgetChange::Reshown; 2], vec![WidgetChange::Reshown; 2])
    );
}

#[test]
fn a_page_whose_content_lamina_pdf_cant_decode_is_refused_and_left_alone() {
    // ReportLab writes its content streams ASCII85-encoded, a filter Lamina PDF doesn't decode:
    // the content must be read to be wrapped, so the resize stops before changing anything.
    let mut doc = common::corpus("reportlab.pdf");
    let ids = page_ids(&doc);
    let before = snapshot(&doc);
    let err = apply(&mut doc, ResizePages::scale(ids, PageSize::A4)).unwrap_err();
    assert_eq!((err.code(), snapshot(&doc) == before), ("unsupported-filter", true));
}
