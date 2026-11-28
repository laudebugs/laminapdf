//! An encrypted base (ADR 0007 §2, todos B3c and E11b): every mode keeps its protection, under the
//! key the document was opened with, and writes nothing readable; `remove_encryption` writes a
//! plaintext full rewrite instead. Over `fixtures/corpus/enc.pdf`, every file of
//! `fixtures/encrypted/` and the signed and encrypted `fixtures/signed/signed-encrypted.pdf`.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use std::sync::{Arc, Mutex};

use common::fixture;
use common::save::{facts, ids, set_title};
use laminapdf_cos::OpenOptions;
use laminapdf_ops::{Document, SaveState};
use laminapdf_save::{
    ConstraintKind, ReadableSink, RewriteReason, Save, SaveError, SaveMode, SaveOptions, SavePlanner, SinkError,
    WriteSink,
};

/// Every encrypted fixture that opens, with the password that opens it.
fn encrypted() -> Vec<(String, String)> {
    let passwords: serde_json::Value = serde_json::from_slice(&fixture("encrypted/passwords.json")).unwrap();
    let mut out = vec![("corpus/enc.pdf".to_owned(), String::new())];
    for (name, entry) in passwords["files"].as_object().unwrap() {
        out.push((format!("encrypted/{name}"), entry["user"].as_str().unwrap().to_owned()));
    }
    out
}

fn open(path: &str, password: &str) -> Document {
    Document::open(fixture(path), OpenOptions::default().with_password(password)).unwrap()
}

fn dirty(path: &str, password: &str) -> Document {
    let mut doc = open(path, password);
    assert!(doc.is_encrypted(), "{path}");
    set_title(&mut doc, "Changed");
    doc
}

/// A readable sink whose bytes the test can still see after the save consumed it.
#[derive(Clone, Default)]
struct Shared(Arc<Mutex<Vec<u8>>>);

impl WriteSink for Shared {
    fn write_all(&mut self, bytes: &[u8]) -> Result<(), SinkError> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(())
    }
}

impl ReadableSink for Shared {
    fn into_source(self) -> Result<Arc<dyn laminapdf_cos::ByteSource>, SinkError> {
        Ok(Arc::new(self.0.lock().unwrap().clone()))
    }
}

const MODES: [SaveMode; 3] = [SaveMode::Auto, SaveMode::Incremental, SaveMode::FullRewrite];

const SEED: [u8; 32] = [7; 32];

fn seeded(mode: SaveMode) -> SaveOptions {
    SaveOptions::new().with_mode(mode).with_entropy_seed(SEED)
}

#[test]
fn every_mode_keeps_the_protection_of_an_encrypted_base_and_leaks_nothing() {
    for (path, password) in encrypted() {
        if path.contains("bad-padding") {
            continue; // damaged on purpose: not every object decrypts
        }
        for (mode, want) in [
            (SaveMode::Auto, SaveMode::Incremental),
            (SaveMode::Incremental, SaveMode::Incremental),
            (SaveMode::FullRewrite, SaveMode::FullRewrite),
        ] {
            let mut doc = dirty(&path, &password);
            let mut out = Vec::new();
            let report = doc
                .save(&mut out, &seeded(mode))
                .unwrap_or_else(|e| panic!("{path} {mode:?}: {e}"));
            assert_eq!(
                (report.mode(), report.encrypted(), report.encryption_removed()),
                (want, true, false),
                "{path} {mode:?}"
            );
            // The new title is in the Info dictionary, which the key encrypts: no plaintext copy.
            // (A file whose strings or streams use `/Identity`, or whose metadata is left clear, keeps
            // those plain on purpose.)
            let plain = laminapdf_syntax::crypt::Cipher::Identity;
            let identity = doc
                .encryption()
                .is_some_and(|e| e.string_cipher == plain || e.stream_cipher == plain || !e.encrypt_metadata);
            assert!(
                identity || !out.windows(7).any(|w| w == b"Changed"),
                "{path} {mode:?}: plaintext in the output"
            );
            let reopened =
                laminapdf_model::Document::open(out, OpenOptions::default().with_password(password.as_str())).unwrap();
            assert!(reopened.is_encrypted(), "{path} {mode:?}");
            assert_eq!(facts(&reopened).title.as_deref(), Some("Changed"), "{path} {mode:?}");
        }
    }
}

#[test]
fn a_save_that_encrypts_needs_an_entropy_seed_and_writes_nothing_without_one() {
    for mode in MODES {
        for readable in [false, true] {
            let mut doc = dirty("corpus/enc.pdf", "");
            let sink = Shared::default();
            let options = SaveOptions::new().with_mode(mode);
            let result = if readable {
                doc.save_and_rebase(sink.clone(), &options)
            } else {
                doc.save(&mut sink.clone(), &options)
            };
            assert_eq!(
                (result.err(), sink.0.lock().unwrap().len(), doc.save_state()),
                (Some(SaveError::EncryptionNoEntropy), 0, SaveState::Dirty),
                "{mode:?}, readable {readable}"
            );
        }
    }
}

#[test]
fn a_clean_encrypted_document_saves_unchanged_without_a_seed() {
    // Writing the base bytes unchanged keeps the encryption and needs no randomness.
    let bytes = fixture("corpus/enc.pdf");
    let mut doc = open("corpus/enc.pdf", "");
    let mut out = Vec::new();
    let report = doc.save(&mut out, &SaveOptions::new()).unwrap();
    assert_eq!((report.mode(), out == bytes), (SaveMode::Incremental, true));
}

#[test]
fn the_same_seed_writes_the_same_bytes_and_another_seed_other_ones() {
    let save = |seed: [u8; 32]| {
        let mut doc = dirty("corpus/enc.pdf", "");
        let mut out = Vec::new();
        doc.save(
            &mut out,
            &SaveOptions::new()
                .with_mode(SaveMode::FullRewrite)
                .with_entropy_seed(seed),
        )
        .unwrap();
        out
    };
    assert_eq!(save([1; 32]), save([1; 32]));
    assert_ne!(save([1; 32]), save([2; 32]));
}

#[test]
fn a_rewrite_keeps_the_protection_so_the_next_save_needs_no_password() {
    let mut doc = dirty("encrypted/r6-aes-256-user.pdf", "Pässwört-6");
    doc.save_and_rebase(Vec::new(), &seeded(SaveMode::FullRewrite)).unwrap();
    assert_eq!((doc.save_state(), doc.is_encrypted()), (SaveState::Clean, true));
    set_title(&mut doc, "Again");
    let mut out = Vec::new();
    let report = doc.save(&mut out, &seeded(SaveMode::Auto)).unwrap();
    assert_eq!((report.mode(), report.encrypted()), (SaveMode::Incremental, true));
    let reopened = laminapdf_model::Document::open(out, OpenOptions::default().with_password("Pässwört-6")).unwrap();
    assert_eq!(facts(&reopened).title.as_deref(), Some("Again"));
}

#[test]
fn the_planner_calls_an_encrypted_base_free_and_removing_it_a_rewrite() {
    let doc = open("corpus/enc.pdf", "");
    let planner = SavePlanner::new(&doc).unwrap();
    assert_eq!(
        (
            planner.constraint().kind(),
            planner.constraint().rewrite_reasons.clone()
        ),
        (ConstraintKind::Free, vec![])
    );
    let plan = planner.plan(&SaveOptions::new().with_remove_encryption(true)).unwrap();
    assert_eq!(
        (plan.removes_encryption(), plan.constraint().rewrite_reasons.clone()),
        (true, vec![RewriteReason::EncryptedBase])
    );
}

#[test]
fn an_incremental_save_with_remove_encryption_requires_a_rewrite() {
    let mut doc = dirty("corpus/enc.pdf", "");
    let options = SaveOptions::new()
        .with_mode(SaveMode::Incremental)
        .with_remove_encryption(true);
    assert_eq!(
        doc.save(&mut Vec::new(), &options).err(),
        Some(SaveError::RequiresRewrite {
            reasons: vec![RewriteReason::EncryptedBase]
        })
    );
}

#[test]
fn auto_with_remove_encryption_writes_a_plaintext_full_rewrite_of_every_fixture() {
    for (path, password) in encrypted() {
        if path.contains("bad-padding") {
            continue; // damaged on purpose: not every object decrypts
        }
        let original = open(&path, &password);
        let mut doc = dirty(&path, &password);
        let mut out = Vec::new();
        let report = doc
            .save(&mut out, &SaveOptions::new().with_remove_encryption(true))
            .unwrap_or_else(|e| panic!("{path}: {e}"));
        let reopened = laminapdf_model::Document::open(out.clone(), OpenOptions::default()).unwrap();
        assert_eq!(
            (report.mode(), report.encryption_removed(), reopened.is_encrypted()),
            (SaveMode::FullRewrite, true, false),
            "{path}"
        );
        assert!(!reopened.trailer().contains_key("Encrypt"), "{path}");
        assert_eq!(facts(&reopened).title.as_deref(), Some("Changed"), "{path}");
        assert_eq!(ids(&reopened).0, ids(&original).0, "{path}: /ID[0] kept");
        let pages = original.page_count().unwrap();
        assert_eq!(reopened.page_count().unwrap(), pages, "{path}");
        let (a, b) = (original.pages().unwrap(), reopened.pages().unwrap());
        for (before, after) in a.iter().zip(b.iter()) {
            assert_eq!(
                reopened.page_content(after.cos()).unwrap(),
                original.page_content(before.cos()).unwrap(),
                "{path}: content"
            );
        }
        let info = |d: &laminapdf_model::Document, key: &str| d.info().unwrap().and_then(|i| i.text(key).unwrap());
        for key in ["Author", "Subject", "Keywords", "Creator", "Producer"] {
            assert_eq!(info(&reopened, key), info(&original, key), "{path}: /{key}");
        }
    }
}

#[test]
fn removing_encryption_rebases_onto_the_plaintext_file() {
    let mut doc = dirty("corpus/enc.pdf", "");
    doc.save_and_rebase(Vec::new(), &SaveOptions::new().with_remove_encryption(true))
        .unwrap();
    assert_eq!((doc.save_state(), doc.is_encrypted()), (SaveState::Clean, false));
    // Now a plain document: Auto appends again.
    set_title(&mut doc, "Again");
    let report = doc.save(&mut Vec::new(), &SaveOptions::new()).unwrap();
    assert_eq!(report.mode(), SaveMode::Incremental);
}

#[test]
fn signed_and_encrypted_is_append_only_until_the_encryption_is_removed() {
    let doc = open("signed/signed-encrypted.pdf", "");
    let planner = SavePlanner::new(&doc).unwrap();
    assert_eq!(
        (planner.constraint().kind(), planner.constraint().signatures.len()),
        (ConstraintKind::AppendOnly, 1)
    );
}

#[test]
fn signed_and_encrypted_needs_both_overrides() {
    let only_remove = SaveOptions::new().with_remove_encryption(true);
    let mut doc = dirty("signed/signed-encrypted.pdf", "");
    assert_eq!(
        doc.save(&mut Vec::new(), &only_remove).map_err(|e| e.code()),
        Err("would-invalidate-signatures")
    );
    let mut doc = dirty("signed/signed-encrypted.pdf", "");
    let mut kept = Vec::new();
    let report = doc
        .save(&mut kept, &SaveOptions::new().with_entropy_seed(SEED))
        .unwrap();
    // Kept under its key, a change appends and the signature's bytes stay as they were.
    assert_eq!(
        (report.mode(), report.encrypted(), report.invalidated().len()),
        (SaveMode::Incremental, true, 0)
    );
    let mut doc = dirty("signed/signed-encrypted.pdf", "");
    let rewrite = seeded(SaveMode::FullRewrite);
    assert_eq!(
        doc.save(&mut Vec::new(), &rewrite).map_err(|e| e.code()),
        Err("would-invalidate-signatures")
    );
    let report = doc
        .save(&mut Vec::new(), &rewrite.with_invalidate_signatures(true))
        .unwrap();
    assert_eq!(
        (report.mode(), report.encrypted(), report.invalidated().len()),
        (SaveMode::FullRewrite, true, 1)
    );
    let mut doc = dirty("signed/signed-encrypted.pdf", "");
    let mut out = Vec::new();
    let report = doc
        .save(&mut out, &only_remove.with_invalidate_signatures(true))
        .unwrap();
    let reopened = Document::open(out, OpenOptions::default()).unwrap();
    assert_eq!(
        (
            report.mode(),
            report.encryption_removed(),
            report.invalidated().len(),
            reopened.is_encrypted(),
            SavePlanner::new(&reopened).unwrap().constraint().signatures.len()
        ),
        (SaveMode::FullRewrite, true, 1, false, 0)
    );
}

#[test]
fn the_refusal_has_the_cos_code() {
    assert_eq!(
        (
            SaveError::EncryptedBase.code(),
            SaveError::from(laminapdf_cos::Error::EncryptedBase).code()
        ),
        ("encrypted-base", "encrypted-base")
    );
}
