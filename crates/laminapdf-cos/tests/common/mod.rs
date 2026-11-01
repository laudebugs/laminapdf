//! Shared helpers for the integration tests: fixture loading, a seeded PRNG and hand-built
//! files. The writer is `laminapdf-save`'s (a dev-dependency); the content builder moved to
//! `laminapdf-build`'s tests.
#![allow(
    dead_code,
    unused_imports,
    unreachable_pub,
    reason = "each test crate uses a different subset of the helpers"
)]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

pub mod classic;
pub mod codes;

use std::path::{Path, PathBuf};

use laminapdf_cos::{ByteSource, Dict, Document, ObjRef, Object, OpenOptions, Result, SourceError};
use laminapdf_save::{Writer, XrefEol, incremental_update};

/// A source that is not in memory (no `as_slice`), so the reader parses it in windows.
pub struct NoSlice(pub Vec<u8>);

impl ByteSource for NoSlice {
    fn len(&self) -> u64 {
        u64::try_from(self.0.len()).unwrap()
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> std::result::Result<usize, SourceError> {
        let tail = self.0.get(usize::try_from(offset).unwrap()..).unwrap_or_default();
        let n = tail.len().min(buf.len());
        buf[..n].copy_from_slice(&tail[..n]);
        Ok(n)
    }
}

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

pub use classic::{classic, stream_body};

/// A classic file with `page_count` pages, all sharing one indirect `/Resources` (the
/// `reader_cache` scaling test and the `read` benchmarks).
pub fn shared_resources_pdf(page_count: u32) -> Result<(Vec<u8>, ObjRef)> {
    let mut w = Writer::new(1, 7);
    let catalog = w.reserve();
    let pages = w.reserve();
    let font: Dict = [("Type", Object::name("Font")), ("BaseFont", Object::name("Helvetica"))]
        .into_iter()
        .collect();
    let fonts: Dict = [("F1", Object::Dict(font))].into_iter().collect();
    let resources: Dict = [("Font", Object::Dict(fonts))].into_iter().collect();
    let resources = w.add(resources);
    let media_box = Object::Array(vec![0.into(), 0.into(), 612.into(), 792.into()]);
    let kids: Vec<Object> = (0..page_count)
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
        ("Count", page_count.into()),
        ("MediaBox", media_box),
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
    Ok((w.finish(&trailer)?, resources))
}
