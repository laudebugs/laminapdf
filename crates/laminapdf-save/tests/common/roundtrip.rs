//! The Lamina PDF half of the round-trip suite (ADR 0018 tier 7; todo B3c), shared by the
//! `roundtrip` example (which writes the outputs for `cargo xtask roundtrip`) and
//! `tests/roundtrip_suite.rs` (the fast lane's lamina-only run): for each fixture × proving op
//! × mode × overrides, saves with Lamina PDF, checks the planner's decision against the expected
//! one, reopens every output in Lamina PDF and compares its facts, and lists what pdf.js, `PDFium`
//! and qpdf must see in each.
//!
//! Expectations come from the fixture's category, not from the planner: plain corpus files
//! save in every mode; a repaired base only as a full rewrite; a signed base incrementally
//! (and refuses a requested full rewrite) unless the op breaks its certification; a signed and
//! repaired base only with the override, as a full rewrite; an encrypted base only with
//! `remove_encryption`, as a plaintext full rewrite.
#![allow(
    deprecated,
    reason = "SetInfo is these tests' simplest edit; it stays supported, so they keep exercising it"
)]
#![allow(
    unreachable_pub,
    reason = "shared by the tests and the `roundtrip` example, as `tests/common` helpers are"
)]
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test support: a panic is a test failure"
)]
#![allow(clippy::match_same_arms, reason = "one arm per category and mode reads as a table")]

use std::path::Path;

use laminapdf_cos::OpenOptions;
use laminapdf_model::{OpenAction, PageLayout, PageMode, Rotation};
use laminapdf_ops::{
    Document, InfoPatch, MetadataKey, MetadataPatch, Op, OpenAt, OpenZoom, Origin, RemoveMetadata, RotatePage, SetInfo,
    SetInitialView, SetMetadata, WindowPreferencesPatch, read_metadata,
};
use laminapdf_save::{Save, SaveError, SaveMode, SaveOptions, SaveReport};
use serde_json::{Value, json};

use super::copy;

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

/// B5b's goldens (text, links, outlines, labels, metadata).
pub const B5B_BUILT: [&str; 4] = [
    "build-text-latin.pdf",
    "build-text-multiscript.pdf",
    "links-and-outlines.pdf",
    "metadata.pdf",
];

/// The goldens `laminapdf-build` writes (B5a, then [`B5B_BUILT`]): plain files too, so the full
/// lane's qpdf, pdf.js and PDFium checks cover the builder's output.
pub const PLAIN_BUILT: [&str; 6] = [
    "vectors.pdf",
    "images.pdf",
    B5B_BUILT[0],
    B5B_BUILT[1],
    B5B_BUILT[2],
    B5B_BUILT[3],
];

/// The byte that damages `corpus/tampered.pdf`'s newest catalog so the file repairs to the
/// older, unsigned one: (offset, value), the mutation `laminapdf-cos`'s `policy.rs`
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
    // unsigned catalog (laminapdf-cos's policy.rs checks the repair).
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

/// The facts Lamina PDF compares after reopening: page count, rotations (degrees) and the
/// [`INFO`] keys' values, in that order (the other tools compare the same ones).
fn facts(doc: &laminapdf_model::Document) -> (usize, Vec<u16>, Vec<Option<String>>) {
    let pages = doc.pages().unwrap();
    let rotations = pages.iter().map(|p| p.rotate().unwrap().degrees()).collect();
    let info = doc.info().unwrap();
    let values = INFO
        .iter()
        .map(|(key, _)| info.as_ref().and_then(|i| i.text(key).unwrap()))
        .collect();
    (pages.len(), rotations, values)
}

/// The ops each fixture is saved with: a title through the deprecated `setInfo`, a rotation,
/// the metadata ops (J.01–J.03, J.10) and the initial view (I.06, I.11).
const OPS: [&str; 5] = [
    "setInfo",
    "rotatePage",
    "setMetadata",
    "removeMetadata",
    "setInitialView",
];

/// What the `setInitialView` cases write: open on the last page at its width, with the
/// thumbnails panel, two columns (PDF 1.5, so older files' `/Version` rises) and the title in
/// the window. The tools check the page mode by its PDF name.
const INITIAL_MODE: PageMode = PageMode::Thumbnails;
const INITIAL_LAYOUT: PageLayout = PageLayout::TwoColumnLeft;

/// The save-time date the metadata cases stamp (J.02).
const STAMP: &str = "D:20261110093000Z";

fn op(doc: &Document, name: &str, rotation: Rotation) -> Op {
    let [title, author, subject, keywords] = INFO.map(|(_, value)| Some(value));
    match name {
        "setInfo" => SetInfo::patch(
            InfoPatch::new()
                .title(title)
                .author(author)
                .subject(subject)
                .keywords(keywords),
        )
        .into(),
        "setMetadata" => SetMetadata::new(
            MetadataPatch::new()
                .title(title)
                .authors(&[INFO[1].1])
                .subject(subject)
                .keywords(keywords)
                .creator(Some("lamina round trip"))
                .created(Some("D:20260101120000+01'00'".parse().unwrap())),
            None,
        )
        .into(),
        "removeMetadata" => RemoveMetadata::new(vec![
            MetadataKey::Title,
            MetadataKey::Authors,
            MetadataKey::Subject,
            MetadataKey::Keywords,
        ])
        .into(),
        "setInitialView" => {
            let pages = doc.pages().unwrap();
            let last = pages.get(pages.len() - 1).unwrap().id().clone();
            SetInitialView::default()
                .with_open(Some(OpenAt::new(last, OpenZoom::FitWidth)))
                .with_page_mode(Some(INITIAL_MODE))
                .with_page_layout(Some(INITIAL_LAYOUT))
                .with_preferences(WindowPreferencesPatch::default().with_display_doc_title(true))
                .into()
        }
        _ => {
            let page = doc.pages().unwrap().get(0).unwrap().id().clone();
            RotatePage::new(page, rotation).into()
        }
    }
}

/// Whether `doc` opens as the `setInitialView` cases ask: on the last page, with the panel,
/// layout and title preference they set.
fn opens_as_set(doc: &laminapdf_model::Document) -> bool {
    let view = doc.initial_view().unwrap();
    let last = doc.pages().unwrap().len().checked_sub(1);
    matches!(&view.open, OpenAction::Destination(dest) if dest.page_index == last)
        && view.page_mode == Some(INITIAL_MODE)
        && view.page_layout == Some(INITIAL_LAYOUT)
        && view.preferences.display_doc_title
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
        for op_name in OPS {
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
                    let metadata_op = matches!(op, Op::SetMetadata(_) | Op::RemoveMetadata(_));
                    match op {
                        Op::SetInfo(_) | Op::SetMetadata(_) => {
                            info = INFO.iter().map(|(_, v)| Some((*v).to_owned())).collect();
                        }
                        Op::RemoveMetadata(_) => info = vec![None; INFO.len()],
                        Op::SetInitialView(_) => {}
                        _ => rotations[0] = rotation.degrees(),
                    }
                    // A removal from a file with nothing to remove would change nothing: it
                    // follows the metadata edit, in the same transaction.
                    let ops = if op_name == "removeMetadata" {
                        vec![self::op(&doc, "setMetadata", rotation), op]
                    } else {
                        vec![op]
                    };
                    doc.apply_all(Origin::Local, ops).unwrap();
                    let mut options = SaveOptions::new()
                        .with_mode(mode)
                        .with_invalidate_signatures(invalidate)
                        .with_remove_encryption(remove);
                    if metadata_op {
                        options = options.with_metadata_date(STAMP.parse().unwrap());
                    }
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
                    let reopened = laminapdf_model::Document::open(output.clone(), OpenOptions::default()).unwrap();
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
                    let initial_view = op_name == "setInitialView";
                    if initial_view && !opens_as_set(&reopened) {
                        suite.failures.push(format!(
                            "{id}: lamina reopened the initial view as {:?}",
                            reopened.initial_view()
                        ));
                    }
                    if metadata_op {
                        let meta = read_metadata(&reopened).unwrap();
                        if !meta.disagreements.is_empty() {
                            suite
                                .failures
                                .push(format!("{id}: Info and XMP disagree on {:?}", meta.disagreements));
                        }
                    }
                    let file = format!("{id}.pdf");

                    suite.manifest.push(json!({
                        "file": &file,
                        "source": name,
                        "mode": format!("{:?}", report.mode()),
                        "pages": pages,
                        "rotations": rotations,
                        "title": info[0],
                        // The other three keys the metadata cases write (J.01), compared by the
                        // tools only there: elsewhere they are the fixture's own values.
                        "info": matches!(op_name, "setInfo" | "setMetadata" | "removeMetadata").then(|| json!({
                            "Author": info[1],
                            "Subject": info[2],
                            "Keywords": info[3],
                        })),
                        "encrypted": reopened.is_encrypted(),
                        // PDFium reads no `/Title` from a direct `/Info` dictionary.
                        "direct_info": matches!(reopened.trailer().get("Info"), Some(laminapdf_cos::Object::Dict(_))),
                        "invalidated": report.invalidated().len(),
                        // The setInitialView cases' open page (0-based), page mode and layout,
                        // by their PDF names; null elsewhere, where the fixture keeps its own.
                        "initial_view": initial_view.then(|| json!({
                            "open_page": pages - 1,
                            "page_mode": INITIAL_MODE.pdf_name(),
                            "page_layout": INITIAL_LAYOUT.pdf_name(),
                        })),
                    }));
                    suite.outputs.push((file, output));
                }
            }
        }
    }
    copies(root, &mut suite);
    suite
}

/// The files the copy cases take their first page from, with the password each opens with:
/// the plain files above, a repaired one, a signed one and three encrypted ones.
fn copy_sources() -> Vec<(String, Option<&'static str>)> {
    let mut out: Vec<(String, Option<&str>)> = PLAIN_CORPUS
        .iter()
        .map(|name| (format!("corpus/{name}"), None))
        .chain(PLAIN_BUILT.iter().map(|name| (format!("golden/{name}"), None)))
        .collect();
    out.extend([
        ("corpus/hello-broken.pdf".into(), None),
        ("corpus/signed.pdf".into(), None),
        ("corpus/enc.pdf".into(), None),
        ("encrypted/r4-aes-128-user.pdf".into(), Some("user-pw")),
        ("text/encrypted/build-text-multiscript-rc4-128.pdf".into(), Some("user")),
    ]);
    out
}

/// The copy cases (todo E2p): the first page of each [`copy_sources`] file, copied with
/// `ObjectCopier` into an empty document and saved. Lamina reopens each output; the tools check
/// it like any other output, and `PDFium` also draws it next to the page it came from
/// (`copy_of`).
fn copies(root: &Path, suite: &mut Suite) {
    for (name, password) in copy_sources() {
        let path = root.join("fixtures").join(&name);
        let id = format!("copy-{}", name.replace('/', "_").trim_end_matches(".pdf")).to_lowercase();
        let mut options = OpenOptions::default();
        if let Some(password) = password {
            options = options.with_password(password);
        }
        let source = laminapdf_model::Document::open(std::fs::read(&path).unwrap(), options).unwrap();
        let pages = source.pages().unwrap();
        let page = pages.get(0).unwrap();
        let rotation = page.rotate().unwrap().degrees();
        let mut target = laminapdf_cos::Document::open(copy::empty_file(), OpenOptions::default()).unwrap();
        copy::copy_pages_into(&mut target, &source, &[page.cos()]).unwrap();
        let mut doc = Document::from(laminapdf_model::Document::from(target));
        let mut output = Vec::new();
        let report = match doc.save(&mut output, &SaveOptions::new()) {
            Ok(report) => report,
            Err(e) => {
                suite
                    .failures
                    .push(format!("{id}: saving the copy failed: {}", e.code()));
                continue;
            }
        };
        suite.saved += 1;
        let reopened = laminapdf_model::Document::open(output.clone(), OpenOptions::default()).unwrap();
        let (pages, rotations, _) = facts(&reopened);
        if (pages, &rotations[..]) != (1, &[rotation][..]) || reopened.was_repaired() || reopened.is_encrypted() {
            suite.failures.push(format!(
                "{id}: lamina reopened {pages} pages turned {rotations:?} (repaired {}, encrypted {}), expected one turned {rotation}",
                reopened.was_repaired(),
                reopened.is_encrypted(),
            ));
        }
        let file = format!("{id}.pdf");
        suite.manifest.push(json!({
            "file": &file,
            "source": name,
            "mode": format!("{:?}", report.mode()),
            "pages": 1,
            "rotations": [rotation],
            "title": null,
            "info": null,
            "encrypted": false,
            "direct_info": false,
            "invalidated": 0,
            "copy_of": { "path": path, "page": 0, "password": password },
        }));
        suite.outputs.push((file, output));
    }
}
