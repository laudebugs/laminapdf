//! The `document` fuzz target's entry point (ADR 0018 tier 9). `fuzz/fuzz_targets/document.rs`
//! calls it (through `fuzz/src/lib.rs`, which includes this file), and
//! `tests/document_regressions.rs` replays its seeds and any saved crash input through it.

use laminapdf_build::flow::document::{Resources, fill};
use laminapdf_build::{BuildOptions, Builder};
use laminapdf_save::{Save as _, SaveOptions};
use laminapdf_syntax::ErrorCode;

/// A template and its data, as JSON, separated by a zero byte (data `{}` without one). Fills
/// the template, lays the document out in the standard 14 fonts and saves it. Invariants: no
/// panic, no hang; every failure has a code `fixtures/codes.json` lists; a document that builds
/// also saves.
pub fn document(data: &[u8]) {
    let (template, values) = match data.iter().position(|&b| b == 0) {
        Some(at) => data.split_at(at),
        None => (data, &b"\0{}"[..]),
    };
    let values = values.get(1..).unwrap_or_default();
    let (Ok(template), Ok(values)) = (
        serde_json::from_slice::<serde_json::Value>(template),
        serde_json::from_slice::<serde_json::Value>(values),
    ) else {
        return;
    };
    let document = match fill(&template, &values) {
        Ok(document) => document,
        Err(e) => {
            assert!(ErrorCode::from_code(e.code()).is_some(), "{e:?}");
            return;
        }
    };
    let options = document.profiles().into_iter().fold(
        BuildOptions::new().with_id(*b"lamina-fuzz-docu"),
        BuildOptions::with_profile,
    );
    let mut b = Builder::new(options);
    if let Err(e) = b.document(&document, &Resources::new()) {
        assert!(ErrorCode::from_code(e.code()).is_some(), "{e:?}");
        return;
    }
    match b.finish() {
        Ok(mut doc) => {
            let mut out = Vec::new();
            doc.save(&mut out, &SaveOptions::new()).unwrap();
        }
        Err(e) => assert!(ErrorCode::from_code(e.code()).is_some(), "{e:?}"),
    }
}
