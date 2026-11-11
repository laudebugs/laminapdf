//! Markdown through the façade (todo T4d): the `markdown_to_pdf` example's Arabic and Hindi
//! paragraphs read back as written in `fixtures/markdown/report.md`. `tests/error_contract.rs`
//! checks that Markdown warnings carry no Markdown text.
#![cfg(feature = "markdown")]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/markdown_to_pdf.rs"]
mod markdown_to_pdf;

use laminapdf::extract::TextOptions;
use laminapdf::{BuildOptions, Document, OpenOptions};
use laminapdf_test_fixtures::fixture;

const FONTS: [&str; 4] = [
    "fonts/NotoSans-Regular.ttf",
    "fonts/NotoSans-RegularBold.ttc",
    "fonts/NotoSansArabic-Regular.ttf",
    "fonts/NotoSansDevanagari-Regular.ttf",
];

fn report() -> String {
    String::from_utf8(fixture("markdown/report.md")).unwrap()
}

#[test]
fn the_arabic_and_hindi_paragraphs_read_back_as_written() {
    let markdown = report();
    let bytes = markdown_to_pdf::build(
        BuildOptions::new(),
        &markdown,
        FONTS.map(fixture),
        fixture("images/photo-rgb.jpg"),
    )
    .unwrap();
    let doc = Document::open_bytes(bytes, OpenOptions::default()).unwrap();
    let text: Vec<String> = (0..doc.page_count().unwrap())
        .flat_map(|i| doc.page_text(i, &TextOptions::new()).unwrap().lines())
        .collect();
    let bare = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    for range in ['\u{600}'..='\u{6FF}', '\u{900}'..='\u{97F}'] {
        let script = |s: &&str| s.chars().any(|c| range.contains(&c));
        let written: Vec<&str> = markdown.lines().filter(script).collect();
        assert_eq!(written.len(), 1);
        let read: String = text.iter().map(String::as_str).filter(script).collect();
        assert_eq!(bare(&read), bare(written[0]));
    }
}
