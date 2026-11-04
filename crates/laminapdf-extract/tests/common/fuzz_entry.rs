//! The `extract` fuzz target's entry point (ADR 0018 tier 9). `fuzz/fuzz_targets/extract.rs`
//! calls it (through `fuzz/src/lib.rs`, which includes this file), and `tests/regressions.rs`
//! replays every saved crash input (`fixtures/regressions/extract/`) through it.

use laminapdf_extract::{ExtractOptions, ReadingOrder, extract as run};
use laminapdf_model::{Document, OpenOptions};
use laminapdf_syntax::CancelToken;

/// Extracts every page of `data` (a whole file; anything else is ignored) by layout and by
/// content order. Invariants: no panic, no hang, every block's box finite with its normalised
/// box in [0, 1], block IDs unique, and no block text holding a dropped run's text.
pub fn extract(data: &[u8]) {
    if !data.starts_with(b"%PDF") {
        return;
    }
    let Ok(doc) = Document::open(data.to_vec(), OpenOptions::default()) else {
        return;
    };
    for order in [ReadingOrder::Layout, ReadingOrder::ContentStream] {
        let mut options = ExtractOptions::new().with_reading_order(order);
        options.drop.keep_spans = true;
        let Ok(out) = run(&doc, &options, &CancelToken::new()) else {
            continue;
        };
        let mut ids = std::collections::BTreeSet::new();
        for block in &out.blocks {
            assert!(block.bbox.is_finite(), "{}: box not finite", block.id);
            if let Some(norm) = block.bbox_norm {
                assert!(norm.iter().all(|v| (0.0..=1.0).contains(v)), "{}: {norm:?}", block.id);
            }
            assert!(ids.insert(block.id.clone()), "{}: id used twice", block.id);
            assert!(block.spans.iter().all(|s| !s.flags.any()), "{}: a flagged span", block.id);
        }
    }
}
