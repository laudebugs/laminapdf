//! `SetEncryption` on signed files (E11b, ADR 0007 §2, E11 plan §5): the op needs a full rewrite,
//! which moves every byte a signature covers, so `would_violate` names every signature whatever
//! its certification level, a save refuses, and the override removes the signatures and encrypts.
//! A misclassified op is a security bug, so each fixture is checked.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::fixture;
use laminapdf_cos::OpenOptions;
use laminapdf_ops::{Document, Op, Origin, SetEncryption};
use laminapdf_save::{Restriction, Save, SaveMode, SaveOptions, SavePlanner};

const OP: fn() -> Op = || SetEncryption::new("open-pw", "owner-pw").into();

/// Every signed fixture: no certification (an approval), DocMDP P1, P2 and P3, a field lock, a
/// usage-rights signature and two signatures.
const SIGNED: [&str; 9] = [
    "signed/approval.pdf",
    "signed/certified-p1.pdf",
    "signed/certified-p2.pdf",
    "signed/certified-p3.pdf",
    "signed/form-locked-exclude.pdf",
    "signed/locked.pdf",
    "signed/usage-rights.pdf",
    "signed/two-signatures.pdf",
    "signed/timestamp.pdf",
];

fn open(path: &str) -> Document {
    Document::open(fixture(path), OpenOptions::default()).unwrap()
}

#[test]
fn would_violate_names_every_signature_of_every_signed_fixture() {
    for path in SIGNED {
        let doc = open(path);
        let planner = SavePlanner::new(&doc).unwrap();
        let signatures = planner.constraint().signatures.len();
        assert!(signatures > 0, "{path} has signatures");
        let violations = planner.would_violate(&OP());
        let signed_bytes = violations.iter().filter(|v| v.restriction == Restriction::SignedBytes).count();
        assert_eq!(signed_bytes, signatures, "{path}: one SignedBytes violation per signature");
    }
}

#[test]
fn an_unsigned_file_has_nothing_to_violate() {
    let doc = open("corpus/hello.pdf");
    assert!(SavePlanner::new(&doc).unwrap().would_violate(&OP()).is_empty());
}

#[test]
fn a_save_refuses_until_the_signatures_may_go_and_then_removes_them() {
    for path in SIGNED {
        let mut doc = open(path);
        let signatures = SavePlanner::new(&doc).unwrap().constraint().signatures.len();
        doc.apply_all(Origin::Local, [OP()]).unwrap();
        let options = SaveOptions::new().with_entropy_seed([1; 32]);
        for mode in [SaveMode::Auto, SaveMode::FullRewrite] {
            let err = doc.save(&mut Vec::new(), &options.with_mode(mode)).unwrap_err();
            assert_eq!(err.code(), "would-invalidate-signatures", "{path} {mode:?}");
        }
        let mut out = Vec::new();
        let report = doc.save(&mut out, &options.with_invalidate_signatures(true)).unwrap();
        assert_eq!(
            (report.mode(), report.encrypted(), report.invalidated().len()),
            (SaveMode::FullRewrite, true, signatures),
            "{path}"
        );
        let reopened =
            laminapdf_model::Document::open(out, OpenOptions::default().with_password("open-pw")).unwrap();
        assert!(reopened.is_encrypted(), "{path}");
    }
}

#[test]
fn save_a_copy_writes_the_protected_file_and_leaves_the_signed_original_pending() {
    let original = fixture("signed/certified-p2.pdf");
    let mut doc = open("signed/certified-p2.pdf");
    doc.apply_all(Origin::Local, [OP()]).unwrap();
    let mut copy = Vec::new();
    let options = SaveOptions::new().with_entropy_seed([1; 32]).with_invalidate_signatures(true);
    doc.save_copy(&mut copy, &options).unwrap();
    assert!(doc.pending_encryption().is_some() && doc.has_unsaved_changes());
    assert_eq!(doc.cos().source().as_slice().unwrap(), &original[..], "the original's bytes are as they were");
    assert_ne!(copy, original);
}
