//! Both writers refuse an encrypted base before writing anything (ADR 0007 §2): there is no
//! encryption on write yet (E11) and removing it is B3c's explicit option.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::save::{corpus_doc, set_title};
use lamina_ops::SaveState;
use lamina_save::{Renumber, Save, SaveError, SaveOptions, XrefKind};

fn dirty_encrypted() -> lamina_ops::Document {
    let mut doc = corpus_doc("enc.pdf");
    assert!(doc.is_encrypted());
    set_title(&mut doc, "Changed");
    doc
}

#[test]
fn every_save_of_an_encrypted_base_fails_and_writes_nothing() {
    let options = [
        SaveOptions::new(),
        SaveOptions::new().with_xref(XrefKind::Classic),
        SaveOptions::new().with_renumber(Renumber::Compact),
    ];
    for (i, options) in options.iter().enumerate() {
        for which in 0..3 {
            let mut doc = dirty_encrypted();
            let before: Vec<_> = doc.overlay().iter().map(|(n, e)| (n, e.clone())).collect();
            let mut out = Vec::new();
            let result = match which {
                0 => doc.save_incremental(&mut out, options),
                1 => doc.save_full(&mut out, options),
                _ => doc.save_full_and_rebase(Vec::new(), options),
            };
            let after: Vec<_> = doc.overlay().iter().map(|(n, e)| (n, e.clone())).collect();
            assert_eq!(
                (result.err(), out.is_empty(), doc.save_state(), after == before),
                (Some(SaveError::EncryptedBase), true, SaveState::Dirty, true),
                "options {i}, writer {which}"
            );
        }
    }
}

#[test]
fn the_refusal_has_the_cos_code() {
    assert_eq!(SaveError::EncryptedBase.code(), "encrypted-base");
}
