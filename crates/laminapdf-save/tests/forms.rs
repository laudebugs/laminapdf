//! `SetFieldValue` on the signed fixtures (E8 plan §5, ADR 0007 §9): the op is class
//! `FormFill`, which approval signatures, `DocMDP` P2 and P3 and usage rights granting
//! `/Form /FillIn` allow and P1 forbids; a field lock (`FieldMDP`, a signature field's `/Lock`)
//! counts only against the fields it covers, and a field whose name can't be read counts as
//! covered. A misclassified op is a security bug, so each file has its verdict here. A fill of
//! many fields (E8g) skips the fields a lock covers, so it never conflicts through one.
//! `FlattenForm` (E8p) is class `Content`, and is held to the lock on every field it names.
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

/// A fill of `name` and `address` as one transaction (E8g's `fill_form`): the locked field is
/// skipped and listed, so the save appends rather than conflicts.
#[test]
fn a_fill_skips_a_locked_field_so_its_save_appends_on_a_locked_and_on_a_certified_p2_form() {
    let outcomes = ["form-locked-include.pdf", "form-certified-p2.pdf"].map(|name| {
        // As the façade opens every document: the lock reader is the save crate's.
        let mut doc = signed(name).with_field_locks(laminapdf_save::SignatureLocks);
        let values = ["name", "address"]
            .map(|f| SetFieldValue::new(field(&doc, f), FieldValue::Text("Grace Hopper".into())))
            .to_vec();
        let filled = doc.fill(values, true).unwrap();
        let locked: Vec<String> = filled.locked.iter().map(ToString::to_string).collect();
        (locked.len(), SavePlanner::new(&doc).unwrap().constraint().kind())
    });
    assert_eq!(
        outcomes,
        [(1, ConstraintKind::AppendOnly), (0, ConstraintKind::AppendOnly)]
    );
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

/// The restrictions hiding `field` would break on `doc`, in order.
fn hide_verdict(doc: &Document, field: FieldId) -> Vec<Restriction> {
    SavePlanner::new(doc)
        .unwrap()
        .would_violate(&laminapdf_ops::SetFieldHidden::new(field, true).into())
        .into_iter()
        .map(|v| v.restriction)
        .collect()
}

#[test]
fn hiding_a_field_is_an_annotation_change_allowed_under_p3_and_approvals_and_locked_fields_only() {
    // A field the file doesn't have: the class alone decides.
    let unknown: FieldId = "f999.0".parse().unwrap();
    let p1 = laminapdf_save::DocMdpPermission::NoChanges;
    let p2 = laminapdf_save::DocMdpPermission::FormFill;
    let verdicts = [
        hide_verdict(&signed("certified-p1.pdf"), unknown.clone()),
        hide_verdict(&signed("certified-p3.pdf"), unknown.clone()),
        hide_verdict(&signed("approval.pdf"), unknown.clone()),
        // UR3 grants `/Annots [/Create /Delete]`: no `/Modify`.
        hide_verdict(&signed("usage-rights.pdf"), unknown),
        hide_verdict(
            &signed("form-certified-p2.pdf"),
            field(&signed("form-certified-p2.pdf"), "name"),
        ),
        hide_verdict(
            &signed("form-locked-include.pdf"),
            field(&signed("form-locked-include.pdf"), "address"),
        ),
        hide_verdict(
            &signed("form-locked-include.pdf"),
            field(&signed("form-locked-include.pdf"), "name"),
        ),
    ];
    assert_eq!(
        verdicts,
        [
            vec![Restriction::DocMdp(p1)],
            vec![],
            vec![],
            vec![Restriction::UsageRights],
            vec![Restriction::DocMdp(p2)],
            vec![],
            vec![Restriction::FieldMdp],
        ]
    );
}

#[test]
fn a_hide_the_planner_allows_plans_without_a_conflict_and_one_on_a_locked_field_conflicts() {
    let plan = |name: &str| {
        let mut doc = signed("form-locked-include.pdf");
        let id = field(&doc, name);
        doc.apply_all(Origin::Local, [laminapdf_ops::SetFieldHidden::new(id, true).into()])
            .unwrap();
        SavePlanner::new(&doc)
            .unwrap()
            .constraint()
            .violations
            .iter()
            .map(|v| v.restriction)
            .collect::<Vec<_>>()
    };
    assert_eq!([plan("address"), plan("name")], [vec![], vec![Restriction::FieldMdp]]);
}

#[test]
fn filling_a_locked_total_directly_is_refused_and_its_inputs_are_not() {
    // The planner still refuses a SetFieldValue on a locked field from any source; a
    // calculation skips it instead (E8 plan §2.3).
    let doc = signed("scripts-locked-total.pdf");
    assert_eq!(
        [verdict(&doc, field(&doc, "total")), verdict(&doc, field(&doc, "qty"))],
        [vec![Restriction::FieldMdp], vec![]]
    );
}

/// The restrictions flattening `names` would break on `file` (todo E8p).
fn flatten_verdict(file: &str, names: &[&str]) -> Vec<Restriction> {
    let doc = signed(file);
    let fields = names.iter().map(|n| field(&doc, n)).collect();
    let op: Op = laminapdf_ops::FlattenForm::new(fields, laminapdf_ops::FlattenIntent::View).into();
    SavePlanner::new(&doc)
        .unwrap()
        .would_violate(&op)
        .into_iter()
        .map(|v| v.restriction)
        .collect()
}

#[test]
fn flattening_a_form_is_a_content_change_every_certification_forbids() {
    use laminapdf_save::DocMdpPermission::{Annotate, FormFill, NoChanges};
    // The planner judges an op before it applies, so the field need not exist.
    let unknown: FieldId = "f999.0".parse().unwrap();
    let op: Op = laminapdf_ops::FlattenForm::new(vec![unknown], laminapdf_ops::FlattenIntent::View).into();
    let verdicts: Vec<Vec<Restriction>> = ["certified-p1.pdf", "certified-p2.pdf", "certified-p3.pdf"]
        .map(|name| {
            let doc = signed(name);
            let planner = SavePlanner::new(&doc).unwrap();
            planner.would_violate(&op).into_iter().map(|v| v.restriction).collect()
        })
        .to_vec();
    assert_eq!(
        (verdicts, flatten_verdict("form-certified-p2.pdf", &["name"])),
        (
            vec![
                vec![Restriction::DocMdp(NoChanges)],
                vec![Restriction::DocMdp(FormFill)],
                vec![Restriction::DocMdp(Annotate)]
            ],
            vec![Restriction::DocMdp(FormFill)]
        )
    );
}

#[test]
fn flattening_a_form_is_held_to_the_lock_on_each_field_it_names() {
    let doc = signed("form-locked-include.pdf");
    let [address, name, agree] = ["address", "name", "agree"].map(|n| field(&doc, n));
    let flatten = |fields: Vec<FieldId>| -> Vec<(Restriction, Option<FieldId>)> {
        let op: Op = laminapdf_ops::FlattenForm::new(fields, laminapdf_ops::FlattenIntent::View).into();
        SavePlanner::new(&doc)
            .unwrap()
            .would_violate(&op)
            .into_iter()
            .map(|v| (v.restriction, v.field))
            .collect()
    };
    assert_eq!(
        (
            flatten(vec![address.clone()]),
            flatten(vec![address, name.clone(), agree.clone()])
        ),
        (
            vec![],
            vec![
                (Restriction::FieldMdp, Some(name)),
                (Restriction::FieldMdp, Some(agree))
            ]
        )
    );
}

#[test]
fn a_flattened_form_undone_saves_the_original_bytes_and_redone_the_same_bytes_again() {
    let base = fixture("forms/fields.pdf");
    let save = |doc: &mut Document| {
        let mut out = Vec::new();
        doc.save_copy(&mut out, &SaveOptions::new().with_mode(SaveMode::Incremental))
            .unwrap();
        out
    };
    let mut doc = editable(&base).with_appearances(AnnotationAppearances);
    let fields = ["name", "agree", "colour", "person.first", "person.last"]
        .iter()
        .map(|n| field(&doc, n))
        .collect();
    let op: Op = laminapdf_ops::FlattenForm::new(fields, laminapdf_ops::FlattenIntent::View).into();
    doc.apply_all(Origin::Local, [op]).unwrap();
    let flattened = save(&mut doc);
    doc.undo().unwrap();
    let undone = save(&mut doc);
    doc.redo().unwrap();
    let redone = save(&mut doc);
    assert_eq!(
        (flattened == base, undone == base, redone == flattened),
        (false, true, true)
    );
}

#[test]
fn a_saved_flattening_of_a_locked_field_conflicts_and_of_another_appends() {
    let kinds = ["name", "address"].map(|name| {
        let mut doc = signed("form-locked-include.pdf");
        let id = field(&doc, name);
        let op: Op = laminapdf_ops::FlattenForm::new(vec![id], laminapdf_ops::FlattenIntent::View).into();
        doc.apply_all(Origin::Local, [op]).unwrap();
        SavePlanner::new(&doc).unwrap().constraint().kind()
    });
    assert_eq!(kinds, [ConstraintKind::Conflict, ConstraintKind::AppendOnly]);
}

#[test]
fn removing_xfa_is_a_document_change_every_certification_and_usage_right_forbids() {
    use laminapdf_save::DocMdpPermission::{Annotate, FormFill, NoChanges};
    let op: Op = laminapdf_ops::RemoveXfa::new().into();
    let verdict = |name: &str| -> Vec<Restriction> {
        let doc = signed(name);
        let planner = SavePlanner::new(&doc).unwrap();
        planner.would_violate(&op).into_iter().map(|v| v.restriction).collect()
    };
    assert_eq!(
        [
            "certified-p1.pdf",
            "certified-p2.pdf",
            "certified-p3.pdf",
            "usage-rights.pdf",
            "approval.pdf"
        ]
        .map(verdict),
        [
            vec![Restriction::DocMdp(NoChanges)],
            vec![Restriction::DocMdp(FormFill)],
            vec![Restriction::DocMdp(Annotate)],
            vec![Restriction::UsageRights],
            vec![]
        ]
    );
}
