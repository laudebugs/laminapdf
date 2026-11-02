//! Shared helpers for laminapdf-ops' integration tests: opening documents, small hand-built
//! files and overlay snapshots. Fixtures and the classic-file builder come from
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

use laminapdf_cos::{Dict, OverlayEntry};
use laminapdf_model::{PageId, Rotation};
use laminapdf_ops::{Document, InfoPatch, Origin, RotatePage, SetInfo};
pub use laminapdf_test_fixtures::{classic_with_root, fixture, fixture_text, fixtures_dir};

pub fn open(bytes: &[u8]) -> Document {
    Document::open(bytes.to_vec(), laminapdf_cos::OpenOptions::default()).unwrap()
}

pub fn corpus(name: &str) -> Document {
    open(&fixture(&format!("corpus/{name}")))
}

/// Three pages under a `/Pages` node with `/Rotate 90` (pages 4 and 5 inherit it, page 6
/// has its own `/Rotate 180`), and an Info dictionary (object 3) with three entries.
pub fn inherited_rotate() -> Document {
    open(&classic_with_root(
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

pub fn page_ids(doc: &laminapdf_model::Document) -> Vec<PageId> {
    doc.pages().unwrap().iter().map(|p| p.id().clone()).collect()
}

pub fn rotations(doc: &laminapdf_model::Document) -> Vec<Rotation> {
    doc.pages().unwrap().iter().map(|p| p.rotate().unwrap()).collect()
}

pub fn title(doc: &laminapdf_model::Document) -> Option<String> {
    doc.info().unwrap().and_then(|i| i.title().unwrap())
}

pub fn author(doc: &laminapdf_model::Document) -> Option<String> {
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
