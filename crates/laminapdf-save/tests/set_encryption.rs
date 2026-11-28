//! `SetEncryption` (E11b, ISO 32000-2 §7.6.4): a file written with passwords and permissions
//! opens only with them, leaks none of its plaintext, reads back what was asked and writes the
//! same bytes for the same entropy.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::fixture;
use laminapdf_cos::{Dict, Object, OpenOptions, PdfString, Stream};
use laminapdf_ops::{Document, EncryptionPermissions, Op, Origin, PrintPermission, SaveState, SetEncryption};
use laminapdf_save::test_support::Writer;
use laminapdf_save::{Save, SaveError, SaveMode, SaveOptions};
use laminapdf_syntax::crypt::PasswordKind;

/// A string that sits in plaintext in the source, once in each place a file keeps text.
const SENTINEL: &str = "LAMINA-SENTINEL-7f3a";

fn dict(entries: Vec<(&str, Object)>) -> Dict {
    entries.into_iter().collect()
}

fn text(s: &str) -> Object {
    Object::String(PdfString::literal(s))
}

/// A one-page file with the sentinel in its page content, Info, an annotation, a form value and
/// an attachment, none of it compressed.
fn sentinel_file() -> Vec<u8> {
    let mut w = Writer::new(1, 7);
    let content = w.add(Object::Stream(Stream::new(
        Dict::new(),
        format!("BT /F1 12 Tf 10 10 Td ({SENTINEL}) Tj ET").into_bytes(),
    )));
    let attachment = w.add(Object::Stream(Stream::new(
        dict(vec![("Type", Object::name("EmbeddedFile"))]),
        SENTINEL.as_bytes().to_vec(),
    )));
    let spec = w.add(dict(vec![
        ("Type", Object::name("Filespec")),
        ("F", text("note.txt")),
        ("EF", Object::Dict(dict(vec![("F", Object::Ref(attachment))]))),
    ]));
    let annot = w.add(dict(vec![
        ("Type", Object::name("Annot")),
        ("Subtype", Object::name("Text")),
        ("Rect", vec![Object::from(0), Object::from(0), Object::from(20), Object::from(20)].into()),
        ("Contents", text(SENTINEL)),
    ]));
    let font = w.add(dict(vec![
        ("Type", Object::name("Font")),
        ("Subtype", Object::name("Type1")),
        ("BaseFont", Object::name("Helvetica")),
    ]));
    let field = w.add(dict(vec![
        ("FT", Object::name("Tx")),
        ("T", text("name")),
        ("V", text(SENTINEL)),
    ]));
    let page_id = w.reserve();
    let pages = w.add(dict(vec![
        ("Type", Object::name("Pages")),
        ("Kids", vec![Object::Ref(page_id)].into()),
        ("Count", 1.into()),
    ]));
    w.set(
        page_id,
        Object::Dict(dict(vec![
            ("Type", Object::name("Page")),
            ("Parent", Object::Ref(pages)),
            ("MediaBox", vec![Object::from(0), Object::from(0), Object::from(200), Object::from(200)].into()),
            ("Contents", Object::Ref(content)),
            ("Resources", Object::Dict(dict(vec![("Font", Object::Dict(dict(vec![("F1", Object::Ref(font))])))]))),
            ("Annots", vec![Object::Ref(annot)].into()),
        ])),
    );
    let names = w.add(dict(vec![(
        "EmbeddedFiles",
        Object::Dict(dict(vec![("Names", vec![text("note.txt"), Object::Ref(spec)].into())])),
    )]));
    let root = w.add(dict(vec![
        ("Type", Object::name("Catalog")),
        ("Pages", Object::Ref(pages)),
        ("Names", Object::Ref(names)),
        ("AcroForm", Object::Dict(dict(vec![("Fields", vec![Object::Ref(field)].into())]))),
    ]));
    let info = w.add(dict(vec![("Title", text(SENTINEL))]));
    w.finish(&[("Root", root), ("Info", info)].into_iter().collect()).unwrap()
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

fn open(bytes: Vec<u8>) -> Document {
    Document::open(bytes, OpenOptions::default()).unwrap()
}

/// `bytes` encrypted with `op`, as a full rewrite under `seed`.
fn protect(bytes: Vec<u8>, op: SetEncryption, seed: u8) -> (Vec<u8>, laminapdf_save::SaveReport) {
    let mut doc = open(bytes);
    doc.apply_all(Origin::Local, [Op::from(op)]).unwrap();
    let mut out = Vec::new();
    let options = SaveOptions::new().with_entropy_seed([seed; 32]);
    let report = doc.save(&mut out, &options).unwrap();
    (out, report)
}

fn reopen(bytes: &[u8], password: &str) -> Result<laminapdf_model::Document, laminapdf_cos::Error> {
    laminapdf_model::Document::open(bytes.to_vec(), OpenOptions::default().with_password(password))
}

#[test]
fn a_protected_file_leaks_none_of_the_plaintext_it_held() {
    assert!(contains(&sentinel_file(), SENTINEL.as_bytes()), "the source has it in plaintext");
    let (out, report) = protect(sentinel_file(), SetEncryption::new("open-pw", "owner-pw"), 1);
    assert!(report.encrypted() && !contains(&out, SENTINEL.as_bytes()));
    // Not even a fragment of the content stream's text operators survives.
    assert!(!contains(&out, b"Tj") && !contains(&out, b"note.txt"));
}

#[test]
fn an_incremental_save_of_the_protected_file_leaks_nothing_either() {
    let (out, _) = protect(sentinel_file(), SetEncryption::new("open-pw", "owner-pw"), 1);
    let mut doc = Document::open(out, OpenOptions::default().with_password("open-pw")).unwrap();
    common::save::set_title(&mut doc, SENTINEL);
    let mut again = Vec::new();
    let report = doc
        .save(&mut again, &SaveOptions::new().with_entropy_seed([2; 32]))
        .unwrap();
    assert_eq!((report.mode(), report.encrypted()), (SaveMode::Incremental, true));
    assert!(!contains(&again, SENTINEL.as_bytes()));
}

#[test]
fn the_right_passwords_open_the_file_and_nothing_else_does() {
    let (out, _) = protect(sentinel_file(), SetEncryption::new("open-pw", "owner-pw"), 1);
    let user = reopen(&out, "open-pw").unwrap();
    let owner = reopen(&out, "owner-pw").unwrap();
    assert_eq!(
        (user.matched_password(), owner.matched_password()),
        (Some(PasswordKind::User), Some(PasswordKind::Owner))
    );
    assert_eq!(reopen(&out, "wrong").err().map(|e| e.code()), Some("wrong-password"));
    assert_eq!(
        laminapdf_model::Document::open(out.clone(), OpenOptions::default()).err().map(|e| e.code()),
        Some("password-required")
    );
    // The file reads the same under either password.
    assert_eq!(user.info().unwrap().unwrap().title().unwrap().as_deref(), Some(SENTINEL));
    assert_eq!(owner.page_count().unwrap(), 1);
}

#[test]
fn the_handler_is_aes_256_revision_6_with_a_consistent_perms_entry() {
    let (out, _) = protect(sentinel_file(), SetEncryption::new("open-pw", "owner-pw"), 1);
    let doc = reopen(&out, "open-pw").unwrap();
    let e = doc.encryption().unwrap();
    assert_eq!((e.version, e.revision, e.key_bits, e.encrypt_metadata), (5, 6, 256, true));
    assert_eq!((e.string_cipher.name(), e.stream_cipher.name()), ("aes-256", "aes-256"));
    assert!(!doc.warnings().iter().any(|w| w.code() == "perms-mismatch"), "/Perms matches /P");
}

#[test]
fn an_empty_user_password_opens_without_a_prompt() {
    let (out, _) = protect(sentinel_file(), SetEncryption::new("", "owner-pw"), 1);
    let doc = laminapdf_model::Document::open(out, OpenOptions::default()).unwrap();
    assert_eq!(doc.matched_password(), Some(PasswordKind::User));
}

#[test]
fn an_empty_owner_password_is_made_and_reported_once() {
    let (out, report) = protect(sentinel_file(), SetEncryption::new("open-pw", ""), 1);
    let made = report.generated_owner_password().expect("a made password");
    let made = std::str::from_utf8(made.expose()).unwrap();
    assert_eq!(made.len(), 32);
    assert_eq!(reopen(&out, made).unwrap().matched_password(), Some(PasswordKind::Owner));
    let (_, given) = protect(sentinel_file(), SetEncryption::new("open-pw", "owner-pw"), 1);
    assert!(given.generated_owner_password().is_none());
}

#[test]
fn each_permission_control_reaches_the_p_entry_and_bit_10_stays_set() {
    let none = EncryptionPermissions::default()
        .with_print(PrintPermission::None)
        .with_copy(false)
        .with_fill_forms(false)
        .with_comment(false)
        .with_assemble(false);
    let (out, _) = protect(sentinel_file(), SetEncryption::new("open-pw", "owner-pw").with_permissions(none), 1);
    let p = reopen(&out, "open-pw").unwrap().permissions().unwrap();
    assert!(!p.print_low_res() && !p.extract() && !p.annotate() && !p.fill_forms() && !p.assemble());
    assert!(!p.modify(), "a permissions password turns editing the contents off");
    assert!(p.accessibility(), "screen readers stay allowed");

    let low = EncryptionPermissions::default().with_print(PrintPermission::LowResolution);
    let (out, _) = protect(sentinel_file(), SetEncryption::new("open-pw", "owner-pw").with_permissions(low), 1);
    let p = reopen(&out, "open-pw").unwrap().permissions().unwrap();
    assert!((p.print_low_res(), p.print_high_res(), p.extract(), p.annotate(), p.assemble()) == (true, false, true, true, true));
}

#[test]
fn a_file_with_only_a_user_password_grants_every_permission() {
    let (out, _) = protect(sentinel_file(), SetEncryption::new("open-pw", ""), 1);
    let p = reopen(&out, "open-pw").unwrap().permissions().unwrap();
    assert_eq!(p.raw(), 0xFFFF_FFFC, "all bits set but the two reserved low ones");
}

#[test]
fn encrypt_metadata_false_is_written_and_read_back() {
    let op = SetEncryption::new("open-pw", "owner-pw").with_encrypt_metadata(false);
    let (out, _) = protect(sentinel_file(), op, 1);
    assert!(!reopen(&out, "open-pw").unwrap().encryption().unwrap().encrypt_metadata);
}

#[test]
fn the_same_entropy_writes_the_same_bytes() {
    let save = |seed| protect(sentinel_file(), SetEncryption::new("open-pw", "owner-pw"), seed).0;
    assert_eq!(save(5), save(5));
    assert_ne!(save(5), save(6));
}

#[test]
fn the_save_rebases_onto_the_encrypted_file_and_edits_go_on() {
    let mut doc = open(sentinel_file());
    doc.apply_all(Origin::Local, [Op::from(SetEncryption::new("open-pw", "owner-pw"))]).unwrap();
    assert!(doc.has_unsaved_changes() && doc.pending_encryption().is_some());
    doc.save_and_rebase(Vec::new(), &SaveOptions::new().with_entropy_seed([3; 32])).unwrap();
    assert_eq!(
        (doc.save_state(), doc.is_encrypted(), doc.has_unsaved_changes(), doc.pending_encryption().is_none()),
        (SaveState::Clean, true, false, true)
    );
    common::save::set_title(&mut doc, "After");
    let report = doc.save(&mut Vec::new(), &SaveOptions::new().with_entropy_seed([4; 32])).unwrap();
    assert_eq!((report.mode(), report.encrypted()), (SaveMode::Incremental, true));
}

#[test]
fn changing_the_passwords_of_an_encrypted_file_replaces_the_old_ones() {
    let (first, _) = protect(sentinel_file(), SetEncryption::new("old-pw", "old-owner"), 1);
    let mut doc = Document::open(first, OpenOptions::default().with_password("old-pw")).unwrap();
    doc.apply_all(Origin::Local, [Op::from(SetEncryption::new("new-pw", "new-owner"))]).unwrap();
    let mut out = Vec::new();
    doc.save(&mut out, &SaveOptions::new().with_entropy_seed([2; 32])).unwrap();
    assert!(reopen(&out, "new-pw").is_ok());
    assert_eq!(reopen(&out, "old-pw").err().map(|e| e.code()), Some("wrong-password"));
    assert!(!contains(&out, SENTINEL.as_bytes()));
}

#[test]
fn a_request_with_neither_password_is_refused_when_it_is_applied() {
    let mut doc = open(sentinel_file());
    let err = doc.apply_all(Origin::Local, [Op::from(SetEncryption::new("", ""))]).unwrap_err();
    assert_eq!(err.code(), "encryption-no-password");
    assert!(doc.pending_encryption().is_none() && !doc.has_unsaved_changes());
}

#[test]
fn a_file_that_claims_pdfa_is_refused() {
    let mut doc = open(fixture("forms/pdfa-form.pdf"));
    let err = doc.apply_all(Origin::Local, [Op::from(SetEncryption::new("open-pw", ""))]).unwrap_err();
    assert_eq!(err.code(), "encryption-pdfa-conflict");
}

#[test]
fn a_save_without_entropy_writes_nothing() {
    let mut doc = open(sentinel_file());
    doc.apply_all(Origin::Local, [Op::from(SetEncryption::new("open-pw", ""))]).unwrap();
    let mut out = Vec::new();
    let err = doc.save(&mut out, &SaveOptions::new()).unwrap_err();
    assert_eq!((err, out.len(), doc.pending_encryption().is_some()), (SaveError::EncryptionNoEntropy, 0, true));
}

#[test]
fn cancelling_drops_the_request_and_its_passwords() {
    let mut doc = open(sentinel_file());
    doc.apply_all(Origin::Local, [Op::from(SetEncryption::new("open-pw", ""))]).unwrap();
    doc.cancel_encryption().unwrap();
    assert!(doc.pending_encryption().is_none() && !doc.has_unsaved_changes());
    assert!(!doc.is_dirty());
}

#[test]
fn a_failed_transaction_leaves_no_pending_encryption() {
    let mut doc = open(sentinel_file());
    let result = doc.transact(Origin::Local, |tx| {
        tx.apply(SetEncryption::new("open-pw", ""))?;
        tx.apply(SetEncryption::new("", ""))?;
        Ok(())
    });
    assert!(result.is_err() && doc.pending_encryption().is_none());
}

#[test]
fn the_op_is_a_barrier_that_requires_a_rewrite_in_class_document() {
    let op = Op::from(SetEncryption::new("open-pw", ""));
    assert_eq!((op.requires_rewrite(), op.class()), (true, laminapdf_ops::ModificationClass::Document));
    let mut doc = open(sentinel_file());
    doc.apply_all(Origin::Local, [op]).unwrap();
    let err = doc
        .save(&mut Vec::new(), &SaveOptions::new().with_mode(SaveMode::Incremental).with_entropy_seed([1; 32]))
        .unwrap_err();
    assert_eq!(err.code(), "requires-rewrite");
}

#[test]
fn an_op_log_never_holds_a_password() {
    let op = Op::from(SetEncryption::new("sesame", "owner-secret"));
    let json = serde_json::to_string(&op).unwrap();
    assert!(!json.contains("sesame") && !json.contains("owner-secret"), "{json}");
    let back: Op = serde_json::from_str(&json).unwrap();
    let err = open(sentinel_file()).apply_all(Origin::Local, [back]).unwrap_err();
    assert_eq!(err.code(), "secret-missing");
}

#[test]
fn no_error_or_debug_output_names_a_password() {
    let op = Op::from(SetEncryption::new("sesame", "owner-secret"));
    let mut doc = open(sentinel_file());
    doc.apply_all(Origin::Local, [op.clone()]).unwrap();
    let shown = format!("{op:?} {:?} {:?}", doc.pending_encryption(), SaveOptions::new().with_entropy_seed([1; 32]));
    assert!(!shown.contains("sesame") && !shown.contains("owner-secret"), "{shown}");
    let err = doc.save(&mut Vec::new(), &SaveOptions::new()).unwrap_err();
    assert!(!format!("{err} {err:?}").contains("sesame"));
}

/// The files `fixtures/encrypted/written/` holds, byte for byte (`UPDATE_GOLDEN=1` rewrites them,
/// then `tools/fixtures/encrypted_facts.py` checks them against qpdf and pypdf): the sentinel file
/// protected with the controls of the Protect dialog, under a fixed entropy seed.
#[test]
fn the_written_goldens_are_reproduced_byte_for_byte() {
    let limits = EncryptionPermissions::default()
        .with_print(PrintPermission::LowResolution)
        .with_copy(false)
        .with_comment(false);
    let cases = [
        ("r6-user.pdf", SetEncryption::new("open-pw", "owner-pw")),
        ("r6-owner-permissions.pdf", SetEncryption::new("", "owner-pw").with_permissions(limits)),
        ("r6-no-metadata-encryption.pdf", SetEncryption::new("open-pw", "owner-pw").with_encrypt_metadata(false)),
    ];
    for (name, op) in cases {
        let (bytes, _) = protect(sentinel_file(), op, 1);
        let path = common::fixtures_dir().join("encrypted/written").join(name);
        if std::env::var_os("UPDATE_GOLDEN").is_some() {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, &bytes).unwrap();
        }
        let golden = std::fs::read(&path).unwrap_or_else(|_| panic!("{name}: run with UPDATE_GOLDEN=1"));
        assert!(golden == bytes, "{name} differs from its golden: run with UPDATE_GOLDEN=1 and review the diff");
    }
}

/// With `LAMINA_ENCRYPT_OUT` set to a folder, writes the files other readers check
/// (`tools/roundtrip/encrypted_outputs.py`: qpdf `--check`, pypdf, pdftotext); without it, only
/// checks the files exist in memory.
#[test]
fn the_files_other_readers_check_are_written_on_request() {
    let none = EncryptionPermissions::default()
        .with_print(PrintPermission::LowResolution)
        .with_copy(false)
        .with_comment(false);
    let cases = [
        ("r6-user.pdf", SetEncryption::new("open-pw", "owner-pw")),
        ("r6-empty-user.pdf", SetEncryption::new("", "owner-pw")),
        ("r6-permissions.pdf", SetEncryption::new("open-pw", "owner-pw").with_permissions(none)),
        ("r6-clear-metadata.pdf", SetEncryption::new("open-pw", "owner-pw").with_encrypt_metadata(false)),
        ("r6-unicode.pdf", SetEncryption::new("Pässwört-6", "владелец")),
    ];
    for (name, op) in cases {
        let (out, _) = protect(sentinel_file(), op, 1);
        assert!(out.starts_with(b"%PDF-"), "{name}");
        if let Some(dir) = std::env::var_os("LAMINA_ENCRYPT_OUT") {
            std::fs::write(std::path::Path::new(&dir).join(name), out).unwrap();
        }
    }
}

/// pypdf and qpdf (`fixtures/encrypted/written/facts.py`) read each written golden as lamina does:
/// the same pages, title, decoded page content, revision and `/P`, and the owner password opens it
/// as the owner's.
#[test]
fn lamina_reads_the_written_goldens_as_pypdf_and_qpdf_do() {
    use sha2::{Digest, Sha256};
    let expected: serde_json::Value =
        serde_json::from_slice(&fixture("encrypted/written/expected.json")).unwrap();
    let owner = expected["owner_password"].as_str().unwrap();
    for (name, want) in expected["files"].as_object().unwrap() {
        let bytes = fixture(&format!("encrypted/written/{name}"));
        let doc = reopen(&bytes, want["password"].as_str().unwrap()).unwrap();
        let pages = doc.pages().unwrap();
        let content = doc.page_content(pages.iter().next().unwrap().cos()).unwrap();
        assert_eq!(doc.page_count().unwrap() as u64, want["pages"].as_u64().unwrap(), "{name}");
        assert_eq!(
            doc.info().unwrap().unwrap().title().unwrap().as_deref(),
            want["title"].as_str(),
            "{name}"
        );
        assert_eq!(hex(&Sha256::digest(&content)), want["content_sha256"].as_str().unwrap(), "{name}");
        assert_eq!(u64::from(doc.encryption().unwrap().revision), want["revision"].as_u64().unwrap(), "{name}");
        assert_eq!(
            i64::from(i32::from_ne_bytes(doc.permissions().unwrap().raw().to_ne_bytes())),
            want["p"].as_i64().unwrap(),
            "{name}"
        );
        assert_eq!(reopen(&bytes, owner).unwrap().matched_password(), Some(PasswordKind::Owner), "{name}");
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
