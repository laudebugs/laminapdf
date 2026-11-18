//! `SetFieldValue` on the signed fixtures (E8 plan §5, ADR 0007 §9): the op is class
//! `FormFill`, which approval signatures, `DocMDP` P2 and P3 and usage rights granting
//! `/Form /FillIn` allow and P1 forbids; a field lock (`FieldMDP`, a signature field's `/Lock`)
//! counts only against the fields it covers, and a field whose name can't be read counts as
//! covered. A misclassified op is a security bug, so each file has its verdict here.
#![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::fixture;
use common::save::editable;
use laminapdf_build::AnnotationAppearances;
use laminapdf_cos::Object;
use laminapdf_model::{FieldId, FieldValue, PdfDate};
use laminapdf_ops::{Document, Op, Origin, SetFieldValue};
use laminapdf_save::{ConstraintKind, MetadataUpdate, Restriction, Save, SaveMode, SaveOptions, SavePlanner};

fn signed(name: &str) -> Document {
    editable(&fixture(&format!("signed/{name}"))).with_appearances(AnnotationAppearances)
}

fn field(doc: &Document, name: &str) -> FieldId {
    doc.form().unwrap().field_named(name).unwrap().id.clone()
}

fn fill(field: FieldId) -> Op {
    SetFieldValue::new(field, FieldValue::Text("Grace Hopper".into())).into()
}

/// The restrictions filling `field` would break on `doc`, in order.
fn verdict(doc: &Document, field: FieldId) -> Vec<Restriction> {
    SavePlanner::new(doc)
        .unwrap()
        .would_violate(&fill(field))
        .into_iter()
        .map(|v| v.restriction)
        .collect()
}

#[test]
fn filling_a_field_gives_each_signed_files_verdict() {
    // A field these files don't have: its name can't be read, so every lock covers it.
    let unknown: FieldId = "f999.0".parse().unwrap();
    let verdicts: Vec<(&str, Vec<Restriction>)> = [
        "approval.pdf",
        "approval-organised.pdf",
        "certified-p1.pdf",
        "certified-p2.pdf",
        "certified-p2-outline.pdf",
        "certified-p3.pdf",
        "locked.pdf",
        "locked-page1.pdf",
        "signed-encrypted.pdf",
        "timestamp.pdf",
        "two-signatures.pdf",
        "usage-rights.pdf",
    ]
    .into_iter()
    .map(|name| (name, verdict(&signed(name), unknown.clone())))
    .collect();
    let p1 = laminapdf_save::DocMdpPermission::NoChanges;
    assert_eq!(
        verdicts,
        [
            ("approval.pdf", vec![]),
            ("approval-organised.pdf", vec![]),
            ("certified-p1.pdf", vec![Restriction::DocMdp(p1)]),
            ("certified-p2.pdf", vec![]),
            ("certified-p2-outline.pdf", vec![]),
            ("certified-p3.pdf", vec![]),
            ("locked.pdf", vec![Restriction::FieldMdp]),
            ("locked-page1.pdf", vec![Restriction::FieldMdp]),
            ("signed-encrypted.pdf", vec![]),
            ("timestamp.pdf", vec![]),
            ("two-signatures.pdf", vec![]),
            ("usage-rights.pdf", vec![]),
        ]
    );
}

#[test]
fn filling_a_real_field_gives_each_form_files_verdict() {
    let verdicts: Vec<(&str, &str, Vec<Restriction>)> = [
        ("locked-page1.pdf", "Name"),
        ("form-locked-include.pdf", "name"),
        ("form-locked-include.pdf", "agree"),
        ("form-locked-include.pdf", "address"),
        ("form-certified-p2.pdf", "name"),
    ]
    .into_iter()
    .map(|(file, name)| {
        let doc = signed(file);
        let id = field(&doc, name);
        (file, name, verdict(&doc, id))
    })
    .collect();
    assert_eq!(
        verdicts,
        [
            ("locked-page1.pdf", "Name", vec![Restriction::FieldMdp]),
            ("form-locked-include.pdf", "name", vec![Restriction::FieldMdp]),
            ("form-locked-include.pdf", "agree", vec![Restriction::FieldMdp]),
            ("form-locked-include.pdf", "address", vec![]),
            ("form-certified-p2.pdf", "name", vec![]),
        ]
    );
}

#[test]
fn a_fill_undone_saves_the_original_bytes_and_redone_the_same_bytes_again() {
    let base = fixture("forms/fields.pdf");
    let save = |doc: &mut Document| {
        let mut out = Vec::new();
        doc.save_copy(&mut out, &SaveOptions::new().with_mode(SaveMode::Incremental))
            .unwrap();
        out
    };
    let mut doc = editable(&base).with_appearances(AnnotationAppearances);
    let id = field(&doc, "name");
    doc.apply_all(Origin::Local, [fill(id)]).unwrap();
    let filled = save(&mut doc);
    doc.undo().unwrap();
    let undone = save(&mut doc);
    doc.redo().unwrap();
    let redone = save(&mut doc);
    assert_eq!((filled == base, undone == base, redone == filled), (false, true, true));
}

#[test]
fn a_locked_field_is_a_violation_naming_the_signature_and_the_field() {
    let doc = signed("form-locked-include.pdf");
    let name = field(&doc, "name");
    let violations = SavePlanner::new(&doc).unwrap().would_violate(&fill(name.clone()));
    let sig1 = doc.field_by_name("Sig1").unwrap().map(FieldId::object);
    let found: Vec<(Restriction, Option<FieldId>, Option<FieldId>)> = violations
        .into_iter()
        .map(|v| (v.restriction, v.signature.field, v.field))
        .collect();
    assert_eq!(found, [(Restriction::FieldMdp, sig1, Some(name))]);
}

#[test]
fn a_field_the_lock_doesnt_cover_is_no_violation() {
    let doc = signed("form-locked-include.pdf");
    let address = field(&doc, "address");
    assert_eq!(verdict(&doc, address), []);
}

#[test]
fn a_saved_fill_of_a_locked_field_conflicts_and_of_another_appends() {
    let kinds = ["name", "address"].map(|name| {
        let mut doc = signed("form-locked-include.pdf");
        let id = field(&doc, name);
        doc.apply_all(Origin::Local, [fill(id)]).unwrap();
        SavePlanner::new(&doc).unwrap().constraint().kind()
    });
    assert_eq!(kinds, [ConstraintKind::Conflict, ConstraintKind::AppendOnly]);
}

#[test]
fn filling_a_certified_p2_form_saves_incrementally_without_stamping_metadata() {
    let base = fixture("signed/form-certified-p2.pdf");
    let mut doc = editable(&base).with_appearances(AnnotationAppearances);
    let name = field(&doc, "name");
    doc.apply_all(Origin::Local, [fill(name)]).unwrap();
    let options = SaveOptions::new().with_metadata_date("D:20261118120000Z".parse::<PdfDate>().unwrap());
    let planner = SavePlanner::new(&doc).unwrap();
    let plan = planner.plan(&options).unwrap();
    assert_eq!(
        (
            planner.constraint().kind(),
            plan.mode(),
            matches!(plan.metadata_update(), MetadataUpdate::Skipped { .. })
        ),
        (ConstraintKind::AppendOnly, SaveMode::Incremental, true)
    );
    let mut out = Vec::new();
    let report = doc.save(&mut out, &options).unwrap();
    let catalog = doc.cos().catalog().unwrap();
    let untouched: Vec<u32> = [
        doc.cos().trailer().get("Info").and_then(Object::as_ref),
        catalog.get("Metadata").and_then(Object::as_ref),
    ]
    .into_iter()
    .flatten()
    .map(|r| r.num)
    .collect();
    let appended = String::from_utf8_lossy(out.get(base.len()..).unwrap()).into_owned();
    assert!(report.metadata_not_updated());
    for num in untouched {
        assert!(
            !appended.contains(&format!("\n{num} 0 obj")),
            "object {num} was written again"
        );
    }
}
