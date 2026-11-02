//! Shared helpers for `laminapdf-save`'s tests: opening documents, the legacy update helper,
//! the frozen `hello`/`imaging-demo` goldens built with the spike-era writer (`legacy`, with
//! the spike's content builder, `content`), the edited golden samples (`samples`) and the
//! overlay-writer helpers (`save`). Fixtures and hand-built classic files come from
//! `laminapdf-test-fixtures`.
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

pub mod content;
pub mod fuzz_entry;
pub mod legacy;
pub mod roundtrip;
pub mod samples;
pub mod save;

use laminapdf_cos::{Document, ObjRef, Object, OpenOptions, Result};
use laminapdf_save::{XrefEol, incremental_update};
pub use laminapdf_test_fixtures::{classic, classic_with_root, corpus, fixture, fixtures_dir, stream_body};

/// Opens a copy of `bytes` with default options (tests keep their inputs).
pub fn open(bytes: &[u8]) -> Result<Document> {
    Document::open(bytes.to_vec(), OpenOptions::default())
}

/// Opens `bytes` with `options`.
pub fn open_with(bytes: &[u8], options: OpenOptions) -> Result<Document> {
    Document::open(bytes.to_vec(), options)
}

/// Opens `base` and appends an incremental update to it.
pub fn update(base: &[u8], changes: &[(ObjRef, Object)], eol: XrefEol) -> Result<Vec<u8>> {
    incremental_update(&open(base)?, changes, eol)
}
