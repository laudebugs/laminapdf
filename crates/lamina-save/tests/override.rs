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
use lamina_cos::OpenOptions;
use lamina_ops::{Document, SaveState};
use lamina_save::{
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
    let doc = lamina_cos::Document::open(bytes.to_vec(), OpenOptions::default()).unwrap();
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
