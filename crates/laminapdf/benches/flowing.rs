//! Release-mode benchmark of flowing layout for the nightly lane (ADR 0018 tier 11; ADR 0032 §9;
//! todo T4b): the `flow-layout` spike's seeded report, 37 sections of headings and paragraphs
//! for 100 A4 pages (the spike's 32 sections fill 87 without its tables), with an Arabic and a
//! Devanagari paragraph, a running header and a "Page X of Y"
//! footer, laid out by `Builder::flow` and finished. ADR 0032 §9's budget is 250 ms natively;
//! the time is recorded, not asserted.
//!
//! ```sh
//! cargo bench -p laminapdf --bench flowing
//! ```
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    missing_docs,
    reason = "a benchmark harness: failing loudly is the right behaviour; criterion's macros \
              generate undocumented items"
)]

#[path = "../tests/common/spike_report.rs"]
mod spike_report;

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};

const FONTS: [&[u8]; 4] = [
    include_bytes!("../../../fixtures/fonts/NotoSans-Regular.ttf"),
    include_bytes!("../../../fixtures/fonts/NotoSans-RegularBold.ttc"),
    include_bytes!("../../../fixtures/fonts/NotoSansArabic-Regular.ttf"),
    include_bytes!("../../../fixtures/fonts/NotoSansDevanagari-Regular.ttf"),
];

/// Sections for about 100 pages.
const SECTIONS: usize = 37;

fn flowing_report(c: &mut Criterion) {
    let pages = spike_report::build(FONTS, false, SECTIONS).page_count().unwrap();
    assert!(
        (95..=105).contains(&pages),
        "the report should be about 100 pages, not {pages}"
    );
    c.bench_function("flowing_report_100_pages", |b| {
        b.iter(|| black_box(spike_report::build(black_box(FONTS), false, SECTIONS)));
    });
}

criterion_group!(benches, flowing_report);
criterion_main!(benches);
