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
    // A page that draws a form with a layer that is off: the summary names the copied group in
    // its /OCProperties.
    let form = b"<< /Type /XObject /Subtype /Form /BBox [0 0 100 100] /Resources << /Properties << /Off 6 0 R >> >> /Length 31 >>\nstream\n/OC /Off BDC 0 0 m 9 9 l S EMC\nendstream";
    let objects: [&[u8]; 8] = [
        b"<< /Type /Catalog /Pages 2 0 R /OCProperties << /OCGs [6 0 R] /D << /OFF [6 0 R] >> >> >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 300] /Contents 4 0 R /Resources << /XObject << /Fm 7 0 R >> >> /Annots [5 0 R] >>",
        b"<< /Length 6 >>\nstream\n/Fm Do\nendstream",
        b"<< /Type /Annot /Subtype /Text /Rect [20 260 40 280] /Contents (Layered) >>",
        b"<< /Type /OCG /Name (Off) >>",
        form,
        b"null",
    ];
    let bytes = laminapdf_test_fixtures::classic(&objects, "<< /Size 9 /Root 1 0 R >>");
    let doc = Document::open_bytes(bytes, OpenOptions::new()).unwrap();
    let mut summary = doc.comment_summary(&options()).unwrap();
    let bytes = summary.save(&SaveOptions::new()).unwrap().bytes;
    std::fs::write(dir.join("summary-layers.pdf"), bytes).unwrap();
}

#[test]
fn a_filter_that_matches_nothing_says_so_in_one_line() {
    let options = options().with_comments(vec!["a999.0".parse().unwrap()]);
    let mut summary = review().comment_summary(&options).unwrap();
    let text = text(&mut summary);
    assert_eq!(
        (
            text.contains("None of the chosen comments are in this document."),
            text.contains("0 comments")
        ),
        (true, false)
    );
}

#[test]
fn a_page_not_drawn_is_named_with_its_reason() {
    let doc = Document::open_bytes(fixture("annotations/flatten.pdf"), OpenOptions::new()).unwrap();
    let summary = doc.comment_summary(&options()).unwrap();
    let warning = summary.warnings().next_batch(1).remove(0);
    let diagnostic = serde_json::to_value(&warning).unwrap();
    assert_eq!(
        (warning.kind().as_str(), &diagnostic["data"]),
        (
            "unsupported",
            &serde_json::json!({ "page": 0, "reason": "fonts-not-embedded" })
        )
    );
}

/// How long the summary of a page with a very large content stream takes, and how large it is:
/// a measurement, run by hand (`cargo test --release -p laminapdf --test summary -- --ignored
/// --nocapture`), recorded in the todo.
#[test]
#[ignore = "a measurement, run by hand"]
fn a_very_large_page_is_summarised_within_the_limits() {
    let content = b"10 10 m 20 20 l S\n".repeat(1_000_000);
    let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    stream.extend_from_slice(&content);
    stream.extend_from_slice(b"\nendstream");
    let objects: [&[u8]; 5] = [
        b"<< /Type /Catalog /Pages 2 0 R >>",
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 600 800] /Contents 4 0 R /Annots [5 0 R] >>",
        &stream,
        b"<< /Type /Annot /Subtype /Text /Rect [50 700 70 720] /Contents (Large) /T (Ada) >>",
    ];
    let bytes = laminapdf_test_fixtures::classic(&objects, "<< /Size 6 /Root 1 0 R >>");
    let doc = Document::open_bytes(bytes, OpenOptions::new()).unwrap();
    let start = std::time::Instant::now();
    let mut summary = doc.comment_summary(&options()).unwrap();
    let saved = summary.save(&SaveOptions::new()).unwrap().bytes;
    println!(
        "content {} bytes: summary in {} ms, {} bytes",
        content.len(),
        start.elapsed().as_millis(),
        saved.len()
    );
    assert_eq!(summary.warnings().next_batch(10), Vec::<laminapdf::Warning>::new());
}
