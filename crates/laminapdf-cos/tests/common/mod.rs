//! Shared helpers for the integration tests: opening documents, a windowed source and files
//! built with `laminapdf-save`'s writer (a dev-dependency). Fixtures, hand-built classic files
//! and the PRNG come from `laminapdf-test-fixtures`.
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

pub use laminapdf_test_fixtures::{Rng, classic, codes, corpus, fixture, fixtures_dir, read_at, stream_body};

use laminapdf_cos::{ByteSource, Dict, Document, ObjRef, Object, OpenOptions, Result, SourceError};
use laminapdf_save::{Writer, XrefEol, incremental_update};

/// A source that is not in memory (no `as_slice`), so the reader parses it in windows.
pub struct NoSlice(pub Vec<u8>);

impl ByteSource for NoSlice {
    fn len(&self) -> u64 {
        u64::try_from(self.0.len()).unwrap()
    }

    fn read_at(&self, offset: u64, buf: &mut [u8]) -> std::result::Result<usize, SourceError> {
        Ok(read_at(&self.0, offset, buf))
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

pub fn expected() -> serde_json::Value {
    laminapdf_test_fixtures::json("corpus/expected.json")
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
