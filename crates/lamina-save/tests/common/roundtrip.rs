//! The lamina half of the round-trip suite (ADR 0018 tier 7; todo B3c), shared by the
//! `roundtrip` example (which writes the outputs for `cargo xtask roundtrip`) and
//! `tests/roundtrip_suite.rs` (the fast lane's lamina-only run): for each fixture × proving op
//! × mode × overrides, saves with lamina, checks the planner's decision against the expected
//! one, reopens every output in lamina and compares its facts, and lists what pdf.js, `PDFium`
//! and qpdf must see in each.
//!
//! Expectations come from the fixture's category, not from the planner: plain corpus files
//! save in every mode; a repaired base only as a full rewrite; a signed base incrementally
//! (and refuses a requested full rewrite) unless the op breaks its certification; a signed and
//! repaired base only with the override, as a full rewrite; an encrypted base only with
//! `remove_encryption`, as a plaintext full rewrite.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test support: a panic is a test failure"
)]
#![allow(clippy::match_same_arms, reason = "one arm per category and mode reads as a table")]

use std::path::Path;

use lamina_cos::OpenOptions;
use lamina_model::Rotation;
use lamina_ops::{Document, InfoPatch, Op, Origin, RotatePage, SetInfo};
use lamina_save::{Save, SaveError, SaveMode, SaveOptions, SaveReport};
use serde_json::{Value, json};

/// What a fixture is, which decides every expectation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    /// Opens without repair, unsigned, unencrypted: every mode saves.
    Plain,
    /// A repaired base: only a full rewrite.
    Repaired,
    /// An approval signature or timestamp: incremental only.
    Signed,
    /// Certified: the proving ops break the certification; only with the override.
    Certified,
    /// Signed and repaired: only with the override, as a full rewrite.
    SignedRepaired,
    /// Encrypted: only with `remove_encryption`, as a full rewrite.
    Encrypted,
    /// Signed and encrypted: both overrides, as a full rewrite.
    SignedEncrypted,
}

/// What the `setInfo` cases write: `/Title`, `/Author`, `/Subject`, `/Keywords` (J.01).
const INFO: [(&str, &str); 4] = [
    ("Title", "Round trip"),
    ("Author", "lamina round trip"),
    ("Subject", "Every case of the suite"),
    ("Keywords", "lamina, round trip"),
];

/// The `rotatePage` rotations, one per fixture in turn, so each of 90, 180 and 270 (C.08) is
/// saved from every kind of fixture.
const ROTATIONS: [Rotation; 3] = [Rotation::Clockwise90, Rotation::Half, Rotation::Clockwise270];

/// The corpus files that open without repair and are neither signed nor encrypted.
pub const PLAIN_CORPUS: [&str; 10] = [
    "hello.pdf",
    "hello-incremental.pdf",
    "hello-xrefstm.pdf",
    "imaging-demo.pdf",
    "text-example.pdf",
    "raw.pdf",
    "pdflib.pdf",
    "fpdf2.pdf",
    "reportlab.pdf",
    "outline.pdf",
];

/// The goldens `lamina-build` writes (B5a): plain files too, so the full lane's qpdf, pdf.js
/// and PDFium checks cover the builder's output.
pub const PLAIN_BUILT: [&str; 2] = ["vectors.pdf", "images.pdf"];

/// The byte that damages `corpus/tampered.pdf`'s newest catalog so the file repairs to the
/// older, unsigned one: (offset, value), the mutation `lamina-cos`'s `policy.rs`
/// (`signed_and_tampered_with_damaged_catalog_repair_to_one_page`) checks.
pub const TAMPERED_CATALOG_DAMAGE: (usize, u8) = (590, 73);

/// `bytes` (`corpus/tampered.pdf`) with [`TAMPERED_CATALOG_DAMAGE`] applied.
pub fn damage_newest_catalog(mut bytes: Vec<u8>) -> Vec<u8> {
    let (at, value) = TAMPERED_CATALOG_DAMAGE;
    bytes[at] = value;
    bytes
}

fn fixtures(root: &Path) -> Vec<(String, Vec<u8>, Category)> {
    let read = |rel: &str| std::fs::read(root.join("fixtures").join(rel)).unwrap();
    let mut out = Vec::new();
    for name in PLAIN_CORPUS {
        out.push((
            format!("corpus/{name}"),
            read(&format!("corpus/{name}")),
            Category::Plain,
        ));
    }
    for name in PLAIN_BUILT {
        out.push((
            format!("golden/{name}"),
            read(&format!("golden/{name}")),
            Category::Plain,
        ));
    }
    out.push((
        "corpus/hello-broken.pdf".into(),
        read("corpus/hello-broken.pdf"),
        Category::Repaired,
    ));
    for name in [
        "corpus/signed.pdf",
        "corpus/tampered.pdf",
        "signed/approval.pdf",
        "signed/locked.pdf",
        "signed/timestamp.pdf",
    ] {
        out.push((name.into(), read(name), Category::Signed));
    }
    for p in 1..=3 {
        let name = format!("signed/certified-p{p}.pdf");
        out.push((name.clone(), read(&name), Category::Certified));
    }
    // corpus/tampered.pdf with its newest catalog damaged too, so it repairs to the older,
    // unsigned catalog (lamina-cos's policy.rs checks the repair).
    let damaged = damage_newest_catalog(read("corpus/tampered.pdf"));
    out.push((
        "corpus/tampered.pdf+catalog-damaged".into(),
        damaged,
        Category::SignedRepaired,
    ));
    out.push(("corpus/enc.pdf".into(), read("corpus/enc.pdf"), Category::Encrypted));
    out.push((
        "signed/signed-encrypted.pdf".into(),
        read("signed/signed-encrypted.pdf"),
        Category::SignedEncrypted,
    ));
    out
}

/// The outcome a case must have: saved in a mode, or refused with a code.
fn expected(category: Category, mode: SaveMode, invalidate: bool, remove: bool) -> Result<SaveMode, &'static str> {
    use Category as C;
    use SaveMode::{Auto, FullRewrite as Full, Incremental as Inc};
    const REFUSED: &str = "would-invalidate-signatures";
    match (category, mode) {
        (C::Plain, Auto | Inc) => Ok(Inc),
        (C::Plain, _) => Ok(Full),
        (C::Repaired, Inc) => Err("requires-rewrite"),
        (C::Repaired, _) => Ok(Full),
        (C::Signed, Auto | Inc) => Ok(Inc),
        (C::Signed, _) if invalidate => Ok(Full),
        (C::Signed, _) => Err(REFUSED),
        (C::Certified, _) if !invalidate => Err(REFUSED),
        (C::Certified, Auto | Inc) => Ok(Inc),
        (C::Certified, _) => Ok(Full),
        (C::SignedRepaired, _) if !invalidate => Err(REFUSED),
        (C::SignedRepaired, Inc) => Err("requires-rewrite"),
        (C::SignedRepaired, _) => Ok(Full),
        (C::Encrypted | C::SignedEncrypted, _) if !remove => Err("encrypted-base"),
        (C::SignedEncrypted, _) if !invalidate => Err(REFUSED),
        (C::Encrypted | C::SignedEncrypted, Inc) => Err("requires-rewrite"),
        (C::Encrypted | C::SignedEncrypted, _) => Ok(Full),
    }
}

/// The facts lamina compares after reopening: page count, rotations (degrees) and the
/// [`INFO`] keys' values, in that order (the other tools compare the same ones).
fn facts(doc: &lamina_model::Document) -> (usize, Vec<u16>, Vec<Option<String>>) {
    let pages = doc.pages().unwrap();
    let rotations = pages.iter().map(|p| p.rotate().unwrap().degrees()).collect();
    let info = doc.info().unwrap();
    let values = INFO
        .iter()
        .map(|(key, _)| info.as_ref().and_then(|i| i.text(key).unwrap()))
        .collect();
    (pages.len(), rotations, values)
}

fn op(doc: &Document, name: &str, rotation: Rotation) -> Op {
    if name == "setInfo" {
        let [title, author, subject, keywords] = INFO.map(|(_, value)| Some(value));
        let patch = InfoPatch::new()
            .title(title)
            .author(author)
            .subject(subject)
            .keywords(keywords);
        SetInfo::new(patch, None).into()
    } else {
        let page = doc.pages().unwrap().get(0).unwrap().id().clone();
        RotatePage::new(page, rotation).into()
    }
}

/// What a run produced.
#[derive(Debug, Default)]
pub struct Suite {
    /// Every saved output: its file name and bytes.
    pub outputs: Vec<(String, Vec<u8>)>,
    /// One entry per output: what the other tools must see.
    pub manifest: Vec<Value>,
    /// Every case whose outcome or reopened facts were wrong.
    pub failures: Vec<String>,
    /// Cases saved and reopened.
    pub saved: usize,
    /// Cases refused as expected.
    pub refused: usize,
}

/// Runs every case over the fixtures under `root` (the repository root).
pub fn run(root: &Path) -> Suite {
    let mut suite = Suite::default();
    for (index, (name, bytes, category)) in fixtures(root).into_iter().enumerate() {
        let rotation = ROTATIONS[index % ROTATIONS.len()];
        for op_name in ["setInfo", "rotatePage"] {
            for mode in [SaveMode::Auto, SaveMode::Incremental, SaveMode::FullRewrite] {
                for (invalidate, remove) in [(false, false), (true, false), (false, true), (true, true)] {
                    let id = format!(
                        "{}-{op_name}-{mode:?}{}{}",
                        name.replace(['/', '+'], "_").trim_end_matches(".pdf"),
                        if invalidate { "-invalidate" } else { "" },
                        if remove { "-decrypt" } else { "" }
                    )
                    .to_lowercase();
                    let mut doc = Document::open(bytes.clone(), OpenOptions::default()).unwrap();
                    let (pages, mut rotations, mut info) = facts(&doc);
                    let op = op(&doc, op_name, rotation);
                    match op {
                        Op::SetInfo(_) => info = INFO.iter().map(|(_, v)| Some((*v).to_owned())).collect(),
                        _ => rotations[0] = rotation.degrees(),
                    }
                    doc.apply_all(Origin::Local, [op]).unwrap();
                    let options = SaveOptions::new()
                        .with_mode(mode)
                        .with_invalidate_signatures(invalidate)
                        .with_remove_encryption(remove);
                    let mut output = Vec::new();
                    let result = doc.save(&mut output, &options);
                    let got = result.as_ref().map(SaveReport::mode).map_err(SaveError::code);
                    let want = expected(category, mode, invalidate, remove);
                    if got != want {
                        suite
                            .failures
                            .push(format!("{id}: lamina saved {got:?}, expected {want:?}"));
                        continue;
                    }
                    let Ok(report) = result else {
                        suite.refused += 1;
                        continue;
                    };
                    suite.saved += 1;
                    let reopened = lamina_model::Document::open(output.clone(), OpenOptions::default()).unwrap();
                    let again = facts(&reopened);
                    if remove && (reopened.is_encrypted() || reopened.trailer().contains_key("Encrypt")) {
                        suite
                            .failures
                            .push(format!("{id}: removing the encryption left /Encrypt"));
                    }
                    if again != (pages, rotations.clone(), info.clone()) || reopened.was_repaired() {
                        suite.failures.push(format!(
                            "{id}: lamina reopened {again:?} (repaired {}), expected {:?}",
                            reopened.was_repaired(),
                            (pages, &rotations, &info)
                        ));
                    }
                    let file = format!("{id}.pdf");

                    suite.manifest.push(json!({
                        "file": &file,
                        "source": name,
                        "mode": format!("{:?}", report.mode()),
                        "pages": pages,
                        "rotations": rotations,
                        "title": info[0],
                        // The other three keys the setInfo cases write (J.01), compared by the
                        // tools only there: elsewhere they are the fixture's own values.
                        "info": (op_name == "setInfo").then(|| json!({
                            "Author": info[1],
                            "Subject": info[2],
                            "Keywords": info[3],
                        })),
                        "encrypted": reopened.is_encrypted(),
                        // PDFium reads no `/Title` from a direct `/Info` dictionary.
                        "direct_info": matches!(reopened.trailer().get("Info"), Some(lamina_cos::Object::Dict(_))),
                        "invalidated": report.invalidated().len(),
                    }));
                    suite.outputs.push((file, output));
                }
            }
        }
    }
    suite
}
