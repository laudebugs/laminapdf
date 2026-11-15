//! Release-mode benchmark for the nightly lane (ADR 0018 tier 11): building the editable content
//! IR (ADR 0022: tokenizing every page's content and grouping it into objects) and writing it
//! back, over the decoded page content of every fixture. Criterion reports the throughput; the
//! E7a spike measured 67 MB/s on its corpus, and E7c's goal is at least 50 MB/s.
//!
//! ```sh
//! cargo bench -p laminapdf-content --bench ir
//! ```
#![allow(
    clippy::unwrap_used,
    missing_docs,
    reason = "a benchmark harness: failing loudly is the right behaviour; criterion's macros \
              generate undocumented items"
)]

use std::hint::black_box;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use laminapdf_content::{ContentOps, ContentOwner};
use laminapdf_cos::Limits;
use laminapdf_model::{Document, OpenOptions};
use laminapdf_test_fixtures::fixtures_dir;

/// Every fixture page's decoded content, one entry per page.
fn pages() -> Vec<Vec<u8>> {
    let mut pages = Vec::new();
    for dir in ["corpus", "golden", "golden/examples", "content"] {
        let mut files: Vec<_> = std::fs::read_dir(fixtures_dir().join(dir))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|x| x == "pdf"))
            .collect();
        files.sort();
        for path in files {
            let Ok(doc) = Document::open(std::fs::read(path).unwrap(), OpenOptions::default()) else {
                continue;
            };
            let Ok(list) = doc.pages() else { continue };
            pages.extend(list.iter().filter_map(|page| doc.page_content(page.cos()).ok()));
        }
    }
    pages
}

fn build_and_encode(c: &mut Criterion) {
    let pages = pages();
    let owner = ContentOwner::Page("p1.0".parse().unwrap());
    let limits = Limits::default();
    let mut group = c.benchmark_group("content_ir");
    group.throughput(Throughput::Bytes(pages.iter().map(|p| p.len() as u64).sum()));
    group.bench_function("build_fixtures", |b| {
        b.iter(|| {
            for page in &pages {
                let ops = ContentOps::from_parts(owner.clone(), vec![(None, page.clone())], &limits);
                black_box(ops.map(|o| o.objects().len()).ok());
            }
        });
    });
    let built: Vec<_> = pages
        .iter()
        .filter_map(|p| ContentOps::from_parts(owner.clone(), vec![(None, p.clone())], &limits).ok())
        .collect();
    group.bench_function("encode_fixtures", |b| {
        b.iter(|| {
            for ops in &built {
                black_box(ops.encode_every_part());
            }
        });
    });
    group.finish();
}

criterion_group!(benches, build_and_encode);
criterion_main!(benches);
