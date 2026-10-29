//! Release-mode benchmarks for the nightly lane (ADR 0018 tier 11): open + page-tree walk,
//! Flate decode and writing. `cargo xtask bench` runs them and keeps the history; a regression
//! over 10% opens an issue rather than failing the build. Save benchmarks join when the save
//! planner lands (B3b); until then `write` measures the `Writer` serialising a document.
//!
//! ```sh
//! cargo bench -p lamina-cos --bench read                  # all, criterion's default budget
//! cargo xtask bench --quick                               # all, short budget, history format
//! ```
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    missing_docs,
    reason = "a benchmark harness: failing loudly is the right behaviour; criterion's macros \
              generate undocumented items"
)]

#[path = "../tests/common/mod.rs"]
mod common;

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use lamina_syntax::filters::{flate_decode, flate_encode};
use lamina_syntax::{DecodeBudget, Limits};

/// Opening a document and walking its page tree: 1,000 pages sharing one `/Resources`, and
/// every file in `fixtures/corpus`.
fn open_and_walk(c: &mut Criterion) {
    let mut group = c.benchmark_group("open_walk");
    let (pages_1000, _) = common::shared_resources_pdf(1000).unwrap();
    group.throughput(Throughput::Bytes(pages_1000.len() as u64));
    group.bench_function("shared_resources_1000_pages", |b| {
        b.iter(|| {
            let doc = common::open(black_box(&pages_1000)).unwrap();
            black_box(doc.pages().unwrap().len())
        });
    });
    let corpus = common::corpus();
    group.throughput(Throughput::Bytes(
        corpus.iter().map(|(_, bytes)| bytes.len() as u64).sum(),
    ));
    group.bench_function("fixtures_corpus", |b| {
        b.iter(|| {
            for (_, bytes) in &corpus {
                // Some corpus files are deliberately broken: an error is a result too.
                if let Ok(doc) = common::open(black_box(bytes)) {
                    black_box(doc.pages().map(|p| p.len()).ok());
                }
            }
        });
    });
    group.finish();
}

/// Flate-decoding 4 MiB of compressible content-stream-like text.
fn decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode");
    let plain: Vec<u8> = b"BT /F1 12 Tf 72 712 Td (Hello, lamina) Tj ET 0.5 0 0 RG 10 10 m 200 200 l S\n"
        .iter()
        .copied()
        .cycle()
        .take(4 << 20)
        .collect();
    let encoded = flate_encode(&plain);
    let limits = Limits::default();
    group.throughput(Throughput::Bytes(plain.len() as u64));
    group.bench_function("flate_4mib", |b| {
        b.iter(|| {
            let mut budget = DecodeBudget::new(&limits);
            black_box(flate_decode(black_box(&encoded), &limits, &mut budget).unwrap());
        });
    });
    group.finish();
}

/// Serialising a 1,000-page document with the `Writer` (until the save planner, B3b).
fn write(c: &mut Criterion) {
    let mut group = c.benchmark_group("write");
    group.bench_function("shared_resources_1000_pages", |b| {
        b.iter(|| black_box(common::shared_resources_pdf(black_box(1000)).unwrap()));
    });
    group.finish();
}

criterion_group!(benches, open_and_walk, decode, write);
criterion_main!(benches);
