//! `Document::comment_summary` over `fixtures/annotations/review.pdf`: both layouts as PDF/UA-1,
//! the page drawn beside its comments as vector text, numbering in document order, replies and
//! statuses on and off, pages without comments, and a page whose font isn't embedded.
#![cfg(feature = "text")]
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf::extract::TextOptions;
use laminapdf::{
    BuildOptions, Builder, Document, Font, NoteIcon, Op, OpenOptions, PageSize, PointPt, Pt, SaveOptions,
    SummaryLayout, SummaryOptions, SummaryPaper,
};
use laminapdf_test_fixtures::fixture;

fn review() -> Document {
    Document::open_bytes(fixture("annotations/review.pdf"), OpenOptions::new()).unwrap()
}

/// On A4: the fixture's pages are 300 points square, which leaves a narrow column beside them.
fn options() -> SummaryOptions {
    SummaryOptions::new(Font::from_bytes(fixture("fonts/NotoSans-Regular.ttf")).unwrap())
        .with_paper(SummaryPaper::Size(PageSize::A4))
}

/// Every page's text, its runs joined by single spaces, after a save and a reopen.
fn text(summary: &mut Document) -> String {
    let bytes = summary.save(&SaveOptions::new()).unwrap().bytes;
    let doc = Document::open_bytes(bytes, OpenOptions::new()).unwrap();
    (0..doc.page_count().unwrap())
        .map(|i| {
            let page = doc.page_text(i, &TextOptions::default()).unwrap();
            page.runs.iter().map(|r| r.text.as_str()).collect::<Vec<_>>().join(" ")
        })
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn the_page_beside_its_comments_is_vector_text() {
    let mut summary = review().comment_summary(&options()).unwrap();
    let text = text(&mut summary);
    // "Quoted words" twice: drawn on the copied page, and quoted under the highlight.
    assert_eq!(
        (text.matches("Quoted words").count(), text.contains("Struck text")),
        (2, true)
    );
}

#[test]
fn comments_are_numbered_in_document_order_with_their_replies_and_status() {
    let mut summary = review().comment_summary(&options()).unwrap();
    let text = text(&mut summary);
    let at = |needle: &str| text.find(needle).unwrap();
    assert!(at("1. Highlight") < at("Agreed") && at("Agreed") < at("2. Note"));
    assert!(text.contains("Status: Accepted · Marked"), "{text}");
}

#[test]
fn replies_and_status_can_be_left_out() {
    let options = options()
        .with_layout(SummaryLayout::List)
        .with_replies(false)
        .with_status(false);
    let mut summary = review().comment_summary(&options).unwrap();
    let text = text(&mut summary);
    assert!(!text.contains("Agreed") && !text.contains("Status:") && text.contains("Check this"));
}

#[test]
fn the_list_layout_draws_no_page() {
    let options = options().with_layout(SummaryLayout::List);
    let mut summary = review().comment_summary(&options).unwrap();
    assert_eq!(text(&mut summary).matches("Quoted words").count(), 1);
}

#[test]
fn pages_without_comments_are_listed_only_when_asked() {
    let mut b = Builder::new(BuildOptions::new().with_id(*b"summary-empty-pg"));
    b.page(PageSize::A4).finish().unwrap();
    b.page(PageSize::A4).finish().unwrap();
    let mut doc = b.finish().unwrap();
    let note = doc
        .note_at(
            1,
            PointPt::new(Pt::new(56.0), Pt::new(56.0)),
            "Only here",
            NoteIcon::Comment,
        )
        .unwrap();
    doc.apply(vec![Op::from(note)]).unwrap();
    let options = options().with_layout(SummaryLayout::List);
    let without = text(&mut doc.comment_summary(&options).unwrap());
    let with = text(&mut doc.comment_summary(&options.with_empty_pages(true)).unwrap());
    assert_eq!(
        (
            without.contains("No comments on this page."),
            with.contains("No comments on this page.")
        ),
        (false, true)
    );
}

#[test]
fn a_page_whose_font_is_not_embedded_is_listed_without_its_drawing() {
    // Both pages of flatten.pdf set their text in Helvetica, which the file only names.
    let bytes = fixture("annotations/flatten.pdf");
    let doc = Document::open_bytes(bytes, OpenOptions::new()).unwrap();
    let summary = doc.comment_summary(&options()).unwrap();
    let codes: Vec<&str> = summary
        .warnings()
        .next_batch(100)
        .iter()
        .map(laminapdf::Warning::code)
        .collect();
    assert_eq!(codes, ["summary-page-not-drawn", "summary-page-not-drawn"]);
}

#[test]
fn only_the_comments_asked_for_are_listed() {
    let doc = review();
    let first = doc.comment_threads(None).unwrap().remove(0).comment.id;
    let options = options()
        .with_layout(SummaryLayout::List)
        .with_comments(vec![first.parse().unwrap()]);
    let mut summary = doc.comment_summary(&options).unwrap();
    let text = text(&mut summary);
    assert!(text.contains("1. Highlight") && !text.contains("2. "), "{text}");
}

/// Writes both layouts of `review.pdf` into `$LAMINA_PDF_SUMMARY_OUT` for veraPDF
/// (`cargo xtask verapdf --dir <dir>`); does nothing without it.
#[test]
fn both_layouts_are_written_for_verapdf_when_asked() {
    let Some(dir) = std::env::var_os("LAMINA_PDF_SUMMARY_OUT") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    for (name, layout) in [("beside", SummaryLayout::BesidePage), ("list", SummaryLayout::List)] {
        let mut summary = review().comment_summary(&options().with_layout(layout)).unwrap();
        let bytes = summary.save(&SaveOptions::new()).unwrap().bytes;
        std::fs::write(dir.join(format!("summary-{name}.pdf")), bytes).unwrap();
    }
}
