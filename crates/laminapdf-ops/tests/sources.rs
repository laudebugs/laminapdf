//! Source documents and the blob store (todo E2p): `add_source` names a file by the hash of
//! its bytes, opens it once, keeps its bytes for the session, and leaves the session as it
//! was when the file doesn't open.

mod common;

use laminapdf_ops::{BlobId, Error, Origin, SourceId};
use laminapdf_test_fixtures::fixture;

/// `fixtures/corpus/hello.pdf`'s ID, which `laminapdf-wasm`'s `tests/boundary.rs` checks on
/// wasm32 too.
const HELLO_ID: &str = "f534f6f1a920e560dfd6103ae24af60e318d129a177a038366135afc943d058d";

#[test]
fn the_same_bytes_added_twice_are_one_source_named_by_their_hash() {
    let mut doc = common::corpus("outline.pdf");
    let hello = fixture("corpus/hello.pdf");
    let first = doc.add_source(hello.clone(), None).unwrap();
    let second = doc.add_source(hello, None).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.to_string(), HELLO_ID);
    assert_eq!(doc.blobs().len(), 1);
    assert_eq!(doc.source_document(&first).unwrap().page_count().unwrap(), 1);
}

#[test]
fn a_source_the_session_lacks_is_unknown_source() {
    let doc = common::corpus("outline.pdf");
    let id: SourceId = HELLO_ID.parse().unwrap();
    let err = doc.source_document(&id).unwrap_err();
    assert_eq!(err, Error::UnknownSource(id));
    assert_eq!(err.code(), "unknown-source");
}

#[test]
fn an_encrypted_source_opens_with_its_password_and_a_wrong_one_adds_nothing() {
    let mut doc = common::corpus("outline.pdf");
    let bytes = fixture("encrypted/r4-aes-128-user.pdf");
    let err = doc.add_source(bytes.clone(), Some("not it")).unwrap_err();
    assert_eq!(err.code(), "wrong-password");
    assert!(doc.blobs().is_empty());
    let id = doc.add_source(bytes, Some("user-pw")).unwrap();
    let source = doc.source_document(&id).unwrap();
    assert!(source.encryption().is_some());
    assert!(source.page_count().unwrap() > 0);
}

#[test]
fn adding_a_source_changes_nothing_to_save_or_undo() {
    let mut doc = common::corpus("outline.pdf");
    doc.add_source(fixture("corpus/hello.pdf"), None).unwrap();
    assert!(!doc.is_dirty());
    assert!(!doc.has_unsaved_changes());
    assert_eq!(doc.history().undo_len(), 0);
    // Sources stay through a transaction rolled back around them.
    let _ = doc.transact(Origin::Local, |_| -> laminapdf_ops::Result<()> {
        Err(Error::NoTransaction)
    });
    assert!(doc.source_document(&HELLO_ID.parse::<BlobId>().unwrap()).is_ok());
}
