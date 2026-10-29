//! Shared helpers for lamina-model's integration tests: fixtures, a seeded PRNG, the
//! differential mutants and `exercise`, which drives every read path and view.
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

pub mod mutants;

use std::path::{Path, PathBuf};

use lamina_cos::ObjRef;
use lamina_model::{Document, OpenOptions, Result};

/// Opens a copy of `bytes` with default options.
pub fn open(bytes: &[u8]) -> Result<Document> {
    Document::open(bytes.to_vec(), OpenOptions::default())
}

/// The repository's shared `fixtures/` directory.
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

pub fn json(rel: &str) -> serde_json::Value {
    serde_json::from_slice(&fixture(rel)).unwrap()
}

/// The outline titles depth-first, as the corpus facts list them.
pub fn outline_titles(doc: &Document) -> Result<Vec<String>> {
    doc.outline()?.iter().map(|item| item.title()).collect()
}

/// A classic file around object bodies numbered `1..`, with a correct xref table.
pub fn classic(objects: &[&str], trailer: &str) -> Vec<u8> {
    let mut out = b"%PDF-1.7\n".to_vec();
    let mut offsets = Vec::new();
    for (i, body) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", i + 1).as_bytes());
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f\r\n", objects.len() + 1).as_bytes());
    for off in offsets {
        out.extend_from_slice(format!("{off:010} 00000 n\r\n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R {trailer} >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

/// Drives every COS read path and every model view; returns the number of errors seen.
/// Errors are fine; the point is that nothing panics or hangs.
pub fn exercise(doc: &Document) -> usize {
    let mut errors = 0;
    let mut count = |r: Result<()>| errors += usize::from(r.is_err());
    count(doc.catalog().map(drop));
    count(views(doc));
    let nums: Vec<u32> = doc.object_numbers().collect();
    for num in nums {
        count(doc.resolve(ObjRef::new(num, 0)).map(drop));
    }
    errors
}

fn views(doc: &Document) -> Result<()> {
    let pages = doc.pages()?;
    for page in pages.iter() {
        let _ = doc.page_content(page.cos());
        page.media_box()?;
        page.crop_box()?;
        page.bleed_box()?;
        page.trim_box()?;
        page.art_box()?;
        page.rotate()?;
        page.user_unit()?;
        page.contents()?;
        page.annots()?;
        page.label()?;
    }
    let outline = doc.outline()?;
    for item in outline.iter() {
        item.title()?;
        item.count()?;
        item.action()?;
        item.target()?;
        let _ = item.children().count();
    }
    if let Some(info) = doc.info()? {
        for key in ["Title", "Author", "Subject", "Keywords", "Creator", "Producer"] {
            info.text(key)?;
        }
        info.creation_date()?;
        info.mod_date()?;
    }
    if let Some(dests) = doc.name_tree("Dests")? {
        dests.entries()?;
        dests.get(b"x")?;
    }
    doc.page_labels()?;
    Ok(())
}

/// xorshift64*: deterministic, so failures are reproducible.
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
