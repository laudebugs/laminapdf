//! `EditText` and `text_runs` (todo E7e, ADR 0022 §5, §6): the runs of text a page shows, and
//! their text changed in the font they are set in, the rest of the line moved.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::{classic, fixture, open, snapshot, stream_body};
use laminapdf_content::NotEditable;
use laminapdf_model::PageId;
use laminapdf_ops::{ContentTextRun, Document, EditText, Error, Op, Origin, TextRunRef, text_runs};
use laminapdf_save::{Save, SaveOptions};
use laminapdf_text::TextExtractor;

/// Two runs on one line in two sizes, the second placed by a `Td` after the first, and a line below.
const LINE: &[u8] = b"BT /F1 12 Tf 20 150 Td (Hello world) Tj 80 0 Td /F1 10 Tf (after) Tj 0 -20 Td (below) Tj ET\n";

/// A 200 × 200 page drawing `content` with `/F1` the font dictionary `font`.
fn page_with(content: &[u8], font: &[u8]) -> Document {
    let content = stream_body("", content);
    let objects: [&[u8]; 5] = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 200] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
        &content,
        font,
    ];
    open(&classic(&objects, "<< /Size 6 /Root 1 0 R >>"))
}

/// [`LINE`] in Helvetica, not embedded: any character of its encoding can be written.
fn helvetica() -> Document {
    page_with(LINE, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>")
}

fn page(doc: &Document) -> PageId {
    doc.pages().unwrap().get(0).unwrap().id().clone()
}

fn runs(doc: &Document) -> Vec<ContentTextRun> {
    text_runs(doc, &page(doc)).unwrap()
}

/// The run that shows `text`.
fn run(doc: &Document, text: &str) -> TextRunRef {
    runs(doc).into_iter().find(|r| r.text == text).unwrap().run
}

fn edit(doc: &mut Document, run: TextRunRef, text: &str) -> laminapdf_ops::Result<()> {
    let op = EditText::new(page(doc), run, text.to_owned());
    doc.apply_all(Origin::Local, [Op::from(op)]).map(drop)
}

/// Makes the run that shows `run` show `text`.
fn edit_run(doc: &mut Document, shown: &str, text: &str) -> laminapdf_ops::Result<()> {
    let found = run(doc, shown);
    edit(doc, found, text)
}

/// The page's text as extraction reads it, line by line.
fn lines(doc: &Document) -> Vec<String> {
    let pages = doc.pages().unwrap();
    TextExtractor::new().page(doc, &pages.get(0).unwrap()).unwrap().lines()
}

/// Where extraction finds the first glyph of `text`: its origin in user space.
fn origin_of(doc: &Document, text: &str) -> (f64, f64) {
    let pages = doc.pages().unwrap();
    let page = TextExtractor::new().page(doc, &pages.get(0).unwrap()).unwrap();
    let run = page.runs.iter().find(|r| r.text == text).unwrap();
    let glyph = &page.glyphs[run.glyphs.start];
    (glyph.origin.x, glyph.origin.y)
}

fn saved(doc: &mut Document) -> Vec<u8> {
    let mut out = Vec::new();
    doc.save(&mut out, &SaveOptions::default()).unwrap();
    out
}

#[test]
fn the_page_should_list_each_run_with_its_line_and_no_reason() {
    let runs = runs(&helvetica());
    let listed: Vec<(&str, u32, Option<NotEditable>)> =
        runs.iter().map(|r| (r.text.as_str(), r.line, r.read_only)).collect();
    assert_eq!(
        listed,
        [("Hello world", 0, None), ("after", 0, None), ("below", 1, None)]
    );
}

#[test]
fn a_run_should_name_its_glyphs_within_the_text_object() {
    let runs = runs(&helvetica());
    let ranges: Vec<(u32, u32, u32)> = runs
        .iter()
        .map(|r| (r.run.object.ordinal, r.run.start, r.run.end))
        .collect();
    assert_eq!(ranges, [(0, 0, 11), (0, 11, 16), (0, 16, 21)]);
}

#[test]
fn replacing_a_word_should_read_back_as_typed() {
    let mut doc = helvetica();
    let hello = run(&doc, "Hello world");
    edit(
        &mut doc,
        TextRunRef::new(hello.object, hello.start + 6, hello.end),
        "there",
    )
    .unwrap();
    assert_eq!(lines(&doc), ["Hello there after", "below"]);
}

#[test]
fn longer_text_should_move_the_rest_of_the_line_and_leave_the_line_below() {
    let mut doc = helvetica();
    let (after, below) = (origin_of(&doc, "after"), origin_of(&doc, "below"));
    edit_run(&mut doc, "Hello world", "Hello, wide world").unwrap();
    let (moved, stayed) = (origin_of(&doc, "after"), origin_of(&doc, "below"));
    assert!(moved.0 > after.0 + 20.0, "{after:?} → {moved:?}");
    assert!((moved.1 - after.1).abs() < 1e-6);
    assert!(
        (stayed.0 - below.0).abs() < 1e-6 && (stayed.1 - below.1).abs() < 1e-6,
        "{below:?} → {stayed:?}"
    );
}

#[test]
fn empty_text_should_delete_the_run() {
    let mut doc = helvetica();
    edit_run(&mut doc, "after", "").unwrap();
    assert_eq!(lines(&doc), ["Hello world", "below"]);
}

#[test]
fn a_character_the_font_cannot_draw_should_be_a_missing_glyph() {
    let mut doc = helvetica();
    let error = edit_run(&mut doc, "after", "日本").unwrap_err();
    assert_eq!(error.code(), "missing-glyph");
    assert!(!doc.is_dirty());
}

/// A subset (`ABCDEF+`) holds only the glyphs the document draws, so only those can be used.
#[test]
fn a_subset_should_take_only_the_characters_the_page_already_shows() {
    let font = b"<< /Type /Font /Subtype /Type1 /BaseFont /ABCDEF+Helvetica >>";
    let mut doc = page_with(LINE, font);
    edit_run(&mut doc, "after", "tear").unwrap();
    assert_eq!(lines(&doc), ["Hello world tear", "below"]);
    let error = edit_run(&mut doc, "tear", "zebra").unwrap_err();
    assert_eq!(error.code(), "missing-glyph");
}

#[test]
fn a_type3_run_should_be_listed_read_only() {
    let font = b"<< /Type /Font /Subtype /Type3 /FontBBox [0 0 1 1] /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << >> /Encoding << /Differences [72 /H] >> /FirstChar 72 /LastChar 72 /Widths [500] >>";
    let doc = page_with(b"BT /F1 12 Tf 20 150 Td (H) Tj ET\n", font);
    assert_eq!(
        runs(&doc).iter().map(|r| r.read_only).collect::<Vec<_>>(),
        [Some(NotEditable::Type3Font)]
    );
}

#[test]
fn editing_a_read_only_run_should_say_why() {
    let mut clipped = page_with(
        b"BT /F1 12 Tf 7 Tr 20 150 Td (Hello world) Tj ET\n",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    );
    let hello = runs(&clipped)[0].run;
    assert_eq!(
        edit(&mut clipped, hello, "Hi"),
        Err(Error::ContentNotEditable {
            reason: NotEditable::ClippingPath
        })
    );
}

#[test]
fn a_stale_run_should_be_refused() {
    let mut doc = helvetica();
    let hello = run(&doc, "Hello world");
    edit_run(&mut doc, "below", "under").unwrap();
    assert_eq!(
        edit(&mut doc, hello, "Hi").map_err(|e| e.code()),
        Err("content-object-stale")
    );
}

#[test]
fn glyphs_outside_the_object_should_be_refused() {
    let mut doc = helvetica();
    let hello = run(&doc, "Hello world");
    let past = TextRunRef::new(hello.object, 20, 30);
    assert_eq!(edit(&mut doc, past, "x").map_err(|e| e.code()), Err("invalid-op"));
}

#[test]
fn edit_text_should_be_class_content_and_save_incrementally() {
    let doc = helvetica();
    let op = Op::from(EditText::new(page(&doc), run(&doc, "after"), String::new()));
    assert_eq!(op.class(), laminapdf_ops::ModificationClass::Content);
    assert!(!op.requires_rewrite());
}

#[test]
fn undoing_an_edit_should_leave_the_overlay_empty_and_the_save_unchanged() {
    let before = saved(&mut helvetica());
    let mut doc = helvetica();
    let empty = snapshot(&doc);
    edit_run(&mut doc, "Hello world", "Goodbye").unwrap();
    doc.undo().unwrap();
    assert_eq!(snapshot(&doc), empty);
    assert_eq!(saved(&mut doc), before);
}

#[test]
fn redoing_an_edit_should_write_the_same_bytes_again() {
    let mut once = helvetica();
    edit_run(&mut once, "Hello world", "Goodbye").unwrap();
    let mut redone = helvetica();
    edit_run(&mut redone, "Hello world", "Goodbye").unwrap();
    redone.undo().unwrap();
    redone.redo().unwrap();
    assert_eq!(saved(&mut redone), saved(&mut once));
}

#[test]
fn an_edit_should_say_the_earlier_revision_still_holds_the_old_text() {
    let mut doc = helvetica();
    let op = EditText::new(page(&doc), run(&doc, "after"), "later".to_owned());
    let applied = doc.apply_all(Origin::Local, [Op::from(op)]).unwrap();
    assert!(applied.still_in_earlier_revisions);
}

/// The frozen wire sample (`fixtures/ops/v1/edit-text.json`), written for [`helvetica`]'s page,
/// applies: "world" becomes "there".
#[test]
fn the_frozen_sample_applies_to_the_page_it_was_written_for() {
    let mut doc = helvetica();
    let op: Op = serde_json::from_slice(&fixture("ops/v1/edit-text.json")).unwrap();
    doc.apply_all(Origin::Local, [op]).unwrap();
    assert_eq!(lines(&doc), ["Hello there after", "below"]);
}

/// Each show operator draws its own run, so an edit never spans the moves between them.
#[test]
fn a_run_should_end_where_its_show_operator_ends() {
    let doc = page_with(
        b"BT /F1 10 Tf 20 150 Td (Hi) Tj 30 0 Td (there) Tj ET\n",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    );
    let texts: Vec<String> = runs(&doc).into_iter().map(|r| r.text).collect();
    assert_eq!(texts, ["Hi", "there"]);
}

#[test]
fn glyphs_of_two_show_operators_should_be_refused() {
    let mut doc = page_with(
        b"BT /F1 10 Tf 20 150 Td (Hi) Tj 30 0 Td (there) Tj ET\n",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    );
    let hi = run(&doc, "Hi");
    let both = TextRunRef::new(hi.object, 0, 7);
    assert_eq!(edit(&mut doc, both, "Hello").map_err(|e| e.code()), Err("invalid-op"));
}

/// A `TJ` number that makes a word space reads as a space, and a space the subset can't draw is
/// written back as a number.
#[test]
fn a_word_space_made_by_a_number_should_read_and_write_as_a_space() {
    let font = b"<< /Type /Font /Subtype /Type1 /BaseFont /ABCDEF+Helvetica >>";
    let mut doc = page_with(b"BT /F1 12 Tf 20 150 Td [(Hello) -333 (world)] TJ ET\n", font);
    assert_eq!(runs(&doc)[0].text, "Hello world");
    edit_run(&mut doc, "Hello world", "Hello low").unwrap();
    assert_eq!(lines(&doc), ["Hello low"]);
}

#[test]
fn text_standing_in_for_actual_text_should_be_read_only() {
    let doc = page_with(
        b"BT /F1 12 Tf 20 150 Td /Span <</ActualText (Total)>> BDC (Tot.) Tj EMC ET\n",
        b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    );
    assert_eq!(runs(&doc)[0].read_only, Some(NotEditable::ActualText));
}

#[test]
fn right_to_left_text_should_be_read_only() {
    let font = b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding << /Differences [65 /afii57664 /afii57665] >> >>";
    let doc = page_with(b"BT /F1 12 Tf 20 150 Td (AB) Tj ET\n", font);
    assert_eq!(runs(&doc)[0].read_only, Some(NotEditable::RightToLeft));
}

/// A code past the font's `/Widths` would be drawn with no advance, so it isn't offered.
#[test]
fn a_code_without_a_width_should_be_a_missing_glyph() {
    let widths = vec!["500"; 34].join(" ");
    let font = format!(
        "<< /Type /Font /Subtype /Type1 /BaseFont /Arial /FirstChar 72 /LastChar 105 /Widths [{widths}] /FontDescriptor << /Type /FontDescriptor /FontName /Arial /Flags 32 /MissingWidth 0 >> >>"
    );
    let mut doc = page_with(b"BT /F1 12 Tf 20 150 Td (Hi) Tj ET\n", font.as_bytes());
    assert_eq!(
        edit_run(&mut doc, "Hi", "Hi~").map_err(|e| e.code()),
        Err("missing-glyph")
    );
    edit_run(&mut doc, "Hi", "IH").unwrap();
}
