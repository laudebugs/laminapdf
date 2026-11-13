//! The `annots` fuzz target's entry point (the annotation views and ops). `fuzz/fuzz_targets/
//! annots.rs` calls it through `fuzz/src/lib.rs`, which includes this file, and
//! `tests/annotations.rs` runs it over `fixtures/annotations/`.

use laminapdf_cos::OpenOptions;
use laminapdf_ops::{DeleteAnnot, Document, Offset, Origin, UpdateAnnot};

/// The most annotations the entry edits per input, so a file with thousands stays fast.
const MAX_EDITS: usize = 16;

/// Opens `data` as a PDF and reads every page's annotations; then, for the first annotations,
/// moves and deletes each in a transaction of its own and undoes it. Invariants: no panic, an
/// op either applies or fails with an error, and undo leaves the overlay empty again.
pub fn annots(data: &[u8]) {
    let Ok(mut doc) = Document::open(data.to_vec(), OpenOptions::default()) else {
        return;
    };
    let Ok(pages) = doc.pages() else { return };
    let mut ids = Vec::new();
    for page in pages.iter() {
        let Ok(annots) = doc.annotations(&page) else { return };
        ids.extend(annots.into_iter().map(|a| a.id));
    }
    drop(pages);
    for id in ids.into_iter().take(MAX_EDITS) {
        let mut moved = UpdateAnnot::new(id.clone());
        moved.translate = Some(Offset::new(1.0, -1.0));
        moved.contents = Some(Some("fuzz".to_owned()));
        for op in [moved.into(), DeleteAnnot::new(id).into()] {
            if doc.apply_all(Origin::Local, [op]).is_ok() {
                assert!(doc.undo().is_ok(), "undo fails after an op applied");
                assert!(doc.overlay().is_empty(), "undo leaves changes in the overlay");
            }
        }
    }
}
