//! Shared helpers for lamina-ops' integration tests: fixtures, small hand-built files and
//! overlay snapshots.
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

use std::path::{Path, PathBuf};

use lamina_cos::{Dict, OverlayEntry};
use lamina_model::{PageId, Rotation};
use lamina_ops::{Document, InfoPatch, Origin, RotatePage, SetInfo};

/// The repository's shared `fixtures/` directory.
pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

pub fn fixture(rel: &str) -> Vec<u8> {
    let path = fixtures_dir().join(rel);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

pub fn fixture_text(rel: &str) -> String {
    String::from_utf8(fixture(rel)).unwrap()
}

pub fn open(bytes: &[u8]) -> Document {
    Document::open(bytes.to_vec(), lamina_cos::OpenOptions::default()).unwrap()
}

pub fn corpus(name: &str) -> Document {
    open(&fixture(&format!("corpus/{name}")))
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

/// Three pages under a `/Pages` node with `/Rotate 90` (pages 4 and 5 inherit it, page 6
/// has its own `/Rotate 180`), and an Info dictionary (object 3) with three entries.
pub fn inherited_rotate() -> Document {
    open(&classic(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [4 0 R 5 0 R 6 0 R] /Count 3 /Rotate 90 /MediaBox [0 0 100 100] >>",
            "<< /Title (Base) /Author (Someone) /Subject (Things) >>",
            "<< /Type /Page /Parent 2 0 R >>",
            "<< /Type /Page /Parent 2 0 R >>",
            "<< /Type /Page /Parent 2 0 R /Rotate 180 >>",
        ],
        "/Info 3 0 R",
    ))
}

/// The overlay and the overlay's trailer, for comparing states.
pub fn snapshot(doc: &Document) -> (Vec<(u32, OverlayEntry)>, Option<Dict>) {
    (
        doc.overlay().iter().map(|(n, e)| (n, e.clone())).collect(),
        doc.overlay().trailer().cloned(),
    )
}

pub fn page_ids(doc: &lamina_model::Document) -> Vec<PageId> {
    doc.pages().unwrap().iter().map(|p| p.id().clone()).collect()
}

pub fn rotations(doc: &lamina_model::Document) -> Vec<Rotation> {
    doc.pages().unwrap().iter().map(|p| p.rotate().unwrap()).collect()
}

pub fn title(doc: &lamina_model::Document) -> Option<String> {
    doc.info().unwrap().and_then(|i| i.title().unwrap())
}

pub fn author(doc: &lamina_model::Document) -> Option<String> {
    doc.info().unwrap().and_then(|i| i.author().unwrap())
}

pub fn set_title(doc: &mut Document, origin: Origin, title: Option<&str>) {
    doc.apply_all(origin, [SetInfo::new(InfoPatch::new().title(title), None).into()])
        .unwrap();
}

pub fn rotate(doc: &mut Document, page: &PageId, rotate: Rotation) {
    doc.apply_all(Origin::Local, [RotatePage::new(page.clone(), rotate).into()])
        .unwrap();
}
