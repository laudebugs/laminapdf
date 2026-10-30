//! Page labels (todo B5b): `/PageLabels` read back through `lamina-model`'s `PageLabels`,
//! and the ranges the builder refuses.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::b5b::LABELS;
use common::{fixture, reopen, save};
use lamina_build::{BuildOptions, Builder, LabelStyle, PageLabel, PageSize};

#[test]
fn the_golden_labels_read_back() {
    let doc = reopen(&fixture("golden/links-and-outlines.pdf"));
    let labels = doc.page_labels().unwrap();
    let got: Vec<String> = (0..4).map(|i| labels.label(i).unwrap()).collect();
    assert_eq!(got, LABELS);
}

fn build(pages: usize, ranges: Vec<PageLabel>) -> lamina_build::Result<Vec<u8>> {
    let mut b = Builder::new(BuildOptions::default());
    for _ in 0..pages {
        b.page(PageSize::A6).finish()?;
    }
    b.page_labels(ranges);
    b.finish().map(save)
}

#[test]
fn prefixes_and_start_numbers_are_kept() {
    let bytes = build(
        3,
        vec![
            PageLabel::new(0, None).with_prefix("Cover"),
            PageLabel::new(1, Some(LabelStyle::UpperLetters))
                .with_prefix("A-")
                .with_first(3),
        ],
    )
    .unwrap();
    let labels = reopen(&bytes).page_labels().unwrap();
    let got: Vec<String> = (0..3).map(|i| labels.label(i).unwrap()).collect();
    assert_eq!(got, ["Cover", "A-C", "A-D"]);
}

#[test]
fn invalid_ranges_are_refused() {
    let codes: Vec<&str> = [
        vec![],
        vec![PageLabel::new(1, None)],
        vec![PageLabel::new(0, None), PageLabel::new(0, None)],
        vec![PageLabel::new(0, None), PageLabel::new(5, None)],
        vec![PageLabel::new(0, Some(LabelStyle::Decimal)).with_first(0)],
    ]
    .into_iter()
    .map(|ranges| build(2, ranges).unwrap_err().code())
    .collect();
    assert_eq!(codes, ["invalid-page-labels"; 5]);
}
