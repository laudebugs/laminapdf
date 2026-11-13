//! Marking up a page through the façade: text marks from line boxes in page space at every
//! rotation, notes, carets, `replace_text` as one undo step, a tagged file's default
//! descriptions, and every mark saved and read back with its appearance.
#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test code: a panic is a test failure"
)]

use laminapdf::{
    AnnotCommon, AnnotationGeometry, Document, MarkupKind, NoteIcon, OpenOptions, PageAnnotation, PointPt, Pt, RectPt,
    SaveOptions,
};
use laminapdf_test_fixtures::fixture;

fn open(path: &str) -> Document {
    Document::open_bytes(fixture(path), OpenOptions::default()).unwrap()
}

/// A one-page document whose page is turned by `rotate` (`/Rotate`), 612 × 792 points unturned.
fn turned(rotate: u16) -> Document {
    let page = format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Rotate {rotate} >>");
    let bytes = laminapdf_test_fixtures::classic_with_root(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            &page,
        ],
        "",
    );
    Document::open_bytes(bytes, OpenOptions::default()).unwrap()
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> RectPt {
    RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
}

fn near(a: Pt, b: Pt) -> bool {
    (a.get() - b.get()).abs() < 1e-6
}

fn same(a: RectPt, b: RectPt) -> bool {
    near(a.x0, b.x0) && near(a.y0, b.y0) && near(a.x1, b.x1) && near(a.y1, b.y1)
}

/// The last annotation of page 0 of `doc` saved and opened again.
fn saved_and_reopened(doc: &mut Document) -> Vec<PageAnnotation> {
    let bytes = doc.save(&SaveOptions::default()).unwrap().bytes;
    Document::open_bytes(bytes, OpenOptions::default())
        .unwrap()
        .page_annotations(0)
        .unwrap()
}

#[test]
fn a_highlight_on_a_line_box_reads_back_on_that_box_at_every_rotation() {
    for rotate in [0, 90, 180, 270] {
        let mut doc = turned(rotate);
        let line = rect(40.0, 60.0, 200.0, 74.0);
        let op = doc.mark_text(0, MarkupKind::Highlight, &[line]).unwrap();
        doc.apply(vec![op.into()]).unwrap();
        let annot = doc.page_annotations(0).unwrap().pop().unwrap();
        let AnnotationGeometry::Quads(quads) = &annot.geometry else {
            panic!("/Rotate {rotate}: no quads")
        };
        let [ul, ur, ll, lr] = quads[0];
        assert!(
            same(annot.rect.unwrap(), line)
                && near(ul.x, line.x0)
                && near(ul.y, line.y0)
                && near(ur.x, line.x1)
                && near(ll.y, line.y1)
                && near(lr.x, line.x1),
            "/Rotate {rotate}: {annot:?}"
        );
    }
}

#[test]
fn every_mark_is_saved_with_its_appearance() {
    let mut doc = turned(0);
    let line = rect(40.0, 60.0, 200.0, 74.0);
    let mut ops = Vec::new();
    for kind in [
        MarkupKind::Highlight,
        MarkupKind::Underline,
        MarkupKind::StrikeOut,
        MarkupKind::Squiggly,
    ] {
        ops.push(doc.mark_text(0, kind, &[line]).unwrap().into());
    }
    ops.push(
        doc.note_at(0, PointPt::from_points(300.0, 100.0), "Why?", NoteIcon::Help)
            .unwrap()
            .into(),
    );
    ops.push(doc.caret_at(0, rect(200.0, 64.0, 206.0, 74.0)).unwrap().into());
    doc.apply(ops).unwrap();
    let annots = saved_and_reopened(&mut doc);
    let subtypes: Vec<&str> = annots.iter().map(|a| a.subtype.as_str()).collect();
    assert_eq!(
        subtypes,
        [
            "Highlight",
            "Underline",
            "StrikeOut",
            "Squiggly",
            "Text",
            "Popup",
            "Caret"
        ]
    );
    assert!(
        annots.iter().filter(|a| a.subtype != "Popup").all(|a| a.has_appearance),
        "{annots:?}"
    );
}

#[test]
fn a_note_sits_at_the_point_in_page_space() {
    let mut doc = turned(0);
    let op = doc
        .note_at(0, PointPt::from_points(300.0, 100.0), "Check", NoteIcon::Comment)
        .unwrap();
    doc.apply(vec![op.into()]).unwrap();
    let note = doc
        .page_annotations(0)
        .unwrap()
        .into_iter()
        .find(|a| a.subtype == "Text")
        .unwrap();
    assert!(
        same(note.rect.unwrap(), rect(300.0, 100.0, 320.0, 120.0)) && note.contents.as_deref() == Some("Check"),
        "{note:?}"
    );
}

#[test]
fn a_notes_popup_opens_beside_it_at_every_rotation() {
    for rotate in [0, 90, 180, 270] {
        let mut doc = turned(rotate);
        let op = doc
            .note_at(0, PointPt::from_points(100.0, 100.0), "Check", NoteIcon::Note)
            .unwrap();
        doc.apply(vec![op.into()]).unwrap();
        let popup = doc
            .page_annotations(0)
            .unwrap()
            .into_iter()
            .find(|a| a.subtype == "Popup")
            .unwrap();
        assert!(
            same(popup.rect.unwrap(), rect(120.0, 100.0, 300.0, 220.0)),
            "/Rotate {rotate}: {popup:?}"
        );
    }
}

#[test]
fn a_popup_near_the_right_edge_opens_left_of_its_note() {
    let mut doc = turned(0);
    let op = doc
        .note_at(0, PointPt::from_points(580.0, 100.0), "Check", NoteIcon::Note)
        .unwrap();
    doc.apply(vec![op.into()]).unwrap();
    let popup = doc
        .page_annotations(0)
        .unwrap()
        .into_iter()
        .find(|a| a.subtype == "Popup")
        .unwrap();
    assert!(same(popup.rect.unwrap(), rect(400.0, 100.0, 580.0, 220.0)), "{popup:?}");
}

#[test]
fn replace_text_groups_a_caret_and_a_strike_out_in_one_undo_step() {
    let mut doc = turned(0);
    let line = rect(40.0, 60.0, 200.0, 74.0);
    let edited = doc
        .replace_text(0, &[line], "new words", AnnotCommon::default())
        .unwrap();
    let annots = doc.page_annotations(0).unwrap();
    let caret = annots.iter().find(|a| a.subtype == "Caret").unwrap();
    let strike = annots.iter().find(|a| a.subtype == "StrikeOut").unwrap();
    assert_eq!(
        (
            edited.created_annotations.len(),
            caret.contents.as_deref(),
            strike.in_reply_to.as_deref(),
            strike.grouped,
        ),
        (2, Some("new words"), Some(caret.id.as_str()), true)
    );
    doc.undo().unwrap();
    assert!(doc.page_annotations(0).unwrap().is_empty());
}

#[test]
fn a_mark_in_a_tagged_file_is_described_by_the_text_it_marks() {
    let doc = open("golden/examples/pdf_ua_report.pdf");
    let text = doc.page_text(0, &laminapdf::extract::TextOptions::default()).unwrap();
    // The page's first tagged run (its header is an artifact): the box of its glyphs.
    let run = text
        .runs
        .iter()
        .find(|r| text.glyphs[r.glyphs.clone()].iter().any(|g| g.mcid.is_some()))
        .unwrap();
    let glyphs = &text.glyphs[run.glyphs.clone()];
    let x0 = glyphs.iter().map(|g| g.quad[0].x.get()).fold(f64::INFINITY, f64::min);
    let x1 = glyphs
        .iter()
        .map(|g| g.quad[1].x.get())
        .fold(f64::NEG_INFINITY, f64::max);
    let y0 = glyphs.iter().map(|g| g.quad[0].y.get()).fold(f64::INFINITY, f64::min);
    let y1 = glyphs
        .iter()
        .map(|g| g.quad[2].y.get())
        .fold(f64::NEG_INFINITY, f64::max);
    let op = doc
        .mark_text(0, MarkupKind::Highlight, &[rect(x0, y0, x1, y1)])
        .unwrap();
    let expected: String = glyphs.iter().map(|g| g.text.as_str()).collect();
    assert_eq!(op.common.contents.as_deref(), Some(expected.trim()));
    assert!(!op.marked.is_empty());
}

#[test]
fn a_mark_on_no_lines_is_refused() {
    let doc = turned(0);
    assert_eq!(
        doc.mark_text(0, MarkupKind::Highlight, &[]).unwrap_err().code(),
        "invalid-op"
    );
}
