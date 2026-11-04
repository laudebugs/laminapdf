//! Standard security handler decryption (B2b) over `fixtures/encrypted/`: every revision and
//! cipher, opened with the user, owner and empty passwords, agrees with the facts pypdf and qpdf
//! recorded in `fixtures/encrypted/expected.json` and with the plaintext `source.pdf`.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure; `expect` would be brittle as each test crate uses a different subset"
)]

mod common;

use common::{fixture, open, open_with};
use laminapdf_cos::{Cipher, Document, Error, ObjRef, Object, OpenOptions, PasswordKind, Permissions, Warning};
use laminapdf_save::XrefEol;
use laminapdf_save::test_support::incremental_update;
use serde_json::Value;
use sha2::{Digest, Sha256};

fn expected() -> Value {
    serde_json::from_slice(&fixture("encrypted/expected.json")).unwrap()
}

/// `(file name, facts)` for every encrypted fixture.
fn files() -> Vec<(String, Value)> {
    let expected = expected();
    let files = expected["files"].as_object().unwrap();
    assert!(files.len() >= 16, "expected.json lists {} files", files.len());
    files.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
}

fn encrypted(name: &str) -> Vec<u8> {
    fixture(&format!("encrypted/{name}"))
}

fn with_password(password: &str) -> OpenOptions {
    OpenOptions::default().with_password(password)
}

/// A text string (§7.9.2.2): UTF-16BE with a BOM, else PDFDocEncoding (ASCII in these fixtures).
fn text(bytes: &[u8]) -> String {
    match bytes.strip_prefix(&[0xFE, 0xFF]) {
        Some(utf16) => {
            let units: Vec<u16> = utf16.chunks(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
            String::from_utf16(&units).unwrap()
        }
        None => bytes.iter().map(|b| char::from(*b)).collect(),
    }
}

fn string_of(doc: &Document, obj: &Object) -> String {
    match &*doc.deref(obj).unwrap() {
        Object::String(s) => text(&s.bytes),
        other => panic!("not a string: {other:?}"),
    }
}

fn dict_of(doc: &Document, obj: &Object) -> laminapdf_cos::Dict {
    doc.deref(obj).unwrap().as_dict().unwrap().clone()
}

/// Outline titles, flattened depth first (the order facts.py records).
fn outline_titles(doc: &Document) -> Vec<String> {
    fn walk(doc: &Document, first: Option<&Object>, out: &mut Vec<String>) {
        let mut next = first.cloned();
        while let Some(item) = next {
            let dict = dict_of(doc, &item);
            out.push(string_of(doc, dict.get("Title").unwrap()));
            walk(doc, dict.get("First"), out);
            next = dict.get("Next").cloned();
        }
    }
    let catalog = doc.catalog().unwrap();
    let outlines = dict_of(doc, catalog.get("Outlines").unwrap());
    let mut out = Vec::new();
    walk(doc, outlines.get("First"), &mut out);
    out
}

fn info(doc: &Document) -> serde_json::Map<String, Value> {
    let info = dict_of(doc, doc.trailer().get("Info").unwrap());
    info.iter()
        .map(|(k, v)| {
            (
                String::from_utf8_lossy(k.as_bytes()).into_owned(),
                Value::String(string_of(doc, v)),
            )
        })
        .collect()
}

fn contents(doc: &Document) -> Vec<Vec<u8>> {
    doc.pages()
        .unwrap()
        .iter()
        .map(|p| doc.page_content(p).unwrap())
        .collect()
}

fn sha256(data: &[u8]) -> String {
    use std::fmt::Write;
    Sha256::digest(data).iter().fold(String::new(), |mut out, b| {
        write!(out, "{b:02x}").unwrap();
        out
    })
}

fn xmp(doc: &Document) -> Vec<u8> {
    let catalog = doc.catalog().unwrap();
    match &*doc.deref(catalog.get("Metadata").unwrap()).unwrap() {
        Object::Stream(s) => doc.decode_stream(s).unwrap(),
        other => panic!("not a stream: {other:?}"),
    }
}

/// Every mismatch between `doc` and `facts`, one line each.
fn mismatches(doc: &Document, facts: &Value, source: &Document) -> Vec<String> {
    let mut out = Vec::new();
    let mut check = |what: &str, ours: Value, theirs: &Value| {
        if &ours != theirs {
            out.push(format!("{what}: ours {ours} vs expected {theirs}"));
        }
    };
    check("page_count", doc.page_count().unwrap().into(), &facts["page_count"]);
    check("outline_titles", outline_titles(doc).into(), &facts["outline_titles"]);
    check("info", Value::Object(info(doc)), &facts["info"]);
    let contents = contents(doc);
    let hashes: Vec<Value> = contents.iter().map(|c| sha256(c).into()).collect();
    check("content_sha256", hashes.into(), &facts["content_sha256"]);
    check("xmp_sha256", sha256(&xmp(doc)).into(), &facts["xmp_sha256"]);
    if contents != self::contents(source) {
        out.push("decoded content differs from source.pdf".to_owned());
    }
    out
}

/// Opens every fixture with `password(facts)` and collects all mismatches into one failure.
fn check_all(password: impl Fn(&Value) -> Option<String>, matched: PasswordKind) {
    let source = open(&fixture("encrypted/source.pdf")).unwrap();
    let mut failures = Vec::new();
    for (name, facts) in files() {
        let Some(password) = password(&facts) else { continue };
        match open_with(&encrypted(&name), with_password(&password)) {
            Ok(doc) => {
                failures.extend(
                    mismatches(&doc, &facts, &source)
                        .into_iter()
                        .map(|m| format!("{name}: {m}")),
                );
                // A password that is both is the owner's (ISO 32000-2 Algorithm 2.A).
                let matched = if facts["user_password"] == facts["owner_password"] {
                    PasswordKind::Owner
                } else {
                    matched
                };
                if doc.matched_password() != Some(matched) {
                    failures.push(format!("{name}: matched {:?}", doc.matched_password()));
                }
                if !doc.warnings().is_empty() {
                    failures.push(format!("{name}: warnings {:?}", doc.warnings()));
                }
            }
            Err(e) => failures.push(format!("{name}: open failed: {e}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn is_empty_user(facts: &Value) -> bool {
    facts["user_password"] == ""
}

mod opening {
    use super::*;

    #[test]
    fn source_facts_match_the_plaintext_original() {
        let expected = expected();
        let source = open(&fixture("encrypted/source.pdf")).unwrap();
        let facts = &expected["source"];
        assert_eq!(mismatches(&source, facts, &source), Vec::<String>::new());
    }

    /// capability: G.01 core read
    #[test]
    fn every_fixture_opens_with_its_user_password() {
        check_all(
            |f| (!is_empty_user(f)).then(|| f["user_password"].as_str().unwrap().to_owned()),
            PasswordKind::User,
        );
    }

    /// capability: G.03 core read
    #[test]
    fn every_fixture_opens_with_its_owner_password() {
        check_all(
            |f| Some(f["owner_password"].as_str().unwrap().to_owned()),
            PasswordKind::Owner,
        );
    }

    #[test]
    fn empty_user_password_fixtures_open_without_a_password() {
        let source = open(&fixture("encrypted/source.pdf")).unwrap();
        let mut failures = Vec::new();
        for (name, facts) in files().into_iter().filter(|(_, f)| is_empty_user(f)) {
            let doc = open(&encrypted(&name)).unwrap();
            failures.extend(
                mismatches(&doc, &facts, &source)
                    .into_iter()
                    .map(|m| format!("{name}: {m}")),
            );
            if doc.matched_password() != Some(PasswordKind::User) {
                failures.push(format!("{name}: matched {:?}", doc.matched_password()));
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn empty_user_password_fixtures_cover_every_revision() {
        let mut revisions: Vec<u64> = files()
            .iter()
            .filter(|(_, f)| is_empty_user(f))
            .map(|(_, f)| f["R"].as_u64().unwrap())
            .collect();
        revisions.dedup();
        assert_eq!(revisions, [2, 3, 4, 5, 6]);
    }

    #[test]
    fn missing_password_is_password_required() {
        let result = open(&encrypted("r6-aes-256-user.pdf"));
        assert_eq!(result.err(), Some(Error::PasswordRequired));
    }

    #[test]
    fn wrong_password_is_wrong_password_for_every_revision() {
        for (name, facts) in files().into_iter().filter(|(_, f)| !is_empty_user(f)) {
            let result = open_with(&encrypted(&name), with_password("not-it"));
            assert_eq!(
                result.err().map(|e| e.code()),
                Some("wrong-password"),
                "{name} (R{})",
                facts["R"]
            );
        }
    }

    #[test]
    fn wrong_password_on_an_empty_user_password_file_is_still_wrong() {
        let result = open_with(&encrypted("r4-aes-128-empty.pdf"), with_password("not-it"));
        assert_eq!(result.err(), Some(Error::WrongPassword));
    }

    // NFKC is SASLprep's part that needs the `saslprep` feature (laminapdf-syntax's mapping step
    // runs in every build).
    #[cfg(feature = "saslprep")]
    #[test]
    fn r6_password_is_prepared_with_saslprep() {
        // NFD input normalises (NFKC) to the NFC password the file was written with.
        let nfd = "Pa\u{308}sswo\u{308}rt-6";
        let doc = open_with(&encrypted("r6-aes-256-user.pdf"), with_password(nfd)).unwrap();
        assert_eq!(doc.matched_password(), Some(PasswordKind::User));
    }

    #[test]
    fn password_as_bytes_is_accepted() {
        let doc = open_with(
            &encrypted("r3-rc4-128-user.pdf"),
            OpenOptions::default().with_password(b"user-pw".to_vec()),
        )
        .unwrap();
        assert!(doc.is_encrypted());
    }

    #[test]
    fn corpus_enc_pdf_now_decrypts_its_title() {
        let doc = open(&fixture("corpus/enc.pdf")).unwrap();
        assert_eq!(info(&doc)["Title"], "Secret");
    }

    #[test]
    fn unencrypted_document_reports_no_encryption() {
        let doc = open(&fixture("encrypted/source.pdf")).unwrap();
        assert_eq!(
            (doc.is_encrypted(), doc.encryption(), doc.permissions()),
            (false, None, None)
        );
    }
}

mod summary {
    use super::*;

    /// capability: G.11 core read
    #[test]
    fn encryption_matches_the_encrypt_dictionary() {
        let mut failures = Vec::new();
        for (name, facts) in files() {
            let doc = open_with(
                &encrypted(&name),
                with_password(facts["owner_password"].as_str().unwrap()),
            )
            .unwrap();
            let enc = doc.encryption().unwrap();
            let ours = (
                u64::from(enc.version),
                u64::from(enc.revision),
                u64::from(enc.key_bits),
                enc.encrypt_metadata,
            );
            let theirs = (
                facts["V"].as_u64().unwrap(),
                facts["R"].as_u64().unwrap(),
                facts["key_bits"].as_u64().unwrap(),
                facts["encrypt_metadata"].as_bool().unwrap(),
            );
            if ours != theirs {
                failures.push(format!("{name}: ours {ours:?} vs expected {theirs:?}"));
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn r3_and_r4_rc4_files_differ_in_revision() {
        let r3 = open(&encrypted("r3-rc4-128-empty.pdf")).unwrap();
        let r4 = open(&encrypted("r4-rc4-128-empty.pdf")).unwrap();
        let summary = |d: &Document| d.encryption().map(|e| (e.version, e.revision, e.stream_cipher));
        assert_eq!(
            (summary(&r3), summary(&r4)),
            (Some((2, 3, Cipher::Rc4)), Some((4, 4, Cipher::Rc4)))
        );
    }

    #[test]
    fn v4_key_length_comes_from_the_crypt_filter_length() {
        let aes = open_with(&encrypted("v4-aes-128-cf-length.pdf"), with_password("user-pw")).unwrap();
        let rc4 = open_with(&encrypted("v4-rc4-40-cf-length.pdf"), with_password("user-pw")).unwrap();
        let bits = |d: &Document| d.encryption().map(|e| (e.key_bits, e.stream_cipher));
        assert_eq!(
            (bits(&aes), bits(&rc4)),
            (Some((128, Cipher::Aes128)), Some((40, Cipher::Rc4)))
        );
    }

    #[test]
    fn r6_password_that_is_both_is_the_owner_password() {
        let doc = open_with(&encrypted("r6-aes-256-same.pdf"), with_password("same-pw")).unwrap();
        assert_eq!(doc.matched_password(), Some(PasswordKind::Owner));
    }

    #[test]
    fn crypt_filter_is_not_passed_through_in_an_unencrypted_document() {
        let bytes = b"%PDF-1.7\n1 0 obj << /Type /Catalog >> endobj\n2 0 obj << /Length 1 /Filter /Crypt >>\nstream\nx\nendstream\nendobj\ntrailer << /Root 1 0 R >>\n";
        let doc = open(bytes).unwrap();
        let Object::Stream(s) = &*doc.object(2).unwrap() else {
            panic!("not a stream")
        };
        assert_eq!(doc.decode_stream(s).err().map(|e| e.code()), Some("unsupported-filter"));
    }

    #[test]
    fn identity_stmf_file_uses_identity_streams_and_aes_strings() {
        let doc = open_with(&encrypted("identity-stmf.pdf"), with_password("user-pw")).unwrap();
        let enc = doc.encryption().unwrap();
        assert_eq!(
            (enc.stream_cipher, enc.string_cipher),
            (Cipher::Identity, Cipher::Aes128)
        );
    }

    fn permission_map(p: Permissions) -> Value {
        serde_json::json!({
            "accessibility": p.accessibility(),
            "extract": p.extract(),
            "print_low_res": p.print_low_res(),
            "print_high_res": p.print_high_res(),
            "assemble": p.assemble(),
            "fill_forms": p.fill_forms(),
            "annotate": p.annotate(),
            "modify": p.modify(),
        })
    }

    /// capability: G.04 core read
    #[test]
    fn permissions_match_qpdf_show_encryption() {
        let mut failures = Vec::new();
        for (name, facts) in files() {
            let doc = open_with(
                &encrypted(&name),
                with_password(facts["owner_password"].as_str().unwrap()),
            )
            .unwrap();
            let p = doc.permissions().unwrap();
            let raw = i64::from(p.raw().cast_signed());
            // qpdf's "modify forms" is bit 9 alone from R3; ISO 32000-1 Table 22 also lets bit 6
            // fill fields ("bit 9 … even if bit 6 is clear"), which qpdf reports as "modify
            // annotations".
            let mut expected = facts["permissions"].clone();
            let fill = expected["fill_forms"].as_bool().unwrap() || expected["annotate"].as_bool().unwrap();
            expected["fill_forms"] = Value::Bool(fill);
            if raw != facts["P"].as_i64().unwrap() || permission_map(p) != expected {
                failures.push(format!("{name}: ours P {raw} {} vs {expected}", permission_map(p)));
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
}

mod exemptions {
    use super::*;

    #[test]
    fn cleartext_metadata_xmp_is_stored_and_read_as_plaintext() {
        let name = "r4-aes-128-cleartext-metadata-user.pdf";
        let bytes = encrypted(name);
        let doc = open_with(&bytes, with_password("user-pw")).unwrap();
        let source = open(&fixture("encrypted/source.pdf")).unwrap();
        let stored_plain = bytes.windows(10).any(|w| w == b"<x:xmpmeta");
        assert_eq!(
            (
                stored_plain,
                doc.encryption().unwrap().encrypt_metadata,
                xmp(&doc) == xmp(&source)
            ),
            (true, false, true)
        );
    }

    #[test]
    fn cleartext_metadata_still_decrypts_everything_else() {
        let doc = open(&encrypted("r4-aes-128-cleartext-metadata-empty.pdf")).unwrap();
        assert_eq!(info(&doc)["Title"], "Secret document");
    }

    #[test]
    fn trailer_id_is_not_decrypted() {
        let bytes = encrypted("r4-aes-128-empty.pdf");
        let doc = open(&bytes).unwrap();
        let Some(Object::Array(id)) = doc.trailer().get("ID") else {
            panic!("no /ID")
        };
        let Object::String(id0) = &id[0] else {
            panic!("/ID[0] is not a string")
        };
        assert_eq!(id0.bytes.len(), 16);
    }

    #[test]
    fn annotation_contents_is_decrypted() {
        let doc = open(&encrypted("r6-aes-256-empty.pdf")).unwrap();
        let page = &doc.pages().unwrap()[0];
        let annots = doc.deref(page.dict.get("Annots").unwrap()).unwrap().into_owned();
        let annot = dict_of(&doc, &annots.as_array().unwrap()[0]);
        assert_eq!(string_of(&doc, annot.get("Contents").unwrap()), "An annotation note");
    }
}

/// Appends an incremental update to `base` (by hand: the writer refuses encrypted bases) that
/// replaces `/Info` with object 99, `body`.
fn append_info(base: &[u8], body: &str) -> Vec<u8> {
    let doc = open(base).unwrap();
    let trailer = doc.trailer().clone();
    let startxref = doc.startxref().unwrap();
    let mut out = base.to_vec();
    let offset = out.len() + 1;
    out.extend_from_slice(format!("\n99 0 obj\n{body}\nendobj\n").as_bytes());
    let xref = out.len();
    out.extend_from_slice(format!("xref\n99 1\n{offset:010} 00000 n\r\n").as_bytes());
    let mut new = laminapdf_cos::Dict::new();
    for key in ["Root", "Encrypt", "ID"] {
        new.insert(key, trailer.get(key).unwrap().clone());
    }
    new.insert("Info", Object::Ref(ObjRef::new(99, 0)));
    new.insert("Size", 100);
    new.insert("Prev", i64::try_from(startxref).unwrap());
    out.extend_from_slice(b"trailer\n");
    out.extend_from_slice(&Object::Dict(new).to_bytes());
    out.extend_from_slice(format!("\nstartxref\n{xref}\n%%EOF\n").as_bytes());
    out
}

mod mixed_plaintext {
    use super::*;

    // PDFex shape (research ch. 04 §7.7): a plaintext object spliced into an encrypted file.
    // Per spec it is decrypted like any other, which yields garbage (or, for AES, data that is
    // not whole blocks and is kept as is with a warning); it never panics.
    #[test]
    fn plaintext_string_in_an_aes_file_is_kept_with_a_warning() {
        let bytes = append_info(&encrypted("r6-aes-256-empty.pdf"), "<< /Title (spliced) >>");
        let doc = open(&bytes).unwrap();
        let title = info(&doc)["Title"].clone();
        assert_eq!(
            (title, doc.warnings()),
            ("spliced".into(), vec![Warning::AesPaddingInvalid { object: 99 }])
        );
    }

    #[test]
    fn plaintext_string_in_an_rc4_file_decrypts_to_garbage() {
        let bytes = append_info(&encrypted("r3-rc4-128-empty.pdf"), "<< /Title (spliced) >>");
        let doc = open(&bytes).unwrap();
        assert_ne!(info(&doc)["Title"], "spliced");
    }

    #[test]
    fn strict_mode_refuses_malformed_aes_data() {
        let bytes = append_info(&encrypted("r4-aes-128-empty.pdf"), "<< /Title (spliced) >>");
        let doc = open_with(&bytes, OpenOptions::default().with_strict(true)).unwrap();
        let title = doc.resolve(ObjRef::new(99, 0)).err();
        assert_eq!(title.map(|e| e.code()), Some("strict"));
    }
}

mod tampering {
    use super::*;

    /// `bytes` with the first `from` replaced by `to` (same length, so offsets stay valid).
    fn patch(bytes: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
        assert_eq!(from.len(), to.len());
        let at = bytes.windows(from.len()).position(|w| w == from).unwrap();
        let mut out = bytes.to_vec();
        out[at..at + to.len()].copy_from_slice(to);
        out
    }

    #[test]
    fn r6_with_a_tampered_p_opens_with_perms_mismatch() {
        let bytes = patch(&encrypted("r6-aes-256-user.pdf"), b"/P -2324", b"/P -2320");
        let doc = open_with(&bytes, with_password("owner-pw")).unwrap();
        assert_eq!(doc.warnings(), [Warning::PermsMismatch]);
    }

    #[test]
    fn r6_with_a_tampered_p_is_refused_in_strict_mode() {
        let bytes = patch(&encrypted("r6-aes-256-user.pdf"), b"/P -2324", b"/P -2320");
        let result = open_with(&bytes, with_password("owner-pw").with_strict(true));
        assert_eq!(
            result.err(),
            Some(Error::Strict {
                warning: Warning::PermsMismatch
            })
        );
    }

    #[test]
    fn r4_with_a_tampered_p_no_longer_opens() {
        // R2–R4 hash /P into the file key, so changing it breaks authentication.
        let bytes = patch(&encrypted("r4-rc4-128-user.pdf"), b"/P -3076", b"/P -3072");
        let result = open_with(&bytes, with_password("user-pw"));
        assert_eq!(result.err(), Some(Error::WrongPassword));
    }

    #[test]
    fn public_key_handler_is_unsupported() {
        let bytes = b"%PDF-1.7\n1 0 obj << /Type /Catalog >> endobj\n2 0 obj << /Filter /Adobe.PubSec /SubFilter /adbe.pkcs7.s5 /V 4 /R 4 >> endobj\ntrailer << /Root 1 0 R /Encrypt 2 0 R >>\n";
        let result = open(bytes);
        assert_eq!(
            result.err(),
            Some(Error::UnsupportedSecurityHandler {
                filter: "Adobe.PubSec".into(),
                version: Some(4),
                revision: Some(4),
            })
        );
    }

    #[test]
    fn encrypt_null_counts_as_absent() {
        let bytes = b"%PDF-1.7\n1 0 obj << /Type /Catalog >> endobj\ntrailer << /Root 1 0 R /Encrypt null >>\n";
        let doc = open(bytes).unwrap();
        assert_eq!((doc.is_encrypted(), doc.encryption()), (false, None));
    }

    #[test]
    fn encrypt_referring_to_a_missing_object_counts_as_absent() {
        let bytes = b"%PDF-1.7\n1 0 obj << /Type /Catalog >> endobj\ntrailer << /Root 1 0 R /Encrypt 9 0 R >>\n";
        let doc = open(bytes).unwrap();
        assert!(!doc.is_encrypted());
    }

    #[test]
    fn encrypt_that_is_not_a_dictionary_is_invalid_structure() {
        let bytes = b"%PDF-1.7\n1 0 obj << /Type /Catalog >> endobj\ntrailer << /Root 1 0 R /Encrypt 42 >>\n";
        assert_eq!(open(bytes).err().map(|e| e.code()), Some("invalid-structure"));
    }

    /// `bytes` with `startxref` pointing nowhere, so the xref has to be rebuilt by scanning.
    fn break_startxref(bytes: &[u8]) -> Vec<u8> {
        let at = bytes.windows(9).rposition(|w| w == b"startxref").unwrap();
        let mut broken = bytes[..at].to_vec();
        broken.extend_from_slice(b"startxref\n999999\n%%EOF\n");
        broken
    }

    #[test]
    fn repair_finds_a_catalog_inside_an_encrypted_object_stream() {
        let bytes = break_startxref(&encrypted("objstm-catalog-aes-128.pdf"));
        let doc = open_with(&bytes, with_password("user-pw")).unwrap();
        assert_eq!(
            (
                doc.was_repaired(),
                doc.page_count().unwrap(),
                info(&doc)["Title"].clone()
            ),
            (true, 2, "Secret document".into())
        );
    }

    #[test]
    fn repair_reports_malformed_aes_in_an_object_stream() {
        let bytes = break_startxref(&encrypted("objstm-bad-padding-aes-128.pdf"));
        let doc = open_with(&bytes, with_password("user-pw")).unwrap();
        assert_eq!(
            doc.warnings(),
            [Warning::XrefRepaired, Warning::AesPaddingInvalid { object: 16 }]
        );
    }

    #[test]
    fn malformed_aes_in_an_object_stream_is_reported_when_loaded() {
        let doc = open_with(&encrypted("objstm-bad-padding-aes-128.pdf"), with_password("user-pw")).unwrap();
        assert_eq!(doc.warnings(), [Warning::AesPaddingInvalid { object: 16 }]);
    }

    #[test]
    fn damaged_encrypt_dictionary_on_the_stored_xref_falls_back_to_repair() {
        // The stored xref points at an /Encrypt object that is an array (same length, so every
        // entry stays valid); a later, intact copy of it is what repair finds.
        let bytes = encrypted("r3-rc4-128-empty.pdf");
        let doc = open(&bytes).unwrap();
        let num = doc.trailer().get("Encrypt").unwrap().as_ref().unwrap().num;
        let header = format!("\n{num} 0 obj\n");
        let start = bytes
            .windows(header.len())
            .position(|w| w == header.as_bytes())
            .unwrap()
            + 1;
        let end = start + bytes[start..].windows(6).position(|w| w == b"endobj").unwrap() + 6;
        let original = bytes[start..end].to_vec();
        let mut damaged = bytes;
        let body = &mut damaged[start..end];
        let open_at = body.windows(2).position(|w| w == b"<<").unwrap();
        body[open_at..open_at + 2].copy_from_slice(b"[ ");
        let close_at = body.windows(2).rposition(|w| w == b">>").unwrap();
        body[close_at..close_at + 2].copy_from_slice(b" ]");
        let at = damaged.windows(9).rposition(|w| w == b"startxref").unwrap();
        let mut appended = damaged[..at].to_vec();
        appended.extend_from_slice(b"\n");
        appended.extend_from_slice(&original);
        appended.extend_from_slice(b"\n");
        appended.extend_from_slice(&damaged[at..]);
        let doc = open(&appended).unwrap();
        assert_eq!(
            (doc.was_repaired(), info(&doc)["Title"].clone()),
            (true, "Secret document".into())
        );
    }

    #[test]
    fn repaired_encrypted_file_with_object_streams_still_decrypts() {
        // Breaking startxref forces repair, which must decrypt object streams to find members.
        let bytes = encrypted("r6-aes-256-empty.pdf");
        let at = bytes.windows(9).rposition(|w| w == b"startxref").unwrap();
        let mut broken = bytes[..at].to_vec();
        broken.extend_from_slice(b"startxref\n999999\n%%EOF\n");
        let doc = open(&broken).unwrap();
        assert_eq!(
            (
                doc.was_repaired(),
                outline_titles(&doc).len(),
                info(&doc)["Title"].clone()
            ),
            (true, 3, "Secret document".into())
        );
    }
}

#[test]
fn incremental_update_on_an_encrypted_base_still_fails() {
    let doc = open(&encrypted("r6-aes-256-empty.pdf")).unwrap();
    let result = incremental_update(&doc, &[], XrefEol::CrLf);
    assert_eq!(result, Err(Error::EncryptedBase));
}

#[test]
fn passwords_txt_lists_every_fixture_password() {
    let listed = String::from_utf8(fixture("../fuzz/seeds/open/passwords.txt")).unwrap();
    let listed: Vec<&str> = listed.lines().collect();
    for (name, facts) in files() {
        for key in ["user_password", "owner_password"] {
            let pw = facts[key].as_str().unwrap();
            assert!(
                pw.is_empty() || listed.contains(&pw),
                "{name}: {key} {pw:?} not in passwords.txt"
            );
        }
    }
}
