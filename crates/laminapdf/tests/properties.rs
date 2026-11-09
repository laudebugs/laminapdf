//! `Document::properties` on every corpus file against pypdf's facts (`fixtures/corpus/expected.json`,
//! `properties`): title, producer, the encryption revision, the fonts by name and the signatures; and
//! the version against the header, for files whose catalog sets none.

use laminapdf::{Document, OpenOptions};
use laminapdf_test_fixtures::{corpus, json};
use serde_json::{Value, json};

#[test]
fn every_corpus_files_properties_match_pypdf() {
    let facts = json("corpus/expected.json");
    for (name, bytes) in corpus() {
        let fact = &facts[&name];
        let doc = Document::open_bytes(bytes, OpenOptions::default()).unwrap();
        let p = doc.properties().unwrap();
        let mut fonts: Vec<String> = p.fonts.iter().filter_map(|f| f.name.clone()).collect();
        fonts.sort();
        fonts.dedup();
        let actual = json!({
            "title": p.info.title,
            "producer": p.info.producer,
            "encryption_revision": p.encryption.as_ref().map(|e| e.revision),
            "fonts": fonts,
            "signatures": p.signatures,
        });
        assert_eq!(actual, fact["properties"], "{name}");
        assert_eq!(u64::from(p.page_count), fact["page_count"].as_u64().unwrap(), "{name}");
        assert_eq!(p.file_size, fact["size"].as_u64().unwrap(), "{name}");
        let catalog_version = fact["root_keys"].as_array().unwrap().contains(&Value::from("/Version"));
        if !catalog_version {
            assert_eq!(p.version.as_deref(), fact["header_version"].as_str(), "{name}");
        }
    }
}

#[test]
fn an_encrypted_files_permissions_and_cipher_are_read() {
    let doc = Document::open_bytes(
        laminapdf_test_fixtures::fixture("corpus/enc.pdf"),
        OpenOptions::default(),
    )
    .unwrap();
    let encryption = doc.properties().unwrap().encryption.unwrap();
    assert_eq!(
        (encryption.revision, encryption.key_bits, encryption.cipher),
        (6, 256, laminapdf::Cipher::Aes256)
    );
    assert!(encryption.permissions.is_some());
}

#[test]
fn dates_come_out_as_iso_8601() {
    let doc = Document::open_bytes(
        laminapdf_test_fixtures::fixture("corpus/outline.pdf"),
        OpenOptions::default(),
    )
    .unwrap();
    let p = doc.properties().unwrap();
    // outline.pdf's /CreationDate is D:20261028120000Z; its /ModDate isn't a valid date.
    assert_eq!(
        (p.created.as_deref(), p.modified.as_deref()),
        (Some("2026-10-28T12:00:00Z"), None)
    );
}
