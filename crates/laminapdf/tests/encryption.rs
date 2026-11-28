//! Protecting a file and reading its protection through the façade (E11b, G.01 to G.04, G.11):
//! `SetEncryption` through `apply` and `save`, `encryption_info` before and after, and the
//! passwords and limits that come out.
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing, reason = "test code")]

use laminapdf::{
    Cipher, Document, EncryptionPermissions, MetadataPatch, Op, OpenOptions, OpenedWith, PrintPermission, SaveMode,
    SaveOptions, SetEncryption, SetMetadata,
};
use laminapdf_test_fixtures::classic_with_root;

const SEED: [u8; 32] = [42; 32];

fn plain() -> Document {
    let bytes = classic_with_root(
        &[
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 200 200] >>",
            "<< /Type /Page /Parent 2 0 R >>",
        ],
        "",
    );
    Document::open_bytes(bytes, OpenOptions::new()).unwrap()
}

/// `op` applied to a plain file and saved with a fixed seed.
fn protected(op: SetEncryption) -> Vec<u8> {
    let mut doc = plain();
    doc.apply(vec![Op::from(op)]).unwrap();
    assert!(doc.has_pending_encryption());
    let saved = doc.save(&SaveOptions::new().with_entropy_seed(SEED)).unwrap();
    assert!(saved.report.encrypted() && !doc.has_pending_encryption());
    saved.bytes
}

/// A plain file has no encryption to inspect.
/// capability: G.11 pdf read
#[test]
fn a_plain_file_has_no_encryption_info() {
    assert_eq!(plain().encryption_info(), None);
}

/// capability: G.01 pdf round-trip
/// capability: G.02 pdf round-trip
/// capability: G.03 pdf round-trip
#[test]
fn a_protected_file_opens_with_either_password_and_says_which() {
    let bytes = protected(SetEncryption::new("open-pw", "owner-pw"));
    let user = Document::open_bytes(bytes.clone(), OpenOptions::new().with_password("open-pw")).unwrap();
    let owner = Document::open_bytes(bytes.clone(), OpenOptions::new().with_password("owner-pw")).unwrap();
    assert_eq!(
        (user.encryption_info().unwrap().opened_with, owner.encryption_info().unwrap().opened_with),
        (OpenedWith::User, OpenedWith::Owner)
    );
    let err = Document::open_bytes(bytes, OpenOptions::new()).err().unwrap();
    assert_eq!(err.code(), "password-required");
}

/// capability: G.11 pdf read
#[test]
fn the_info_names_the_handler_and_no_key() {
    let bytes = protected(SetEncryption::new("open-pw", "owner-pw"));
    let doc = Document::open_bytes(bytes, OpenOptions::new().with_password("open-pw")).unwrap();
    let info = doc.encryption_info().unwrap();
    assert_eq!(
        (info.handler, info.version, info.revision, info.key_bits, info.encrypt_metadata),
        ("Standard", 5, 6, 256, true)
    );
    assert_eq!((info.string_cipher, info.stream_cipher), (Cipher::Aes256, Cipher::Aes256));
    assert!(!format!("{info:?}").contains("open-pw"));
}

/// capability: G.04 pdf round-trip
#[test]
fn the_permissions_the_op_sets_are_the_ones_the_file_reports() {
    let limits = EncryptionPermissions::default()
        .with_print(PrintPermission::LowResolution)
        .with_copy(false)
        .with_comment(false);
    let bytes = protected(SetEncryption::new("", "owner-pw").with_permissions(limits));
    let info = Document::open_bytes(bytes, OpenOptions::new())
        .unwrap()
        .encryption_info()
        .unwrap();
    let p = info.permissions;
    assert_eq!(
        (p.print, p.print_high_quality, p.copy, p.annotate, p.assemble, p.accessibility),
        (true, false, false, false, true, true)
    );
    assert_eq!(info.opened_with, OpenedWith::User);
}

#[test]
fn a_protected_file_edits_and_saves_incrementally_without_the_password_again() {
    let bytes = protected(SetEncryption::new("open-pw", "owner-pw"));
    let mut doc = Document::open_bytes(bytes, OpenOptions::new().with_password("open-pw")).unwrap();
    doc.apply(vec![Op::from(SetMetadata::new(MetadataPatch::new().title(Some("Quarterly")), None))])
        .unwrap();
    let saved = doc
        .save(&SaveOptions::new().with_entropy_seed(SEED).with_mode(SaveMode::Auto))
        .unwrap();
    assert_eq!((saved.report.mode(), saved.report.encrypted()), (SaveMode::Incremental, true));
    let again = Document::open_bytes(saved.bytes, OpenOptions::new().with_password("open-pw")).unwrap();
    assert_eq!(again.metadata().unwrap().title.as_deref(), Some("Quarterly"));
}
