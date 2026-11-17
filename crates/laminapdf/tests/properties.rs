//! `Document::properties` on every corpus file against pypdf's facts (`fixtures/corpus/expected.json`,
//! `properties`): title, producer, the encryption revision, the fonts by name and the signatures; and
//! the version against the header, for files whose catalog sets none. The pypdf metadata
//! differential compares `Document::info` and the properties' dates with the same facts.
#![expect(clippy::unwrap_used, reason = "test code: a panic is a test failure")]

use laminapdf::{Document, OpenOptions, PdfDate};
use laminapdf_test_fixtures::{corpus, fixture, json};
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
        let expected: serde_json::Map<String, Value> =
            ["title", "producer", "encryption_revision", "fonts", "signatures"]
                .into_iter()
                .map(|key| (key.to_owned(), fact["properties"][key].clone()))
                .collect();
        assert_eq!(actual, Value::Object(expected), "{name}");
        assert_eq!(u64::from(p.page_count), fact["page_count"].as_u64().unwrap(), "{name}");
        assert_eq!(p.file_size, fact["size"].as_u64().unwrap(), "{name}");
        let catalog_version = fact["root_keys"].as_array().unwrap().contains(&Value::from("/Version"));
        if !catalog_version {
            assert_eq!(p.version.as_deref(), fact["header_version"].as_str(), "{name}");
        }
    }
}

/// The `properties` facts that `fixtures/corpus/exceptions.toml` excuses, as
/// `(file, "properties.<key>")` pairs.
fn excused_properties() -> Vec<(String, String)> {
    let table: toml::Table = String::from_utf8(fixture("corpus/exceptions.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let entries = table
        .get("exception")
        .and_then(toml::Value::as_array)
        .cloned()
        .unwrap_or_default();
    entries
        .iter()
        .flat_map(|e| {
            let file = e["file"].as_str().unwrap().to_owned();
            e["facts"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|f| f.as_str().filter(|f| f.starts_with("properties.")))
                .map(move |f| (file.clone(), f.to_owned()))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// pypdf's raw Info date as ISO 8601, read with Lamina PDF's date grammar; `None` when absent
/// or not a date, as `DocumentProperties` reports it.
fn iso_date(raw: &Value) -> Value {
    raw.as_str()
        .and_then(|s| s.parse::<PdfDate>().ok())
        .map_or(Value::Null, |d| Value::from(d.to_iso8601()))
}

#[test]
fn the_pypdf_metadata_differential_agrees_on_every_corpus_file() {
    let facts = json("corpus/expected.json");
    let excused = excused_properties();
    for (name, bytes) in corpus() {
        let pypdf = &facts[&name]["properties"];
        let doc = Document::open_bytes(bytes, OpenOptions::default()).unwrap();
        let info = doc.info().unwrap();
        let p = doc.properties().unwrap();
        let lamina = [
            ("title", json!(info.title)),
            ("author", json!(info.author)),
            ("subject", json!(info.subject)),
            ("keywords", json!(info.keywords)),
            ("creator", json!(info.creator)),
            ("producer", json!(info.producer)),
            ("creation_date", json!(p.created)),
            ("mod_date", json!(p.modified)),
        ];
        for (key, actual) in lamina {
            if excused.contains(&(name.clone(), format!("properties.{key}"))) {
                continue;
            }
            let expected = match key {
                "creation_date" | "mod_date" => iso_date(&pypdf[key]),
                _ => pypdf[key].clone(),
            };
            assert_eq!(actual, expected, "{name}: {key}");
        }
    }
}

#[test]
fn an_encrypted_files_permissions_and_cipher_are_read() {
    let doc = Document::open_bytes(fixture("corpus/enc.pdf"), OpenOptions::default()).unwrap();
    let encryption = doc.properties().unwrap().encryption.unwrap();
    assert_eq!(
        (encryption.revision, encryption.key_bits, encryption.cipher),
        (6, 256, laminapdf::Cipher::Aes256)
    );
    assert!(encryption.permissions.is_some());
}

#[test]
fn dates_come_out_as_iso_8601() {
    let doc = Document::open_bytes(fixture("corpus/outline.pdf"), OpenOptions::default()).unwrap();
    let p = doc.properties().unwrap();
    // outline.pdf's /CreationDate is D:20261028120000Z; its /ModDate isn't a valid date.
    assert_eq!(
        (p.created.as_deref(), p.modified.as_deref()),
        (Some("2026-10-28T12:00:00Z"), None)
    );
}
