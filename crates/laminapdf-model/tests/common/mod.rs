//! Shared helpers for laminapdf-model's integration tests: opening documents, the differential
//! mutants and `exercise`, which drives every read path and view. Fixtures, hand-built classic
//! files and the PRNG come from `laminapdf-test-fixtures`.
#![allow(
    dead_code,
    unused_imports,
    unreachable_pub,
    reason = "each test crate uses a different subset of the helpers"
)]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

pub mod differential;
pub mod exercise;
pub mod fuzz_entry;
pub mod mutants;

use laminapdf_model::{Document, OpenOptions, Result};
pub use laminapdf_test_fixtures::{Rng, classic_with_root, corpus, fixture, fixtures_dir, json};

/// Opens a copy of `bytes` with default options.
pub fn open(bytes: &[u8]) -> Result<Document> {
    Document::open(bytes.to_vec(), OpenOptions::default())
}

/// The outline titles depth-first, as the corpus facts list them.
pub fn outline_titles(doc: &Document) -> Result<Vec<String>> {
    doc.outline()?.iter().map(|item| item.title()).collect()
}
