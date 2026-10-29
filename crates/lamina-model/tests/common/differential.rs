//! lamina's side of the differential suite (ADR 0018 tier 6): the facts that
//! `tools/corpus-facts/facts.py` (pypdf) and `tools/differential/` (pdf.js, PDFium, qpdf) also
//! report. Shared by `tests/differential.rs` (the PR lane, against precomputed pypdf facts) and
//! `examples/differential-facts.rs` (the nightly lane's live run over the external corpus).

use serde_json::{Value, json};

use super::{open, outline_titles};

/// The facts every differential tool reports about one file.
pub const FACTS: [&str; 3] = ["outcome", "page_count", "outline_titles"];

/// lamina's facts about `bytes`, extracted the same way as `facts.py` does: the outcome is
/// about opening the file and walking its pages; an unreadable outline is its own fact
/// (`null`) and leaves the outcome. A failure also carries lamina's error code (`error`), which
/// only the nightly report shows.
pub fn facts(bytes: &[u8]) -> Value {
    let failed = |code: &str| json!({"outcome": "failed", "page_count": null, "outline_titles": null, "error": code});
    let doc = match open(bytes) {
        Ok(doc) => doc,
        Err(e) => return failed(e.code()),
    };
    let pages = match doc.pages() {
        Ok(pages) => pages,
        Err(e) => return failed(e.code()),
    };
    let outcome = if doc.was_repaired() { "repaired" } else { "opened" };
    json!({
        "outcome": outcome,
        "page_count": pages.len(),
        "outline_titles": outline_titles(&doc).ok(),
    })
}
