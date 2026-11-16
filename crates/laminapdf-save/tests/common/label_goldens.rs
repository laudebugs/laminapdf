//! The page-label goldens: files a labels test saves into a folder of `fixtures/pages/`, with
//! the `facts.json` that `tools/fixtures/label_facts.py` writes for them (each file's SHA-256,
//! `qpdf --check`, and the labels pdf.js and PDFium show), so the tests need neither Node nor
//! Python.

use std::path::PathBuf;

use laminapdf_cos::OpenOptions;
use laminapdf_test_fixtures::{fixture, fixtures_dir};

/// Whether this run rewrites the goldens (`UPDATE_GOLDEN=1`); run `label_facts.py` after.
pub fn updating() -> bool {
    std::env::var_os("UPDATE_GOLDEN").is_some()
}

/// Writes `bytes` as `fixtures/pages/<folder>/<name>` when updating; otherwise checks they
/// are that file's bytes.
pub fn check_golden(folder: &str, name: &str, bytes: &[u8]) {
    if updating() {
        let dir: PathBuf = fixtures_dir().join("pages").join(folder);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(name), bytes).unwrap();
        return;
    }
    assert!(
        bytes == fixture(&format!("pages/{folder}/{name}")).as_slice(),
        "{folder}/{name}: run with UPDATE_GOLDEN=1, then tools/fixtures/label_facts.py"
    );
}

/// The labels Lamina PDF reads from `bytes`, `None` for a page without one.
pub fn labels_of(bytes: Vec<u8>) -> Vec<Option<String>> {
    let doc = laminapdf_model::Document::open(bytes, OpenOptions::default()).unwrap();
    let labels = doc.page_labels().unwrap();
    (0..doc.pages().unwrap().len()).map(|i| labels.label(i)).collect()
}

/// What pdf.js and PDFium show on `fixtures/pages/<folder>/<name>`, and whether qpdf accepts
/// it, from the folder's `facts.json`.
pub fn shown_elsewhere(folder: &str, name: &str) -> (serde_json::Value, serde_json::Value, bool) {
    let facts: serde_json::Value = serde_json::from_slice(&fixture(&format!("pages/{folder}/facts.json"))).unwrap();
    let file = facts.get("files").and_then(|f| f.get(name)).unwrap();
    let field = |key: &str| file.get(key).cloned().unwrap();
    (field("pdfjs"), field("pdfium"), field("qpdf_check").as_bool().unwrap())
}
