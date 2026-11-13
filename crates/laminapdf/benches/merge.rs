//! Release-mode benchmark of merging for the nightly lane (ADR 0018 tier 11; E2 plan §10): 100
//! files of 10 pages each (the labelled fixture, with its bookmarks, page labels and named
//! destinations) merged by `Document::merge` with a bookmark per file, then saved. The plan's
//! budget is 1 s natively; the time is recorded, not asserted.
//!
//! ```sh
//! cargo bench -p laminapdf --bench merge
//! ```
#![allow(
    clippy::unwrap_used,
    missing_docs,
    reason = "a benchmark harness: failing loudly is the right behaviour; criterion's macros \
              generate undocumented items"
)]

use std::hint::black_box;
use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};
use laminapdf::{Document, MergeInput, MergeOptions, OpenOptions, SaveOptions};

const FILE: &[u8] = include_bytes!("../../../fixtures/pages/labels.pdf");

/// How many copies of the ten-page file are merged.
const FILES: usize = 100;

fn merge(files: &[MergeInput]) -> Vec<u8> {
    let mut merged = Document::merge(files.to_vec(), MergeOptions::default()).unwrap();
    merged.save(&SaveOptions::new()).unwrap().bytes
}

fn merge_100_files(c: &mut Criterion) {
    let bytes: Arc<[u8]> = Arc::from(FILE);
    let files: Vec<MergeInput> = (0..FILES)
        .map(|i| MergeInput::new(Arc::clone(&bytes)).with_title(format!("file-{i}.pdf")))
        .collect();
    let pages = Document::open_bytes(merge(&files), OpenOptions::default())
        .unwrap()
        .page_count()
        .unwrap();
    assert_eq!(usize::try_from(pages).unwrap(), FILES * 10, "every page should be merged");
    let mut group = c.benchmark_group("merge");
    group.sample_size(10);
    group.bench_function("merge_100_files_of_10_pages", |b| {
        b.iter(|| black_box(merge(black_box(&files))));
    });
    group.finish();
}

criterion_group!(benches, merge_100_files);
criterion_main!(benches);
