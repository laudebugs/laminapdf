//! Flowing tables (todo T4c), on `fixtures/golden/flow-table.pdf`'s document: the 60-row
//! table runs over three pages with its header row at the top of each; no row is on two
//! pages; the totals are on the last row's page; one `Table` element with one `THead`, its
//! `TH`s with `/Scope /Column`; the header rows on pages 2 and 3 inside `Pagination`
//! artifacts, so a reader that leaves artifacts out reads the header once.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::flow_table::{HEADER, ROWS, TOTAL, flow_table};
use common::reopen;
use laminapdf_text::TextExtractor;

/// Each page's runs as (text, inside an artifact), in content order. A run can join cells
/// side by side on one baseline.
fn runs(bytes: &[u8]) -> Vec<Vec<(String, bool)>> {
    let doc = reopen(bytes);
    let mut extractor = TextExtractor::new();
    doc.pages()
        .unwrap()
        .iter()
        .map(|page| {
            let text = extractor.page(&doc, &page).unwrap();
            text.runs
                .iter()
                .map(|r| (r.text.trim().to_owned(), r.artifact))
                .collect()
        })
        .collect()
}

/// The item numbers ("001" to "060") on each page, read from the content.
fn items(pages: &[Vec<(String, bool)>]) -> Vec<Vec<usize>> {
    pages
        .iter()
        .map(|runs| {
            runs.iter()
                .filter(|(t, _)| t.len() == 3 && t.bytes().all(|b| b.is_ascii_digit()))
                .map(|(t, _)| t.parse().unwrap())
                .collect()
        })
        .collect()
}

#[test]
fn the_table_runs_over_three_pages_with_its_header_on_each_and_the_totals_on_the_last() {
    let pages = runs(&flow_table());
    assert_eq!(pages.len(), 3);
    for (n, page) in pages.iter().enumerate() {
        let header = page.iter().find(|(t, _)| t.contains(HEADER[1])).unwrap();
        // The header is the document's on page 1 and an artifact after it.
        assert_eq!(header.1, n > 0, "page {}", n + 1);
    }
    let numbers = items(&pages);
    // Every row once, in order, each on one page.
    assert_eq!(numbers.concat(), (1..=ROWS).collect::<Vec<_>>());
    assert!(numbers.iter().all(|p| !p.is_empty()));
    let totals: Vec<usize> = (0..3).filter(|&n| pages[n].iter().any(|(t, _)| t == TOTAL)).collect();
    assert_eq!(totals, [2], "the totals are on the last row's page");
}

#[test]
fn the_table_is_one_element_with_one_header_its_cells_scoped_to_their_columns() {
    let bytes = flow_table();
    let tree = reopen(&bytes).structure().unwrap().unwrap();
    let count = |kind: &[u8]| tree.elements().into_iter().filter(|e| e.kind == kind).count();
    assert_eq!(
        [b"Table".as_slice(), b"THead", b"TBody", b"TFoot", b"TH"].map(count),
        [1, 1, 1, 1, HEADER.len()]
    );
    // The header row, the body rows and the totals row.
    assert_eq!(count(b"TR"), ROWS + 2);
    assert_eq!(count(b"TD"), ROWS * HEADER.len() + 2);
    let table = tree.elements().into_iter().find(|e| e.kind == b"Table").unwrap();
    let parts: Vec<&[u8]> = table.children().map(|e| e.kind.as_slice()).collect();
    assert_eq!(parts, [b"THead".as_slice(), b"TBody", b"TFoot"]);
    let occurrences = |marker: &[u8]| bytes.windows(marker.len()).filter(|w| *w == marker).count();
    assert_eq!(occurrences(b"/Scope /Column"), HEADER.len());
    assert_eq!(occurrences(b"/ColSpan 3"), 1);
}

#[test]
fn the_repeated_header_rows_are_pagination_artifacts_and_rules_are_layout_ones() {
    let pages = runs(&flow_table());
    let content: Vec<usize> = pages
        .iter()
        .map(|p| {
            p.iter()
                .filter(|(_, artifact)| !artifact)
                .flat_map(|(t, _)| t.split_whitespace())
                .filter(|w| HEADER.contains(w))
                .count()
        })
        .collect();
    assert_eq!(content, [HEADER.len(), 0, 0], "the header is content once");
    let bytes = flow_table();
    let occurrences = |marker: &[u8]| bytes.windows(marker.len()).filter(|w| *w == marker).count();
    // Each later page's header cells, one artifact each.
    assert_eq!(
        occurrences(b"/Artifact <<\n  /Type /Pagination\n>> BDC"),
        2 * HEADER.len()
    );
    // The background of each page's header row and each page's rules.
    assert_eq!(occurrences(b"/Artifact <<\n  /Type /Layout\n>> BDC"), 2 * 3);
}

#[test]
fn automatic_columns_keep_their_widths_on_every_page() {
    let doc = reopen(&flow_table());
    let mut extractor = TextExtractor::new();
    // Where each page's description column starts: the left edge of its header cell's first
    // glyph, the page's first "D" in content order.
    let starts: Vec<f64> = doc
        .pages()
        .unwrap()
        .iter()
        .map(|page| {
            let text = extractor.page(&doc, &page).unwrap();
            let glyph = text.glyphs.iter().find(|g| g.text == "D").unwrap();
            (glyph.quad[0].x.get() * 100.0).round() / 100.0
        })
        .collect();
    assert_eq!(starts.len(), 3);
    assert!(starts.windows(2).all(|w| (w[0] - w[1]).abs() < 0.005), "{starts:?}");
}

#[test]
fn the_document_is_the_same_twice() {
    assert_eq!(flow_table(), flow_table());
}
