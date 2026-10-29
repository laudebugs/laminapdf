//! Release-mode save benchmarks for the nightly lane (ADR 0018 tier 11), next to the read
//! benchmarks in `lamina-cos`: a full rewrite of a 1,000-page document with the default layout
//! (object streams + xref stream) and with a classic table, and the incremental writer on the same
//! document with nothing changed (it copies the base). `cargo xtask bench` runs them and keeps
//! the history; a regression over 10% opens an issue rather than failing the build.
//!
//! ```sh
//! cargo bench -p lamina-save --bench save
//! ```
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    missing_docs,
    reason = "a benchmark harness: failing loudly is the right behaviour; criterion's macros \
              generate undocumented items"
)]

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use lamina_cos::{Dict, Document, Object, OpenOptions};
use lamina_save::{FullWriter, IncrementalWriter, SaveOptions, Writer, XrefKind};

/// A classic file with 1,000 pages sharing one indirect `/Resources`.
fn thousand_pages() -> Vec<u8> {
    let mut w = Writer::new(1, 7);
    let catalog = w.reserve();
    let pages = w.reserve();
    let resources: Dict = [("ProcSet", Object::Array(vec![Object::name("PDF")]))]
        .into_iter()
        .collect();
    let resources = w.add(resources);
    let kids: Vec<Object> = (0..1000)
        .map(|_| {
            let page: Dict = [
                ("Type", Object::name("Page")),
                ("Parent", pages.into()),
                ("Resources", resources.into()),
            ]
            .into_iter()
            .collect();
            w.add(page).into()
        })
        .collect();
    let pages_dict: Dict = [
        ("Type", Object::name("Pages")),
        ("Count", 1000.into()),
        (
            "MediaBox",
            Object::Array(vec![0.into(), 0.into(), 612.into(), 792.into()]),
        ),
        ("Kids", Object::Array(kids)),
    ]
    .into_iter()
    .collect();
    w.set(pages, pages_dict);
    let catalog_dict: Dict = [("Type", Object::name("Catalog")), ("Pages", pages.into())]
        .into_iter()
        .collect();
    w.set(catalog, catalog_dict);
    let trailer: Dict = [("Root", catalog)].into_iter().collect();
    w.finish(&trailer).unwrap()
}

fn save(c: &mut Criterion) {
    let bytes = thousand_pages();
    let doc = Document::open(bytes.clone(), OpenOptions::default()).unwrap();
    let mut group = c.benchmark_group("save");
    group.throughput(Throughput::Bytes(bytes.len() as u64));
    group.bench_function("full_1000_pages", |b| {
        b.iter(|| {
            let mut out = Vec::new();
            FullWriter::new(black_box(&doc), SaveOptions::new())
                .write(&mut out)
                .unwrap();
            black_box(out)
        });
    });
    group.bench_function("full_classic_1000_pages", |b| {
        b.iter(|| {
            let mut out = Vec::new();
            FullWriter::new(black_box(&doc), SaveOptions::new().with_xref(XrefKind::Classic))
                .write(&mut out)
                .unwrap();
            black_box(out)
        });
    });
    group.bench_function("incremental_unchanged_1000_pages", |b| {
        b.iter(|| {
            let mut out = Vec::new();
            black_box(
                IncrementalWriter::new(black_box(&doc), SaveOptions::new())
                    .write(&mut out)
                    .unwrap(),
            );
            black_box(out)
        });
    });
    group.finish();
}

criterion_group!(benches, save);
criterion_main!(benches);
