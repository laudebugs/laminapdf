//! An encrypted base (ADR 0007 §2, todo B3c): with no encryption on write (E11), every mode is
//! refused unless the caller removes the encryption, which writes a plaintext full rewrite.
//! Over `fixtures/corpus/enc.pdf`, every file of `fixtures/encrypted/` and the signed and
//! encrypted `fixtures/signed/signed-encrypted.pdf`.
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
use lamina_cos::OpenOptions;
use lamina_ops::{Document, SaveState};
use lamina_save::{
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
    fn into_source(self) -> Result<Arc<dyn lamina_cos::ByteSource>, SinkError> {
        Ok(Arc::new(self.0.lock().unwrap().clone()))
    }
}

const MODES: [SaveMode; 3] = [SaveMode::Auto, SaveMode::Incremental, SaveMode::FullRewrite];

#[test]
fn every_mode_refuses_an_encrypted_base_and_writes_nothing() {
    for mode in MODES {
        for readable in [false, true] {
            let mut doc = dirty("corpus/enc.pdf", "");
            let before: Vec<_> = doc.overlay().iter().map(|(n, e)| (n, e.clone())).collect();
            let sink = Shared::default();
            let options = SaveOptions::new().with_mode(mode);
            let result = if readable {
                doc.save_and_rebase(sink.clone(), &options)
            } else {
                doc.save(&mut sink.clone(), &options)
            };
            let after: Vec<_> = doc.overlay().iter().map(|(n, e)| (n, e.clone())).collect();
            assert_eq!(
                (
                    result.err(),
                    sink.0.lock().unwrap().len(),
                    doc.save_state(),
                    after == before
                ),
                (Some(SaveError::EncryptedBase), 0, SaveState::Dirty, true),
                "{mode:?}, readable {readable}"
            );
        }
    }
}

#[test]
fn a_clean_encrypted_document_saves_unchanged_but_refuses_a_rewrite() {
    // Writing the base bytes unchanged keeps the encryption and breaks nothing.
    let bytes = fixture("corpus/enc.pdf");
    let mut doc = open("corpus/enc.pdf", "");
    let mut out = Vec::new();
    let report = doc.save(&mut out, &SaveOptions::new()).unwrap();
    assert_eq!((report.mode(), out == bytes), (SaveMode::Incremental, true));
    assert_eq!(
        doc.save(&mut Vec::new(), &SaveOptions::new().with_mode(SaveMode::FullRewrite))
            .err(),
        Some(SaveError::EncryptedBase)
    );
}

#[test]
fn the_planner_reports_an_encrypted_base_as_a_rewrite_reason() {
    let doc = open("corpus/enc.pdf", "");
    let planner = SavePlanner::new(&doc).unwrap();
    assert_eq!(
        (
            planner.constraint().kind(),
            planner.constraint().rewrite_reasons.clone()
        ),
        (ConstraintKind::RequiresRewrite, vec![RewriteReason::EncryptedBase])
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
        let reopened = lamina_model::Document::open(out.clone(), OpenOptions::default()).unwrap();
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
        let info = |d: &lamina_model::Document, key: &str| d.info().unwrap().and_then(|i| i.text(key).unwrap());
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
fn signed_and_encrypted_is_a_conflict() {
    let doc = open("signed/signed-encrypted.pdf", "");
    let planner = SavePlanner::new(&doc).unwrap();
    assert_eq!(
        (planner.constraint().kind(), planner.constraint().signatures.len()),
        (ConstraintKind::Conflict, 1)
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
    assert_eq!(
        doc.save(&mut Vec::new(), &SaveOptions::new().with_invalidate_signatures(true))
            .err(),
        Some(SaveError::EncryptedBase)
    );
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
            SaveError::from(lamina_cos::Error::EncryptedBase).code()
        ),
        ("encrypted-base", "encrypted-base")
    );
}
