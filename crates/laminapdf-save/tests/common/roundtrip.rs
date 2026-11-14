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
use laminapdf_model::xmp::{RdfValue, XmpValue};
use laminapdf_model::{
    LabelStyle, OpenAction, PageId, PageLayout, PageMode, PagePosition, PageSize, Rotation, UserSpaceRect,
};
use laminapdf_ops::{
    CropPages, DeletePages, Document, DuplicatePages, InfoPatch, InsertBlankPages, InsertPagesFrom, Margins,
    MergeDocuments, MergeSource, MetadataKey, MetadataPatch, MovePages, Op, OpenAt, OpenZoom, Origin, PageBoxes,
    PageLabelRange, RemoveMetadata, ReplacePages, ReversePages, RotatePage, RotatePages, SetInfo, SetInitialView,
    SetMetadata, SetPageBoxes, SetPageLabels, SetXmpProperty, SourceId, SplitBy, WindowPreferencesPatch, read_metadata,
};
use laminapdf_save::{Restriction, Save, SaveError, SaveMode, SaveOptions, SavePlanner, SaveReport};
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

/// `category`, or [`Category::Certified`] for a signed file whose op reshows a signed
/// signature's widget (rotating its page): that breaks the signature's appearance (ADR 0007
/// §11), so it saves only with the override.
fn with_widget_rule(doc: &Document, op: &Op, category: Category) -> Category {
    let reshows_signed = SavePlanner::new(doc)
        .unwrap()
        .would_violate(op)
        .iter()
        .any(|v| v.restriction == Restriction::SignedWidget);
    if reshows_signed && category == Category::Signed {
        Category::Certified
    } else {
        category
    }
}

/// The facts Lamina PDF compares after reopening: page count, rotations (degrees), the
/// [`INFO`] keys' values and each page's label, in that order (the other tools compare the same
/// ones; the labels only in the `setPageLabels` cases).
type Facts = (usize, Vec<u16>, Vec<Option<String>>, Vec<Option<String>>);

fn facts(doc: &laminapdf_model::Document) -> Facts {
    let pages = doc.pages().unwrap();
    let rotations = pages.iter().map(|p| p.rotate().unwrap().degrees()).collect();
    let info = doc.info().unwrap();
    let values = INFO
        .iter()
        .map(|(key, _)| info.as_ref().and_then(|i| i.text(key).unwrap()))
        .collect();
    let labels = doc.page_labels().unwrap();
    let labels = (0..pages.len()).map(|i| labels.label(i)).collect();
    (pages.len(), rotations, values, labels)
}

/// The labels `doc` should show after `op_name` once saved: the `setPageLabels` cases' own, else
/// the edited document's (the page ops keep each label with its page, and the save must keep
/// them there).
fn labels_after(doc: &laminapdf_model::Document, op_name: &str, pages: usize) -> Vec<Option<String>> {
    if op_name == "setPageLabels" {
        new_labels(pages)
    } else {
        facts(doc).3
    }
}

/// The labels the `setPageLabels` cases give a document of `pages` pages: `i` for the first
/// page, then `p. 1`, `p. 2`, … (I.10).
fn new_labels(pages: usize) -> Vec<Option<String>> {
    (0..pages)
        .map(|i| Some(if i == 0 { "i".to_owned() } else { format!("p. {i}") }))
        .collect()
}

/// The ops each fixture is saved with: a title through the deprecated `setInfo`, a rotation,
/// the metadata ops (J.01–J.03, J.10), the XMP property op (J.04: the title, mirrored in
/// Info, and a custom property, [`CUSTOM`]), page labels (I.10) and the page ops (C.01, C.03,
/// C.04, C.08, C.13), duplicating included (C.03), pages copied in from another document
/// (C.02) or replacing the first page (C.16), from [`SOURCE`], merged in as a whole document
/// (C.06), the initial view (I.06, I.11), and page boxes set (C.09) and cropped (C.11).
const OPS: [&str; 18] = [
    "setInfo",
    "rotatePage",
    "setMetadata",
    "removeMetadata",
    "setXmpProperty",
    "insertBlankPages",
    "deletePages",
    "reversePages",
    "movePages",
    "rotatePages",
    "setPageLabels",
    "duplicatePages",
    "insertPagesFrom",
    "replacePages",
    "mergeDocuments",
    "setInitialView",
    "setPageBoxes",
    "cropPages",
];

/// The source document the import cases copy their page from: one upright page.
const SOURCE: &str = "fixtures/corpus/hello.pdf";

/// What the `setInitialView` cases write: open on the last page at its width, with the
/// thumbnails panel, two pages side by side (PDF 1.5, so older files' `/Version` rises) and the title in
/// the window. The tools check the page mode by its PDF name.
const INITIAL_MODE: PageMode = PageMode::Thumbnails;
const INITIAL_LAYOUT: PageLayout = PageLayout::TwoPageLeft;

/// The custom XMP property the `setXmpProperty` cases add: namespace and name.
const CUSTOM: (&str, &str) = ("http://example.com/roundtrip/", "batch");

/// The save-time date the metadata cases stamp (J.02).
const STAMP: &str = "D:20261110093000Z";

/// The op `name`, or `None` for a deletion, a reversal or a move on a one-page file (a document
/// keeps a page, and one page reversed or moved is no change).
fn op(doc: &Document, name: &str, rotation: Rotation, source: SourceId) -> Option<Op> {
    let [title, author, subject, keywords] = INFO.map(|(_, value)| Some(value));
    let ids: Vec<PageId> = doc.pages().unwrap().iter().map(|p| p.id().clone()).collect();
    Some(match name {
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
        "setXmpProperty" => SetXmpProperty::new(
            "http://purl.org/dc/elements/1.1/",
            "dc",
            "title",
            XmpValue::Text(INFO[0].1.to_owned()),
        )
        .into(),
        "removeMetadata" => RemoveMetadata::new(vec![
            MetadataKey::Title,
            MetadataKey::Authors,
            MetadataKey::Subject,
            MetadataKey::Keywords,
        ])
        .into(),
        "insertBlankPages" => InsertBlankPages::new(PagePosition::End, 1, PageSize::LETTER)
            .with_rotate(rotation)
            .into(),
        "deletePages" if ids.len() > 1 => DeletePages::new(ids.last().cloned().into_iter().collect()).into(),
        "deletePages" => return None,
        "reversePages" if ids.len() > 1 => ReversePages::new(ids).into(),
        "reversePages" => return None,
        "movePages" if ids.len() > 1 => {
            MovePages::new(ids.last().cloned().into_iter().collect(), PagePosition::Start).into()
        }
        "movePages" => return None,
        "rotatePages" => RotatePages::turn(ids, rotation).into(),
        "setPageBoxes" => {
            // A trim box 1 point inside the first page's media box: every fixture's page has room.
            let first = ids.first().cloned()?;
            let media = doc.pages().unwrap().by_id(&first)?.media_box().ok()??;
            let trim = UserSpaceRect::from_corners(media.x0 + 1.0, media.y0 + 1.0, media.x1 - 1.0, media.y1 - 1.0);
            SetPageBoxes::new(vec![first], PageBoxes::default().with_trim(Some(trim))).into()
        }
        "cropPages" => CropPages::new(ids, Margins::new(1.0, 2.0, 3.0, 4.0)).into(),
        "duplicatePages" => DuplicatePages::new(ids.first().cloned().into_iter().collect(), PagePosition::End).into(),
        "mergeDocuments" => {
            MergeDocuments::new(vec![MergeSource::new(source).with_title("Merged")], PagePosition::End).into()
        }
        "insertPagesFrom" | "replacePages" => {
            let from = doc.source_document(&source).unwrap().pages().unwrap();
            let page = vec![from.get(0)?.id().clone()];
            if name == "insertPagesFrom" {
                InsertPagesFrom::new(source, page, PagePosition::End).into()
            } else {
                ReplacePages::new(vec![ids.first().cloned()?], source, page).into()
            }
        }
        "setPageLabels" => {
            let pages = doc.pages().unwrap();
            let mut ranges = vec![PageLabelRange::new(
                pages.get(0).unwrap().id().clone(),
                Some(LabelStyle::LowerRoman),
            )];
            if let Some(second) = pages.get(1) {
                ranges.push(PageLabelRange::new(second.id().clone(), Some(LabelStyle::Decimal)).with_prefix("p. "));
            }
            SetPageLabels::new(ranges).into()
        }
        "setInitialView" => SetInitialView::default()
            .with_open(Some(OpenAt::new(ids.last().cloned()?, OpenZoom::FitWidth)))
            .with_page_mode(Some(INITIAL_MODE))
            .with_page_layout(Some(INITIAL_LAYOUT))
            .with_preferences(WindowPreferencesPatch::default().with_display_doc_title(true))
            .into(),
        _ => RotatePage::new(ids.first().cloned()?, rotation).into(),
    })
}

/// The ops a case applies: `op`, after the edit a removal removes, or with the custom XMP
/// property beside the title. A removal from a file with nothing to remove would change
/// nothing, so it follows the metadata edit, in the same transaction.
fn case_ops(doc: &Document, op_name: &str, op: Op, rotation: Rotation) -> Vec<Op> {
    match op_name {
        "removeMetadata" => vec![self::op(doc, "setMetadata", rotation, SourceId::of(&[])).unwrap(), op],
        "setXmpProperty" => vec![
            op,
            SetXmpProperty::new(CUSTOM.0, "rt", CUSTOM.1, XmpValue::Integer(7)).into(),
        ],
        _ => vec![op],
    }
}

/// What is wrong with the custom XMP property a `setXmpProperty` case added, as reopened.
fn custom_property_problem(id: &str, reopened: &laminapdf_model::Document) -> Option<String> {
    let custom = reopened
        .xmp()
        .unwrap()
        .packet()
        .and_then(|p| p.property(CUSTOM.0, CUSTOM.1).cloned());
    (custom != Some(RdfValue::Text("7".to_owned())))
        .then(|| format!("{id}: the custom XMP property reads back as {custom:?}"))
}

/// The facts after `op` (page count, rotations in degrees, the [`INFO`] keys' values), from
/// those before it.
fn after_op(op: &Op, rotation: Rotation, pages: &mut usize, rotations: &mut Vec<u16>, info: &mut Vec<Option<String>>) {
    match op {
        Op::SetInfo(_) | Op::SetMetadata(_) => {
            *info = INFO.iter().map(|(_, v)| Some((*v).to_owned())).collect();
        }
        Op::SetXmpProperty(_) => info[0] = Some(INFO[0].1.to_owned()),
        Op::RemoveMetadata(_) => *info = vec![None; INFO.len()],
        Op::RotatePage(_) => {
            if let Some(first) = rotations.first_mut() {
                *first = rotation.degrees();
            }
        }
        Op::InsertBlankPages(_) => {
            *pages += 1;
            rotations.push(rotation.degrees());
        }
        Op::DeletePages(_) => {
            *pages -= 1;
            rotations.pop();
        }
        Op::ReversePages(_) => rotations.reverse(),
        Op::MovePages(_) => rotations.rotate_right(1),
        Op::InsertPagesFrom(_) | Op::MergeDocuments(_) => {
            *pages += 1;
            rotations.push(0);
        }
        Op::ReplacePages(_) => {
            if let Some(first) = rotations.first_mut() {
                *first = 0;
            }
        }
        Op::DuplicatePages(_) => {
            *pages += 1;
            if let Some(&first) = rotations.first() {
                rotations.push(first);
            }
        }
        Op::RotatePages(_) => {
            for r in rotations.iter_mut() {
                *r = (*r + rotation.degrees()) % 360;
            }
        }
        _ => {}
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

/// What a reopened output got wrong beyond the facts every case compares: an encryption a
/// decrypting save left behind, Info and XMP out of step after a metadata op, the custom XMP
/// property, or the initial view a `setInitialView` case wrote.
fn reopened_failures(
    id: &str,
    op_name: &str,
    remove: bool,
    metadata_op: bool,
    reopened: &laminapdf_model::Document,
) -> Vec<String> {
    let mut failures = Vec::new();
    if remove && (reopened.is_encrypted() || reopened.trailer().contains_key("Encrypt")) {
        failures.push(format!("{id}: removing the encryption left /Encrypt"));
    }
    if metadata_op {
        let meta = read_metadata(reopened).unwrap();
        if !meta.disagreements.is_empty() {
            failures.push(format!("{id}: Info and XMP disagree on {:?}", meta.disagreements));
        }
    }
    if op_name == "setXmpProperty" {
        failures.extend(custom_property_problem(id, reopened));
    }
    if op_name == "setInitialView" && !opens_as_set(reopened) {
        failures.push(format!(
            "{id}: lamina reopened the initial view as {:?}",
            reopened.initial_view()
        ));
    }
    failures
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
    let source_bytes = std::fs::read(root.join(SOURCE)).unwrap();
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
                    // Adding a source changes nothing to save.
                    let source = doc.add_source(source_bytes.clone(), None).unwrap();
                    let (mut pages, mut rotations, mut info, _) = facts(&doc);
                    let Some(op) = op(&doc, op_name, rotation, source) else {
                        continue;
                    };
                    let category = with_widget_rule(&doc, &op, category);
                    let metadata_op = matches!(op, Op::SetMetadata(_) | Op::RemoveMetadata(_) | Op::SetXmpProperty(_));
                    after_op(&op, rotation, &mut pages, &mut rotations, &mut info);
                    let ops = case_ops(&doc, op_name, op, rotation);
                    doc.apply_all(Origin::Local, ops).unwrap();
                    let labels = labels_after(&doc, op_name, pages);
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
                    suite
                        .failures
                        .extend(reopened_failures(&id, op_name, remove, metadata_op, &reopened));
                    if again != (pages, rotations.clone(), info.clone(), labels.clone()) || reopened.was_repaired() {
                        suite.failures.push(format!(
                            "{id}: lamina reopened {again:?} (repaired {}), expected {:?}",
                            reopened.was_repaired(),
                            (pages, &rotations, &info, &labels)
                        ));
                    }
                    let initial_view = op_name == "setInitialView";
                    let file = format!("{id}.pdf");

                    suite.manifest.push(json!({
                        "file": &file,
                        "source": name,
                        "mode": format!("{:?}", report.mode()),
                        "pages": pages,
                        "rotations": rotations,
                        "title": info[0],
                        // The other three keys the Info and metadata cases write (J.01, J.10),
                        // compared by the tools only there: elsewhere they are the fixture's own
                        // values, which the tools read in their own ways (PDFium reads an empty
                        // `/Keywords` as none).
                        "info": matches!(op_name, "setInfo" | "setMetadata" | "removeMetadata").then(|| json!({
                            "Author": info[1],
                            "Subject": info[2],
                            "Keywords": info[3],
                        })),
                        // Each page's label, compared by the tools only in the setPageLabels cases.
                        "labels": (op_name == "setPageLabels").then(|| labels.clone()),
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
    extra_cases(root, &mut suite);
    suite
}

/// The cases beyond one op on one fixture: page copies, images, new documents and outlines.
fn extra_cases(root: &Path, suite: &mut Suite) {
    copies(root, suite);
    images(root, suite);
    extracted(root, suite);
    outlines(root, suite);
    links(root, suite);
    marks(root, suite);
}

/// Each page's links as the tools list them, sorted: `["page", index]` for a destination in the
/// file, `["uri", address]` for a web address, `["other"]` for anything else.
pub fn link_facts(doc: &laminapdf_model::Document) -> Vec<Vec<Value>> {
    use laminapdf_model::LinkAction;
    let pages = doc.pages().unwrap();
    pages
        .iter()
        .map(|page| {
            let mut found: Vec<Value> = pages
                .links(&page)
                .unwrap()
                .into_iter()
                .map(|link| match link.action {
                    LinkAction::GoTo(dest) => match dest.page_index {
                        Some(i) => json!(["page", i]),
                        None => json!(["other"]),
                    },
                    LinkAction::Uri(uri) => json!(["uri", String::from_utf8_lossy(&uri)]),
                    _ => json!(["other"]),
                })
                .collect();
            found.sort_by_key(ToString::to_string);
            found
        })
        .collect()
}

/// The link cases (todo E3c, I.02, I.03 and A.09): on every plain fixture, a link on an area of
/// the first page to the last page, and a web link on a line of text; the page's first link, when
/// it has one, retargeted to a web address, and its second deleted, saved incrementally and as a
/// full rewrite. Lamina reopens each with the same links and pages; the tools compare the links
/// they read (`links`).
fn links(root: &Path, suite: &mut Suite) {
    use laminapdf_model::{AnnotSubtype, UserSpacePoint, UserSpaceRect};
    use laminapdf_ops::{AddLink, DeleteLink, DestinationView, ExplicitDestination, OutlineTarget, UpdateLink};
    let names = PLAIN_CORPUS
        .iter()
        .map(|n| format!("corpus/{n}"))
        .chain(PLAIN_BUILT.iter().map(|n| format!("golden/{n}")));
    for name in names {
        let bytes = std::fs::read(root.join("fixtures").join(&name)).unwrap();
        for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
            let id = format!("links-{}-{mode:?}", name.replace('/', "_").trim_end_matches(".pdf")).to_lowercase();
            let mut doc = Document::open(bytes.clone(), OpenOptions::default()).unwrap();
            let (pages, rotations, info, labels) = facts(&doc);
            let all = doc.pages().unwrap();
            let first = all.get(0).unwrap();
            let existing: Vec<_> = doc
                .annotations(&first)
                .unwrap()
                .into_iter()
                .filter(|a| a.subtype == AnnotSubtype::Link)
                .map(|a| a.id)
                .collect();
            let last = all.get(pages - 1).unwrap().id().clone();
            let first = first.id().clone();
            drop(all);
            let to_last = OutlineTarget::Destination(ExplicitDestination::new(last, DestinationView::Fit));
            let area = AddLink::new(
                first.clone(),
                UserSpaceRect::from_corners(36.0, 36.0, 120.0, 60.0),
                to_last,
            );
            let mut on_text = AddLink::new(
                first,
                UserSpaceRect::from_corners(36.0, 80.0, 200.0, 94.0),
                OutlineTarget::Uri("https://example.com/round-trip".into()),
            );
            on_text.quads = vec![[
                UserSpacePoint::new(36.0, 94.0),
                UserSpacePoint::new(200.0, 94.0),
                UserSpacePoint::new(36.0, 80.0),
                UserSpacePoint::new(200.0, 80.0),
            ]];
            on_text.contents = Some("Round trip".into());
            let mut ops = vec![Op::from(area), Op::from(on_text)];
            if let Some(link) = existing.first() {
                let mut retarget = UpdateLink::new(link.clone());
                retarget.target = Some(OutlineTarget::Uri("https://example.com/retargeted".into()));
                ops.push(retarget.into());
            }
            if let Some(link) = existing.get(1) {
                ops.push(DeleteLink::new(link.clone()).into());
            }
            doc.apply_all(Origin::Local, ops).unwrap();
            let links = link_facts(&doc);
            let mut output = Vec::new();
            let report = doc.save(&mut output, &SaveOptions::new().with_mode(mode)).unwrap();
            suite.saved += 1;
            let reopened = laminapdf_model::Document::open(output.clone(), OpenOptions::default()).unwrap();
            if link_facts(&reopened) != links
                || facts(&reopened) != (pages, rotations.clone(), info.clone(), labels.clone())
                || reopened.was_repaired()
            {
                suite.failures.push(format!(
                    "{id}: lamina reopened links {:?} (repaired {}), expected {links:?}",
                    link_facts(&reopened),
                    reopened.was_repaired()
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
                "info": null,
                "encrypted": false,
                "direct_info": matches!(reopened.trailer().get("Info"), Some(laminapdf_cos::Object::Dict(_))),
                "invalidated": 0,
                "links": links,
            }));
            suite.outputs.push((file, output));
        }
    }
}

/// The outline as the tools list it: `[depth, title, page]` per item in document order, the
/// depth from 0, the page `null` when the item goes to no page of the file.
pub fn outline_facts(doc: &laminapdf_model::Document) -> Vec<Value> {
    let outline = doc.outline().unwrap();
    outline
        .iter()
        .map(|item| {
            let page = item.target().unwrap().and_then(|d| d.page_index);
            json!([item.depth() - 1, item.title().unwrap(), page])
        })
        .collect()
}

/// The outline cases (todo E3b, I.01 and I.04): on every plain fixture, a bookmark added at the
/// top, styled, with a child that goes to a new named destination, then opened; the first old
/// bookmark renamed, and a named destination a bookmark uses renamed with its uses (its page must
/// still resolve), saved incrementally and as a full rewrite. Lamina reopens each with the
/// same outline and pages; the tools compare the outline they read (`outline`).
fn outlines(root: &Path, suite: &mut Suite) {
    use laminapdf_model::SiblingPosition;
    use laminapdf_ops::{
        AddOutlineItem, DestinationView, ExplicitDestination, OutlineStyle, OutlineTarget, RenameNamedDestination, Rgb,
        SetNamedDestination, UpdateOutlineItem,
    };
    let names = PLAIN_CORPUS
        .iter()
        .map(|n| format!("corpus/{n}"))
        .chain(PLAIN_BUILT.iter().map(|n| format!("golden/{n}")));
    for name in names {
        let bytes = std::fs::read(root.join("fixtures").join(&name)).unwrap();
        for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
            let id = format!("outline-{}-{mode:?}", name.replace('/', "_").trim_end_matches(".pdf")).to_lowercase();
            let mut doc = Document::open(bytes.clone(), OpenOptions::default()).unwrap();
            let (pages, rotations, info, labels) = facts(&doc);
            let first_old = doc.outline().unwrap().roots().next().and_then(|i| i.id().cloned());
            let page = |doc: &Document, i: usize| doc.pages().unwrap().get(i).unwrap().id().clone();
            let fit = ExplicitDestination::new(page(&doc, 0), DestinationView::Fit);
            let last = ExplicitDestination::new(page(&doc, pages - 1), DestinationView::FitH { top: None });
            let mut top = AddOutlineItem::new(
                None,
                SiblingPosition::First,
                "Round trip",
                OutlineTarget::Destination(fit),
            );
            top.style = OutlineStyle::new(Some(Rgb::new(192, 0, 0)), true, false);
            let mut ops = vec![Op::from(top), Op::from(SetNamedDestination::new("round-trip", last))];
            if let Some(old) = first_old {
                let mut rename = UpdateOutlineItem::new(old);
                rename.title = Some("Renamed by the round trip".into());
                ops.push(rename.into());
            }
            let created = doc.apply_all(Origin::Local, ops).unwrap().created_outline_items;
            let child = AddOutlineItem::new(
                created.first().cloned(),
                SiblingPosition::Last,
                "Last page, by name",
                OutlineTarget::Named("round-trip".into()),
            );
            // With a child, the new bookmark can hold its open state; an existing name the file's
            // bookmarks use is renamed with its uses, which must still go to their pages.
            let mut open = UpdateOutlineItem::new(created.first().cloned().unwrap());
            open.open = Some(true);
            let mut more = vec![Op::from(child), Op::from(open)];
            let used = doc
                .outline()
                .unwrap()
                .iter()
                .find_map(|i| i.target().unwrap().and_then(|d| d.name))
                .and_then(|n| String::from_utf8(n).ok())
                .filter(|n| n != "round-trip");
            if let Some(used) = used {
                more.push(RenameNamedDestination::new(used, "renamed-by-the-round-trip").into());
            }
            doc.apply_all(Origin::Local, more).unwrap();
            let outline = outline_facts(&doc);
            let mut output = Vec::new();
            let report = doc.save(&mut output, &SaveOptions::new().with_mode(mode)).unwrap();
            suite.saved += 1;
            let reopened = laminapdf_model::Document::open(output.clone(), OpenOptions::default()).unwrap();
            if outline_facts(&reopened) != outline
                || facts(&reopened) != (pages, rotations.clone(), info.clone(), labels.clone())
                || reopened.was_repaired()
            {
                suite.failures.push(format!(
                    "{id}: lamina reopened outline {:?} (repaired {}), expected {outline:?}",
                    outline_facts(&reopened),
                    reopened.was_repaired()
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
                "info": null,
                "encrypted": false,
                "direct_info": matches!(reopened.trailer().get("Info"), Some(laminapdf_cos::Object::Dict(_))),
                "invalidated": 0,
                "outline": outline,
            }));
            suite.outputs.push((file, output));
        }
    }
}

/// The new-document cases (C.05, H.10): every fixture's last page made into a new document with
/// `extract_pages`, and its pages split in two with `export_ranges`, each saved (a full write)
/// and reopened with the pages, rotations and labels it was given. The tools check them like any
/// other output.
fn extracted(root: &Path, suite: &mut Suite) {
    for (name, bytes, _) in fixtures(root) {
        let Ok(doc) = Document::open(bytes, OpenOptions::default()) else {
            continue;
        };
        let (_, rotations, _, labels) = facts(&doc);
        let ids: Vec<PageId> = doc.pages().unwrap().iter().map(|p| p.id().clone()).collect();
        let half = ids.len().div_ceil(2);
        let mut cases = vec![("extract", ids.len() - 1..ids.len())];
        if ids.len() > 1 {
            cases.push(("export-first", 0..half));
            cases.push(("export-second", half..ids.len()));
        }
        let mut parts: Vec<(String, std::ops::Range<usize>, Document)> = cases
            .into_iter()
            .map(|(case, range)| (case.to_owned(), range.clone(), doc.extract_pages(&ids[range]).unwrap()))
            .collect();
        // C.07: the document split in two by page count, each part checked as an export's is.
        if ids.len() > 1 {
            let count = std::num::NonZeroUsize::new(half).unwrap();
            for (i, part) in doc.split(&SplitBy::Count(count)).unwrap().into_iter().enumerate() {
                let range = i * half..((i + 1) * half).min(ids.len());
                parts.push((format!("split-{}", i + 1), range, part));
            }
        }
        for (case, range, mut part) in parts {
            let id = format!("{}-{case}", name.replace(['/', '+'], "_").trim_end_matches(".pdf")).to_lowercase();
            let mut output = Vec::new();
            let report = match part.save(&mut output, &SaveOptions::new()) {
                Ok(report) => report,
                Err(e) => {
                    suite
                        .failures
                        .push(format!("{id}: lamina refused the save: {}", e.code()));
                    continue;
                }
            };
            suite.saved += 1;
            let reopened = laminapdf_model::Document::open(output.clone(), OpenOptions::default()).unwrap();
            let (pages, got_rotations, _, got_labels) = facts(&reopened);
            let want = (
                range.len(),
                rotations[range.clone()].to_vec(),
                labels[range.clone()].to_vec(),
            );
            if (pages, got_rotations.clone(), got_labels) != want || reopened.was_repaired() || reopened.is_encrypted()
            {
                suite
                    .failures
                    .push(format!("{id}: lamina reopened {pages} pages, expected {want:?}"));
            }
            let file = format!("{id}.pdf");
            suite.manifest.push(json!({
                "file": &file,
                "source": name,
                "mode": format!("{:?}", report.mode()),
                "pages": pages,
                "rotations": got_rotations,
                "title": null,
                "info": null,
                "encrypted": false,
                "direct_info": false,
                "invalidated": 0,
            }));
            suite.outputs.push((file, output));
        }
    }
}

/// The image cases (todo E3h): photos made into PDFs with `images_to_pdf`, a page per photo,
/// sized from the photo and fitted to A4, one of them a photo marked as taken sideways. Lamina
/// reopens each; the tools check it like any other output.
fn images(root: &Path, suite: &mut Suite) {
    use laminapdf_build::{Image, ImagePages, ImagesOptions, PageSize, Pt, images_to_pdf};
    let photo = |name: &str| Image::jpeg(std::fs::read(root.join("fixtures/images").join(name)).unwrap()).unwrap();
    let photos = [
        "photo-rgb.jpg",
        "photo-gray.jpg",
        "photo-cmyk.jpg",
        "photo-wide-exif6.jpg",
    ]
    .map(photo);
    let fitted = ImagesOptions::new()
        .with_pages(ImagePages::Fit(PageSize::A4))
        .with_margin(Pt::from_mm(15.0));
    for (id, options) in [("images-sized", ImagesOptions::new()), ("images-fitted", fitted)] {
        let mut doc = images_to_pdf(&photos, &options).unwrap();
        let mut output = Vec::new();
        let report = doc.save(&mut output, &SaveOptions::new()).unwrap();
        suite.saved += 1;
        let reopened = laminapdf_model::Document::open(output.clone(), OpenOptions::default()).unwrap();
        let (pages, rotations, _, _) = facts(&reopened);
        if pages != photos.len() || reopened.was_repaired() {
            suite.failures.push(format!(
                "{id}: lamina reopened {pages} pages (repaired {}), expected {}",
                reopened.was_repaired(),
                photos.len()
            ));
        }
        let file = format!("{id}.pdf");
        suite.manifest.push(json!({
            "file": &file,
            "source": "images/photo-*.jpg",
            "mode": format!("{:?}", report.mode()),
            "pages": photos.len(),
            "rotations": rotations,
            "title": null,
            "info": null,
            "encrypted": false,
            "direct_info": false,
            "invalidated": 0,
        }));
        suite.outputs.push((file, output));
    }
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
        let (pages, rotations, _, _) = facts(&reopened);
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

/// The subtypes the mark cases make, as the tools name them.
const MARKS: [&[u8]; 6] = [b"Highlight", b"Underline", b"StrikeOut", b"Squiggly", b"Text", b"Caret"];

/// Each page's text markup, notes and carets as the tools list them, sorted: `[subtype,
/// contents]`, contents `""` when there are none. A group member's contents are its primary's
/// (ISO 32000-2 §12.5.6.2: the group takes the primary's properties), as pdf.js reads them.
pub fn mark_facts(doc: &laminapdf_model::Document) -> Vec<Vec<Value>> {
    let pages = doc.pages().unwrap();
    pages
        .iter()
        .map(|page| {
            let annots = doc.annotations(&page).unwrap();
            let contents = |a: &laminapdf_model::Annot| {
                let primary = a
                    .in_reply_to
                    .as_ref()
                    .filter(|_| a.reply_type == laminapdf_model::ReplyType::Group)
                    .and_then(|p| annots.iter().find(|b| b.id == *p));
                primary.unwrap_or(a).contents.clone().unwrap_or_default()
            };
            let mut found: Vec<Value> = annots
                .iter()
                .filter(|a| MARKS.contains(&a.subtype.name()))
                .map(|a| json!([String::from_utf8_lossy(a.subtype.name()), contents(a)]))
                .collect();
            found.sort_by_key(ToString::to_string);
            found
        })
        .collect()
}

/// The mark cases (todo E4c, D.01 to D.03): on every plain fixture, a highlight and an underline
/// over two lines of the first page, a note, and a text replacement (a caret holding the new
/// text, grouped with a strike-out), saved incrementally and as a full rewrite. Lamina reopens
/// each with the same marks and pages, every mark with its appearance; the tools compare the
/// marks they read (`marks`).
fn marks(root: &Path, suite: &mut Suite) {
    use laminapdf_model::{UserSpacePoint, UserSpaceRect};
    use laminapdf_ops::{AddAnnot, AnnotSpec, Caret, MarkupKind, Note, NoteIcon, TextMarkup};
    let names = PLAIN_CORPUS
        .iter()
        .map(|n| format!("corpus/{n}"))
        .chain(PLAIN_BUILT.iter().map(|n| format!("golden/{n}")));
    let line = |y: f64| {
        let p = UserSpacePoint::new;
        [p(36.0, y + 14.0), p(200.0, y + 14.0), p(36.0, y), p(200.0, y)]
    };
    for name in names {
        let bytes = std::fs::read(root.join("fixtures").join(&name)).unwrap();
        for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
            let id = format!("marks-{}-{mode:?}", name.replace('/', "_").trim_end_matches(".pdf")).to_lowercase();
            let mut doc = Document::open(bytes.clone(), OpenOptions::default())
                .unwrap()
                .with_appearances(laminapdf_build::AnnotationAppearances);
            let (pages, rotations, info, labels) = facts(&doc);
            let first = doc.pages().unwrap().get(0).unwrap().id().clone();
            let markup =
                |kind, quads| AddAnnot::new(first.clone(), AnnotSpec::TextMarkup(TextMarkup::new(kind, quads)));
            let mut highlight = markup(MarkupKind::Highlight, vec![line(80.0), line(60.0)]);
            highlight.common.contents = Some("Round trip".into());
            let mut note = AddAnnot::new(
                first.clone(),
                AnnotSpec::Note(Note::new(UserSpacePoint::new(220.0, 94.0), NoteIcon::Comment)),
            );
            note.common.contents = Some("A note".into());
            let mut caret = AddAnnot::new(
                first.clone(),
                AnnotSpec::Caret(Caret::new(UserSpaceRect::from_corners(198.0, 40.0, 204.0, 50.0))),
            );
            caret.common.contents = Some("replacement".into());
            let ops = vec![
                Op::from(highlight),
                markup(MarkupKind::Underline, vec![line(100.0)]).into(),
                note.into(),
                caret.into(),
            ];
            let applied = doc.apply_all(Origin::Local, ops).unwrap();
            let mut strike = markup(MarkupKind::StrikeOut, vec![line(40.0)]);
            strike.common.group_with = applied.created_annotations.last().cloned();
            doc.apply_all(Origin::Local, [Op::from(strike)]).unwrap();
            let marks = mark_facts(&doc);
            let mut output = Vec::new();
            let report = doc.save(&mut output, &SaveOptions::new().with_mode(mode)).unwrap();
            suite.saved += 1;
            let reopened = laminapdf_model::Document::open(output.clone(), OpenOptions::default()).unwrap();
            let drawn = reopened.pages().unwrap().iter().all(|page| {
                reopened
                    .annotations(&page)
                    .unwrap()
                    .iter()
                    .filter(|a| MARKS.contains(&a.subtype.name()) && a.contents.is_some())
                    .all(|a| a.has_appearance)
            });
            if mark_facts(&reopened) != marks
                || !drawn
                || facts(&reopened) != (pages, rotations.clone(), info.clone(), labels.clone())
                || reopened.was_repaired()
            {
                suite.failures.push(format!(
                    "{id}: lamina reopened marks {:?} (drawn {drawn}, repaired {}), expected {marks:?}",
                    mark_facts(&reopened),
                    reopened.was_repaired()
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
                "info": null,
                "encrypted": false,
                "direct_info": matches!(reopened.trailer().get("Info"), Some(laminapdf_cos::Object::Dict(_))),
                "invalidated": 0,
                "marks": marks,
            }));
            suite.outputs.push((file, output));
        }
    }
}
