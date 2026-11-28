//! `SetTextStyle` (todo E7f, ADR 0022 §6, E7 plan §2.1): runs restyled, and their lines aligned
//! and spaced, with the text unchanged.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{classic, fixture, open, snapshot, stream_body};
use laminapdf_model::PageId;
use laminapdf_ops::{
    AnnotColor, AnnotFont, Baseline, Document, Op, Origin, SetTextStyle, StandardFamily, TextAlign, TextRunRef,
    text_runs,
};
use laminapdf_save::{Save, SaveOptions};
use laminapdf_text::{Glyph, TextExtractor};

/// Two lines in one text object, the first made of two runs, in Helvetica (not embedded).
const LINES: &[u8] = b"BT /F1 12 Tf 20 150 Td (Hello world) Tj 80 0 Td (after) Tj -80 -20 Td (below) Tj ET\n";

fn page_with(content: &[u8], font: &[u8]) -> Document {
    let content = stream_body("", content);
    let objects: [&[u8]; 5] = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 200] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
        &content,
        font,
    ];
    open(&classic(&objects, "<< /Size 6 /Root 1 0 R >>"))
}

fn helvetica() -> Document {
    page_with(LINES, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>")
}

fn page(doc: &Document) -> PageId {
    doc.pages().unwrap().get(0).unwrap().id().clone()
}

fn run(doc: &Document, text: &str) -> TextRunRef {
    text_runs(doc, &page(doc))
        .unwrap()
        .into_iter()
        .find(|r| r.text == text)
        .unwrap()
        .run
}

fn style(doc: &Document, text: &str) -> SetTextStyle {
    SetTextStyle::new(page(doc), vec![run(doc, text)])
}

fn apply(doc: &mut Document, op: SetTextStyle) -> laminapdf_ops::Result<laminapdf_ops::Applied> {
    doc.apply_all(Origin::Local, [Op::from(op)])
}

/// The glyphs of the run that shows `text`, as extraction reads the page now.
fn glyphs(doc: &Document, text: &str) -> Vec<Glyph> {
    let pages = doc.pages().unwrap();
    let page = TextExtractor::new().page(doc, &pages.get(0).unwrap()).unwrap();
    // The glyphs' texts joined, which a run of `text` is a stretch of.
    let all: String = page.glyphs.iter().map(|g| g.text.as_str()).collect();
    let at = all.find(text).unwrap();
    let first = all[..at].chars().count();
    page.glyphs[first..first + text.chars().count()].to_vec()
}

fn lines(doc: &Document) -> Vec<String> {
    let pages = doc.pages().unwrap();
    TextExtractor::new().page(doc, &pages.get(0).unwrap()).unwrap().lines()
}

fn saved(doc: &mut Document) -> Vec<u8> {
    let mut out = Vec::new();
    doc.save(&mut out, &SaveOptions::default()).unwrap();
    out
}

#[test]
fn a_colour_and_a_size_should_change_only_the_run_they_name() {
    let mut doc = helvetica();
    let mut op = style(&doc, "after");
    op.color = Some(AnnotColor::Rgb(1.0, 0.0, 0.0));
    op.size = Some(24.0);
    apply(&mut doc, op).unwrap();
    let after = glyphs(&doc, "after");
    assert!((after[0].size - 24.0).abs() < 1e-6);
    assert_eq!(after[0].color, Some([1.0, 0.0, 0.0]));
    // The run before it, the run after it and the text are as they were.
    let hello = glyphs(&doc, "Hello world");
    assert!((hello[0].size - 12.0).abs() < 1e-6);
    assert_eq!(hello[0].color, Some([0.0, 0.0, 0.0]));
    assert_eq!(lines(&doc), ["Hello world after", "below"]);
    let below = glyphs(&doc, "below");
    assert!((below[0].size - 12.0).abs() < 1e-6);
    assert_eq!(below[0].color, Some([0.0, 0.0, 0.0]));
}

#[test]
fn a_bigger_run_should_move_the_text_after_it_on_its_line_and_leave_the_lines_below() {
    let mut doc = helvetica();
    let before = glyphs(&doc, "below")[0].origin;
    let hello = glyphs(&doc, "Hello world");
    let mut op = style(&doc, "Hello world");
    op.size = Some(24.0);
    apply(&mut doc, op).unwrap();
    let after = glyphs(&doc, "below")[0].origin;
    assert!((after.x - before.x).abs() < 1e-6 && (after.y - before.y).abs() < 1e-6);
    assert!(glyphs(&doc, "Hello world")[0].origin.x - hello[0].origin.x < 1e-6);
}

#[test]
fn bold_should_choose_the_bold_face_of_a_standard_font() {
    let mut doc = helvetica();
    let mut op = style(&doc, "Hello world");
    op.bold = Some(true);
    apply(&mut doc, op).unwrap();
    assert_eq!(&*glyphs(&doc, "Hello world")[0].font_name, "Helvetica-Bold");
    assert_eq!(lines(&doc), ["Hello world after", "below"]);
}

#[test]
fn italic_after_bold_should_choose_the_bold_italic_face() {
    let mut doc = helvetica();
    let mut op = style(&doc, "Hello world");
    op.bold = Some(true);
    op.italic = Some(true);
    apply(&mut doc, op).unwrap();
    assert_eq!(&*glyphs(&doc, "Hello world")[0].font_name, "Helvetica-BoldOblique");
}

#[test]
fn a_family_should_replace_the_runs_font() {
    let mut doc = helvetica();
    let mut op = style(&doc, "after");
    op.font = Some(AnnotFont::Standard(StandardFamily::Times));
    apply(&mut doc, op).unwrap();
    assert_eq!(&*glyphs(&doc, "after")[0].font_name, "Times-Roman");
    assert_eq!(lines(&doc), ["Hello world after", "below"]);
}

#[test]
fn a_font_given_by_its_bytes_should_be_embedded_and_draw_the_text() {
    let mut doc = helvetica();
    let noto = doc.add_blob(fixture("fonts/NotoSans-Regular.ttf"));
    let mut op = style(&doc, "after");
    op.font = Some(AnnotFont::Font(noto));
    apply(&mut doc, op).unwrap();
    assert!(glyphs(&doc, "after")[0].font_name.ends_with("NotoSans-Regular"));
    assert_eq!(lines(&doc), ["Hello world after", "below"]);
    let bytes = saved(&mut doc);
    let reopened = open(&bytes);
    assert_eq!(lines(&reopened), ["Hello world after", "below"]);
}

#[test]
fn a_font_that_lacks_a_character_should_be_a_missing_glyph() {
    let mut doc = page_with(
        b"BT /F1 12 Tf 20 150 Td (\xA4) Tj ET\n",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>",
    );
    let noto = doc.add_blob(fixture("fonts/NotoSans-Regular.ttf"));
    let mut op = SetTextStyle::new(page(&doc), vec![text_runs(&doc, &page(&doc)).unwrap()[0].run]);
    op.font = Some(AnnotFont::Font(noto));
    // NotoSans has a glyph for the currency sign, so the run changes.
    apply(&mut doc, op).unwrap();
}

#[test]
fn superscript_should_shrink_and_raise_the_run() {
    let mut doc = helvetica();
    let before = glyphs(&doc, "after")[0].origin;
    let mut op = style(&doc, "after");
    op.baseline = Some(Baseline::Super);
    apply(&mut doc, op).unwrap();
    let after = glyphs(&doc, "after")[0].clone();
    assert!((after.size - 12.0 * 0.65).abs() < 1e-6);
    assert!(after.origin.y > before.y + 2.0, "{before:?} {:?}", after.origin);
}

#[test]
fn subscript_should_lower_the_run() {
    let mut doc = helvetica();
    let before = glyphs(&doc, "after")[0].origin;
    let mut op = style(&doc, "after");
    op.baseline = Some(Baseline::Sub);
    apply(&mut doc, op).unwrap();
    assert!(glyphs(&doc, "after")[0].origin.y < before.y - 1.0);
}

#[test]
fn an_underline_should_add_a_path_under_the_run() {
    let mut doc = helvetica();
    let objects = |doc: &Document| -> usize {
        let pages = doc.pages().unwrap();
        let page = pages.get(0).unwrap();
        let ir = laminapdf_content::ContentOps::read_page(
            doc,
            page.cos(),
            laminapdf_content::ContentOwner::Page(page.id().clone()),
        )
        .unwrap();
        ir.objects().len()
    };
    let before = objects(&doc);
    let mut op = style(&doc, "after");
    op.underline = Some(true);
    apply(&mut doc, op).unwrap();
    assert_eq!(objects(&doc), before + 1);
    assert_eq!(lines(&doc), ["Hello world after", "below"]);
}

#[test]
fn taking_an_underline_off_should_be_refused() {
    let mut doc = helvetica();
    let mut op = style(&doc, "after");
    op.underline = Some(false);
    assert_eq!(apply(&mut doc, op).unwrap_err().code(), "invalid-op");
}

#[test]
fn bold_on_a_font_with_no_bold_face_to_take_should_say_to_set_the_font() {
    let mut doc = page_with(
        b"BT /F1 12 Tf 20 150 Td (Hi) Tj ET\n",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Arial >>",
    );
    let mut op = SetTextStyle::new(page(&doc), vec![text_runs(&doc, &page(&doc)).unwrap()[0].run]);
    op.bold = Some(true);
    assert_eq!(apply(&mut doc, op).unwrap_err().code(), "invalid-op");
}

#[test]
fn right_alignment_should_end_each_line_at_the_blocks_right_edge() {
    let mut doc = helvetica();
    let end = |doc: &Document, text: &str| {
        let g = glyphs(doc, text);
        let last = g.last().unwrap();
        last.origin.x + last.advance.0
    };
    let block = end(&doc, "after");
    let mut op = style(&doc, "below");
    op.align = Some(TextAlign::Right);
    apply(&mut doc, op).unwrap();
    assert!((end(&doc, "below") - block).abs() < 1e-3, "{} {block}", end(&doc, "below"));
    // The first line stays.
    assert_eq!(lines(&doc), ["Hello world after", "below"]);
}

#[test]
fn centring_should_put_the_line_in_the_middle_of_the_block() {
    let mut doc = helvetica();
    let extent = |doc: &Document, text: &str| {
        let g = glyphs(doc, text);
        (g[0].origin.x, g.last().map(|l| l.origin.x + l.advance.0).unwrap())
    };
    let (left, _) = extent(&doc, "Hello world");
    let (_, right) = extent(&doc, "after");
    let mut op = style(&doc, "below");
    op.align = Some(TextAlign::Center);
    apply(&mut doc, op).unwrap();
    let (from, to) = extent(&doc, "below");
    assert!(((from + to) / 2.0 - (left + right) / 2.0).abs() < 1e-3);
}

#[test]
fn line_spacing_should_set_the_distance_between_the_runs_lines() {
    let mut doc = helvetica();
    let first = glyphs(&doc, "Hello world")[0].origin.y;
    let hello = run(&doc, "Hello world");
    let below = run(&doc, "below");
    let mut op = SetTextStyle::new(page(&doc), vec![hello, below]);
    op.line_spacing = Some(2.0);
    apply(&mut doc, op).unwrap();
    let second = glyphs(&doc, "below")[0].origin.y;
    assert!(((first - second) - 24.0).abs() < 1e-6, "{first} {second}");
}

#[test]
fn a_style_should_be_class_content_and_save_incrementally() {
    let doc = helvetica();
    let mut style = style(&doc, "after");
    style.bold = Some(true);
    let op = Op::from(style);
    assert_eq!(op.class(), laminapdf_ops::ModificationClass::Content);
    assert!(!op.requires_rewrite());
}

#[test]
fn undoing_a_style_should_leave_the_overlay_empty_and_the_save_unchanged() {
    let before = saved(&mut helvetica());
    let mut doc = helvetica();
    let empty = snapshot(&doc);
    let mut op = style(&doc, "after");
    op.font = Some(AnnotFont::Standard(StandardFamily::Courier));
    op.underline = Some(true);
    op.align = Some(TextAlign::Right);
    apply(&mut doc, op).unwrap();
    doc.undo().unwrap();
    assert_eq!(snapshot(&doc), empty);
    assert_eq!(saved(&mut doc), before);
}

#[test]
fn redoing_a_style_should_write_the_same_bytes_again() {
    let make = |doc: &mut Document| {
        let mut op = style(doc, "after");
        op.font = Some(AnnotFont::Standard(StandardFamily::Courier));
        op.size = Some(18.0);
        apply(doc, op).unwrap();
    };
    let mut once = helvetica();
    make(&mut once);
    let mut redone = helvetica();
    make(&mut redone);
    redone.undo().unwrap();
    redone.redo().unwrap();
    assert_eq!(saved(&mut redone), saved(&mut once));
}

#[test]
fn a_style_that_changes_nothing_should_be_refused() {
    let mut doc = helvetica();
    let op = style(&doc, "after");
    assert_eq!(apply(&mut doc, op).unwrap_err().code(), "invalid-op");
}

#[test]
fn overlapping_runs_should_be_refused() {
    let mut doc = helvetica();
    let hello = run(&doc, "Hello world");
    let part = TextRunRef::new(hello.object, 2, 5);
    let mut op = SetTextStyle::new(page(&doc), vec![hello, part]);
    op.bold = Some(true);
    assert_eq!(apply(&mut doc, op).unwrap_err().code(), "invalid-op");
    assert!(!doc.is_dirty());
}

#[test]
fn a_stale_run_should_be_refused() {
    let mut doc = helvetica();
    let hello = run(&doc, "Hello world");
    let mut first = style(&doc, "below");
    first.size = Some(30.0);
    apply(&mut doc, first).unwrap();
    let mut op = SetTextStyle::new(page(&doc), vec![hello]);
    op.bold = Some(true);
    assert_eq!(apply(&mut doc, op).unwrap_err().code(), "content-object-stale");
}

#[test]
fn several_runs_of_one_object_should_all_change() {
    let mut doc = helvetica();
    let runs = vec![run(&doc, "Hello world"), run(&doc, "below")];
    let mut op = SetTextStyle::new(page(&doc), runs);
    op.color = Some(AnnotColor::Gray(0.5));
    apply(&mut doc, op).unwrap();
    assert_eq!(glyphs(&doc, "Hello world")[0].color, Some([0.5, 0.5, 0.5]));
    assert_eq!(glyphs(&doc, "below")[0].color, Some([0.5, 0.5, 0.5]));
    assert_eq!(glyphs(&doc, "after")[0].color, Some([0.0, 0.0, 0.0]));
}

#[test]
fn a_run_set_in_a_type3_font_should_be_refused_with_its_reason() {
    let font = b"<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1 1] /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << >> /Encoding << /Differences [72 /H] >> /FirstChar 72 /LastChar 72 /Widths [500] >>";
    let mut doc = page_with(b"BT /F1 12 Tf 20 150 Td (H) Tj ET\n", font);
    let mut op = SetTextStyle::new(page(&doc), vec![text_runs(&doc, &page(&doc)).unwrap()[0].run]);
    op.bold = Some(true);
    assert_eq!(apply(&mut doc, op).unwrap_err().code(), "content-not-editable");
}
