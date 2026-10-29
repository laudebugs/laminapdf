//! Shared helpers for the integration tests: fixture loading, a seeded PRNG, hand-built files
//! and the content builder (`content`). The writer is `lamina-save`'s (a dev-dependency).
#![allow(
    dead_code,
    unreachable_pub,
    reason = "each test crate uses a different subset of the helpers"
)]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

pub mod content;

use std::path::{Path, PathBuf};

use lamina_cos::{Document, ObjRef, Object, OpenOptions, Result};
use lamina_save::{XrefEol, incremental_update};

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

/// The repository's shared `fixtures/` directory (docs/guidelines/code-organization.md §6).
pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

pub fn fixture(rel: &str) -> Vec<u8> {
    let path = fixtures_dir().join(rel);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

/// Every `corpus/*.pdf` as `(file name, bytes)`, sorted by name.
pub fn corpus() -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<(String, Vec<u8>)> = std::fs::read_dir(fixtures_dir().join("corpus"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "pdf"))
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read(&p).unwrap(),
            )
        })
        .collect();
    files.sort();
    files
}

pub fn expected() -> serde_json::Value {
    serde_json::from_slice(&fixture("corpus/expected.json")).unwrap()
}

/// xorshift64*: deterministic, so failures are reproducible (the TS twin uses a seeded PRNG too).
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    pub fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next_u64() % u64::try_from(n.max(1)).unwrap()).unwrap()
    }
}

/// Drives every read path on an opened document; returns the number of errors seen.
/// Errors are fine — the point is that nothing panics or hangs.
pub fn exercise(doc: &Document) -> usize {
    let mut errors = 0;
    errors += usize::from(doc.catalog().is_err());
    match doc.pages() {
        Ok(pages) => {
            for page in &pages {
                errors += usize::from(doc.page_content(page).is_err());
            }
        }
        Err(_) => errors += 1,
    }
    let nums: Vec<u32> = doc.object_numbers().collect();
    for num in nums {
        errors += usize::from(doc.resolve(ObjRef::new(num, 0)).is_err());
    }
    errors
}

/// A classic file around object bodies numbered `1..`, with a correct xref table.
pub fn classic(objects: &[&[u8]], trailer: &str) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f\r\n", objects.len() + 1).as_bytes());
    for off in offsets {
        out.extend_from_slice(format!("{off:010} 00000 n\r\n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n{trailer}\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    out
}

/// A stream object body: `dict` (without `/Length`) plus `data`, with a correct `/Length`.
pub fn stream_body(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut out = format!("<< {dict} /Length {} >>\nstream\n", data.len()).into_bytes();
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}
