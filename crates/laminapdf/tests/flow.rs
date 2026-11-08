//! Flowing layout through the façade (todo T4b), where building and extraction meet: the
//! 12-page example's running header and "Page n of 12" footer read back on every page but the
//! cover, and `Document::extract` drops them; every paragraph is one `P` element however many
//! pages it crosses; and the spike's report keeps Typst's tagging (one element per paragraph,
//! `/Subtype /Header` and `/Footer`) and reads back as written.
#![cfg(feature = "text")]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

#[expect(dead_code, reason = "the example's own `main` is not used here")]
#[path = "../examples/flowing_report.rs"]
mod flowing_report;
#[path = "common/spike_report.rs"]
mod spike_report;

use laminapdf::extract::{DropReason, ExtractOptions, TextOptions};
use laminapdf::{BuildOptions, Document, OpenOptions, SaveOptions};
use laminapdf_model::{StructKid, StructTree};
use laminapdf_test_fixtures::fixture;

const TOPICS: [&str; 5] = ["Opening files", "Search", "Printing", "Signing", "What comes next"];
const FONTS: [&str; 4] = [
    "fonts/NotoSans-Regular.ttf",
    "fonts/NotoSans-RegularBold.ttc",
    "fonts/NotoSansArabic-Regular.ttf",
    "fonts/NotoSansDevanagari-Regular.ttf",
];

fn example() -> Vec<u8> {
    flowing_report::build(BuildOptions::new(), FONTS.map(fixture), fixture("images/photo-rgb.jpg")).unwrap()
}

fn lines(doc: &Document) -> Vec<Vec<String>> {
    (0..doc.page_count().unwrap())
        .map(|i| doc.page_text(i, &TextOptions::new()).unwrap().lines())
        .collect()
}

/// Each `P` element's pages, in its `/K` order, deduplicated.
fn paragraph_pages(bytes: &[u8]) -> Vec<Vec<usize>> {
    let doc = laminapdf_model::Document::open(bytes.to_vec(), OpenOptions::default()).unwrap();
    let pages: Vec<_> = doc.pages().unwrap().iter().map(|p| p.id().clone()).collect();
    let tree: StructTree = doc.structure().unwrap().unwrap();
    tree.elements()
        .into_iter()
        .filter(|e| e.kind == b"P")
        .map(|e| {
            let mut on: Vec<usize> = e
                .kids
                .iter()
                .filter_map(|k| match k {
                    StructKid::Mcid { page, .. } => pages.iter().position(|p| Some(p) == page.as_ref()),
                    _ => None,
                })
                .collect();
            on.dedup();
            on
        })
        .collect()
}

#[test]
fn every_page_after_the_cover_has_its_section_and_page_of_twelve() {
    let doc = Document::open_bytes(example(), OpenOptions::default()).unwrap();
    let pages = lines(&doc);
    assert_eq!(pages.len(), 12);
    assert!(
        !pages[0]
            .iter()
            .any(|l| l.starts_with("Page ") || TOPICS.contains(&l.as_str()))
    );
    for (n, page) in pages.iter().enumerate().skip(1) {
        assert!(TOPICS.contains(&page[0].as_str()), "page {}: {:?}", n + 1, page[0]);
        assert_eq!(page.last().unwrap(), &format!("Page {} of 12", n + 1));
    }
}

#[test]
fn extraction_drops_the_header_and_footer() {
    let doc = Document::open_bytes(example(), OpenOptions::default()).unwrap();
    let extraction = doc.extract(&ExtractOptions::new()).unwrap();
    let blocks: Vec<&str> = extraction.blocks.iter().map(|b| b.text.as_str()).collect();
    assert!(!blocks.iter().any(|t| t.contains(" of 12")), "a footer in the text");
    let dropped = extraction.dropped.iter().filter(|d| d.reason == DropReason::Artifact);
    // A header and a footer on each of the 11 pages after the cover.
    assert_eq!(dropped.count(), 22);
}

#[test]
fn every_paragraph_is_one_element_and_some_cross_pages() {
    let pages = paragraph_pages(&example());
    // The cover's line, four paragraphs per section, the Arabic and the Hindi one.
    assert_eq!(pages.len(), 1 + 4 * TOPICS.len() + 2);
    assert!(
        pages
            .iter()
            .all(|p| p.len() == 1 || p.windows(2).all(|w| w[1] == w[0] + 1))
    );
    assert!(pages.iter().filter(|p| p.len() == 2).count() >= 2);
}

#[test]
fn the_spike_report_keeps_typst_s_tagging_and_reads_back_as_written() {
    let fonts = FONTS.map(fixture);
    let mut doc = spike_report::build(fonts.each_ref().map(Vec::as_slice), true, 32);
    let bytes = doc.save(&SaveOptions::new()).unwrap().bytes;
    // Typst's file has one `P` per paragraph, whatever page breaks cut it.
    let pages = paragraph_pages(&bytes);
    assert_eq!(pages.len(), spike_report::paragraphs(32).len());
    assert!(pages.iter().any(|p| p.len() == 2));
    let doc = Document::open_bytes(bytes, OpenOptions::default()).unwrap();
    let extraction = doc.extract(&ExtractOptions::new()).unwrap();
    let text: String = extraction
        .blocks
        .iter()
        .map(|b| b.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let bare = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    let text = bare(&text);
    // Every paragraph in the Latin script, whole even when a page break splits it (the header
    // and footer between its parts are dropped).
    let missing: Vec<String> = spike_report::paragraphs(32)
        .into_iter()
        .skip(2)
        .filter(|p| !text.contains(&bare(p)))
        .collect();
    assert!(
        missing.is_empty(),
        "{} paragraphs not read back: {:?}",
        missing.len(),
        missing.first()
    );
}
