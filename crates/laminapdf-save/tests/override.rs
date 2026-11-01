//! The explicit override (ADR 0007 §3): the invalidated signatures' values and their `/Perms`
//! entries are removed, the fields stay unsigned, the report names them, and a failed save
//! leaves the document as it was.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::fixture;
use common::save::{editable, set_title, tampered_and_repaired};
use laminapdf_cos::OpenOptions;
use laminapdf_ops::{Document, SaveState};
use laminapdf_save::{
    ConstraintKind, IoWrite, RewriteReason, Save, SaveError, SaveMode, SaveOptions, SavePlanner, SignatureKind,
    SinkError, WriteSink,
};

/// A sink that refuses every write.
struct Failing;

impl WriteSink for Failing {
    fn write_all(&mut self, _: &[u8]) -> Result<(), SinkError> {
        Err(SinkError::Failed("closed".to_owned()))
    }
}

fn open(path: &str) -> Document {
    editable(&fixture(path))
}

fn overridden(mode: SaveMode) -> SaveOptions {
    SaveOptions::new().with_mode(mode).with_invalidate_signatures(true)
}

/// The signature fields of a saved file: (has `/V`), and whether the catalog has `/Perms`.
fn fields_and_perms(bytes: &[u8]) -> (Vec<bool>, bool) {
    let doc = laminapdf_cos::Document::open(bytes.to_vec(), OpenOptions::default()).unwrap();
    let catalog = doc.catalog().unwrap();
    let form = doc.deref(catalog.get("AcroForm").unwrap()).unwrap();
    let fields = doc.deref(form.as_dict().unwrap().get("Fields").unwrap()).unwrap();
    let signed = fields
        .as_array()
        .unwrap()
        .iter()
        .map(|f| doc.deref(f).unwrap().as_dict().unwrap().contains_key("V"))
        .collect();
    (signed, catalog.contains_key("Perms"))
}

#[test]
fn certified_p1_with_set_info_saves_incrementally_and_unsigns_the_field() {
    let bytes = fixture("signed/certified-p1.pdf");
    let mut doc = editable(&bytes);
    set_title(&mut doc, "Changed");
    let mut out = Vec::new();
    let report = doc.save(&mut out, &overridden(SaveMode::Auto)).unwrap();
    let kinds: Vec<_> = report.invalidated().iter().map(|s| s.kind).collect();
    assert_eq!(
        (report.mode(), kinds, out.starts_with(&bytes)),
        (SaveMode::Incremental, vec![SignatureKind::Certification], true)
    );
    assert_eq!(fields_and_perms(&out), (vec![false], false));
    let reopened = Document::open(out, OpenOptions::default()).unwrap();
    assert_eq!(
        SavePlanner::new(&reopened).unwrap().constraint().kind(),
        ConstraintKind::Free
    );
}

#[test]
fn a_full_rewrite_of_an_approval_signed_file_keeps_the_field_unsigned() {
    let mut doc = open("signed/approval.pdf");
    let mut out = Vec::new();
    let report = doc.save(&mut out, &overridden(SaveMode::FullRewrite)).unwrap();
    assert_eq!((report.mode(), report.invalidated().len()), (SaveMode::FullRewrite, 1));
    assert_eq!(fields_and_perms(&out), (vec![false], false));
    // The signature dictionary is gone with the value: nothing reaches it.
    assert!(!out.windows(10).any(|w| w == b"/ByteRange"));
}

#[test]
fn a_full_rewrite_of_a_certified_file_removes_the_perms_entry() {
    let mut doc = open("signed/certified-p2.pdf");
    let mut out = Vec::new();
    doc.save(&mut out, &overridden(SaveMode::FullRewrite)).unwrap();
    assert_eq!(fields_and_perms(&out), (vec![false], false));
}

#[test]
fn a_signed_and_repaired_file_is_rewritten_without_its_signature() {
    for mode in [SaveMode::Auto, SaveMode::FullRewrite] {
        let mut doc = editable(&tampered_and_repaired());
        set_title(&mut doc, "Changed");
        let mut out = Vec::new();
        let report = doc.save(&mut out, &overridden(mode)).unwrap();
        let reopened = Document::open(out.clone(), OpenOptions::default()).unwrap();
        assert_eq!(
            (
                report.mode(),
                report.invalidated().len(),
                reopened.was_repaired(),
                SavePlanner::new(&reopened).unwrap().constraint().signatures.len(),
                out.windows(10).any(|w| w == b"/ByteRange")
            ),
            (SaveMode::FullRewrite, 1, false, 0, false),
            "{mode:?}"
        );
    }
}

#[test]
fn a_signed_and_repaired_file_still_refuses_an_incremental_save() {
    let mut doc = editable(&tampered_and_repaired());
    set_title(&mut doc, "Changed");
    assert_eq!(
        doc.save(&mut Vec::new(), &overridden(SaveMode::Incremental)).err(),
        Some(SaveError::RequiresRewrite {
            reasons: vec![RewriteReason::RepairedBase]
        })
    );
}

#[test]
fn a_failed_save_puts_the_signature_back() {
    let mut doc = open("signed/approval.pdf");
    let before: Vec<_> = doc.overlay().iter().map(|(n, e)| (n, e.clone())).collect();
    let result = doc.save(&mut Failing, &overridden(SaveMode::FullRewrite));
    let after: Vec<_> = doc.overlay().iter().map(|(n, e)| (n, e.clone())).collect();
    assert_eq!(
        (result.is_err(), after == before, doc.save_state()),
        (true, true, SaveState::Clean)
    );
    assert_eq!(SavePlanner::new(&doc).unwrap().constraint().signatures.len(), 1);
}

#[test]
fn a_full_rewrite_to_a_write_only_sink_keeps_the_removal_until_the_rebase() {
    let mut doc = open("signed/approval.pdf");
    let mut out = Vec::new();
    doc.save(&mut IoWrite::new(&mut out), &overridden(SaveMode::FullRewrite))
        .unwrap();
    assert_eq!(doc.save_state(), SaveState::SavedNotRebased);
    doc.rebase(out).unwrap();
    assert_eq!(
        SavePlanner::new(&doc).unwrap().constraint().kind(),
        ConstraintKind::Free
    );
}

#[test]
fn the_removal_is_not_on_the_undo_stack() {
    let mut doc = open("signed/certified-p1.pdf");
    set_title(&mut doc, "Changed");
    doc.save(&mut Vec::new(), &overridden(SaveMode::Auto)).unwrap();
    // The history survived the incremental save; undoing the title leaves the field unsigned.
    doc.undo().unwrap();
    let catalog = doc.catalog().unwrap();
    assert!(!catalog.contains_key("Perms"));
    let title = doc.info().unwrap().and_then(|i| i.text("Title").unwrap());
    assert_eq!(title.as_deref(), Some("Signed fixture"));
}

// --- review fixes ------------------------------------------------------------------------

const SIG: &str = "<< /Type /Sig /ByteRange [0 1 2 3] /Contents <00> >>";

/// A catalog with a direct `/AcroForm` whose `/Fields` holds a direct signed field, and a
/// direct `/Perms` certifying the same signature dictionary (object 4).
fn direct_certified() -> Vec<u8> {
    common::save::classic(
        &[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [<< /FT /Sig /T (C) /V 4 0 R >>] >> /Perms << /DocMDP 4 0 R >> >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R >>",
            SIG,
        ],
        "",
    )
}

#[test]
fn a_direct_field_and_a_direct_perms_entry_are_stripped() {
    let mut doc = editable(&direct_certified());
    set_title(&mut doc, "Changed");
    let mut out = Vec::new();
    let report = doc.save(&mut out, &overridden(SaveMode::Auto)).unwrap();
    assert_eq!((report.mode(), report.invalidated().len()), (SaveMode::Incremental, 1));
    assert_eq!(fields_and_perms(&out), (vec![false], false));
}

#[test]
fn an_overridden_incremental_save_also_invalidates_every_restricting_signature() {
    // Field 5 is an approval signature with a FieldMDP lock; field 6 a P2 certification.
    // `SetInfo` violates only the certification, but removing its value changes a locked
    // document, so the lock's signature breaks too.
    let bytes = common::save::classic(
        &[
            "<< /Type /Catalog /Pages 2 0 R /AcroForm 4 0 R /Perms << /DocMDP 8 0 R >> >>",
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
            "<< /Type /Page /Parent 2 0 R >>",
            "<< /Fields [5 0 R 6 0 R] >>",
            "<< /FT /Sig /T (A) /V 7 0 R /Lock << /Action /All >> >>",
            "<< /FT /Sig /T (C) /V 8 0 R >>",
            SIG,
            "<< /Type /Sig /ByteRange [0 1 2 3] /Contents <00> /Reference [<< /TransformMethod /DocMDP /TransformParams << /P 2 >> >>] >>",
        ],
        "",
    );
    let mut doc = editable(&bytes);
    set_title(&mut doc, "Changed");
    let mut out = Vec::new();
    let report = doc.save(&mut out, &overridden(SaveMode::Auto)).unwrap();
    let mut kinds: Vec<_> = report.invalidated().iter().map(|s| s.kind).collect();
    kinds.sort();
    assert_eq!(
        (report.mode(), kinds),
        (
            SaveMode::Incremental,
            vec![SignatureKind::Approval, SignatureKind::Certification]
        )
    );
    assert_eq!(fields_and_perms(&out), (vec![false, false], false));
}

#[test]
fn after_a_failed_override_the_next_save_still_protects_the_signature() {
    let mut doc = open("signed/approval.pdf");
    assert!(doc.save(&mut Failing, &overridden(SaveMode::FullRewrite)).is_err());
    assert_eq!(
        doc.save(&mut Vec::new(), &SaveOptions::new().with_mode(SaveMode::FullRewrite))
            .map_err(|e| e.code()),
        Err("would-invalidate-signatures")
    );
}

#[test]
fn a_clean_signed_and_repaired_file_saves_unchanged() {
    let bytes = tampered_and_repaired();
    let mut doc = editable(&bytes);
    let mut out = Vec::new();
    let report = doc.save(&mut out, &SaveOptions::new()).unwrap();
    assert_eq!(
        (report.mode(), out == bytes, report.invalidated().len()),
        (SaveMode::Incremental, true, 0)
    );
}
