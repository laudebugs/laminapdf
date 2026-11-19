//! The planner's checks of form-authoring ops (E9 plan §5, ADR 0007 §13): each field an op
//! creates, renames, changes or deletes is held to every lock that covers a name it has or gets
//! (and the fields below it), and to signed signature fields; UR3 judges a creation by `/Form
//! /Add` and a deletion by `/Form /Delete`, and every other authoring op by its class. The
//! authoring ops don't exist yet, so `laminapdf_ops::test_support::Declared` stands in for them
//! around an edit that authors nothing (a blank page added at the end). A misjudged op is a
//! security bug, so every case is checked both before the op applies (`would_violate`) and at
//! save time, and each must give the expected verdict.
#![allow(clippy::unwrap_used, clippy::panic, reason = "tests: a panic is a test failure")]

mod common;

use common::fixture;
use common::save::editable;
use laminapdf_model::{FieldId, PagePosition, PageSize};
use laminapdf_ops::test_support::Declared;
use laminapdf_ops::{Document, FormRight, InsertBlankPages, ModificationClass, Op, Origin};
use laminapdf_save::{
    ConstraintKind, DocMdpPermission, Restriction, Save, SaveOptions, SavePlanner, SignatureKind, Violation,
};

fn signed(name: &str) -> Document {
    editable(&fixture(&format!("signed/{name}")))
}

/// An authoring op, as the planner sees it.
#[derive(Debug, Clone, Copy)]
enum Authoring {
    /// `CreateField`: a field named so.
    Create(&'static str),
    /// `CreateField` of class `Signature`: a signature field named so.
    CreateSignature(&'static str),
    /// A rename of the field named `.0` to `.1`.
    Rename(&'static str, &'static str),
    /// A change to the definition of the field named so, its name kept.
    Change(&'static str),
    /// `DeleteField` of the field named so.
    Delete(&'static str),
    /// A `SetCalculationOrder`- or `SetTabOrder`-like change, naming no field.
    Order,
}

fn field(doc: &Document, name: &str) -> FieldId {
    let form = doc.form().unwrap();
    form.field_named(name)
        .unwrap_or_else(|| {
            panic!(
                "no field {name}: {:?}",
                form.fields.iter().map(|f| &f.name).collect::<Vec<_>>()
            )
        })
        .id
        .clone()
}

fn op(doc: &Document, authoring: Authoring) -> Op {
    let blank = InsertBlankPages::new(PagePosition::End, 1, PageSize::LETTER);
    let document = ModificationClass::Document;
    let names = |names: &[&str]| names.iter().map(ToString::to_string).collect::<Vec<_>>();
    let declared = match authoring {
        Create(name) => Declared::authoring(blank, document, vec![(None, names(&[name]))], Some(FormRight::Add)),
        CreateSignature(name) => Declared::authoring(
            blank,
            ModificationClass::Signature,
            vec![(None, names(&[name]))],
            Some(FormRight::Add),
        ),
        Rename(from, to) => Declared::authoring(
            blank,
            document,
            vec![(Some(field(doc, from)), names(&[from, to]))],
            None,
        ),
        Change(name) => Declared::authoring(blank, document, vec![(Some(field(doc, name)), Vec::new())], None),
        Delete(name) => Declared::authoring(
            blank,
            document,
            vec![(Some(field(doc, name)), names(&[name]))],
            Some(FormRight::Delete),
        ),
        Order => Declared::authoring(blank, document, Vec::new(), None),
    };
    declared.into()
}

fn restrictions(violations: &[Violation]) -> Vec<Restriction> {
    violations.iter().map(|v| v.restriction).collect()
}

/// The restrictions `authoring` breaks on the fixture `name` before it applies, and at save
/// time after it did; the two must agree.
fn verdict(name: &str, authoring: Authoring) -> Vec<Restriction> {
    let doc = signed(name);
    let before = SavePlanner::new(&doc).unwrap().would_violate(&op(&doc, authoring));
    let mut applied = signed(name);
    let declared = op(&applied, authoring);
    applied.apply_all(Origin::Local, [declared]).unwrap();
    let after = SavePlanner::new(&applied).unwrap().constraint().violations.clone();
    assert_eq!(
        before, after,
        "{name} {authoring:?}: would_violate and the save disagree"
    );
    restrictions(&after)
}

/// Checks each `(authoring, expected)` on the fixture `name`, reporting every mismatch at once.
fn check(name: &str, cases: &[(Authoring, Vec<Restriction>)]) {
    let got: Vec<(Authoring, Vec<Restriction>)> = cases.iter().map(|(a, _)| (*a, verdict(name, *a))).collect();
    let wrong: Vec<_> = got
        .iter()
        .zip(cases)
        .filter(|((_, got), (_, expected))| got != expected)
        .map(|((a, got), (_, expected))| format!("{a:?}: got {got:?}, expected {expected:?}"))
        .collect();
    assert!(wrong.is_empty(), "{name}:\n{}", wrong.join("\n"));
}

use Authoring::{Change, Create, CreateSignature, Delete, Order, Rename};
use Restriction::{FieldMdp, SignedWidget, UsageRights};

#[test]
fn approval_signatures_forbid_only_changes_to_the_signed_field() {
    check(
        "approval.pdf",
        &[
            (Create("extra"), vec![]),
            (CreateSignature("Sig2"), vec![]),
            (Create("Sig1.extra"), vec![SignedWidget]),
            (Change("Sig1"), vec![SignedWidget]),
            (Rename("Sig1", "Renamed"), vec![SignedWidget]),
            (Delete("Sig1"), vec![SignedWidget]),
            (Order, vec![]),
        ],
    );
}

#[test]
fn a_second_approval_signature_guards_its_own_field() {
    check(
        "two-signatures.pdf",
        &[(Create("extra"), vec![]), (Change("Sig2"), vec![SignedWidget])],
    );
}

#[test]
fn docmdp_p1_forbids_every_authoring_op() {
    let p1 = Restriction::DocMdp(DocMdpPermission::NoChanges);
    check(
        "certified-p1.pdf",
        &[
            (Create("extra"), vec![p1]),
            (CreateSignature("Sig2"), vec![p1]),
            (Change("Sig1"), vec![p1, SignedWidget]),
            (Order, vec![p1]),
        ],
    );
}

#[test]
fn docmdp_p2_and_p3_allow_only_a_new_signature_field() {
    for (name, permission) in [
        ("certified-p2.pdf", DocMdpPermission::FormFill),
        ("certified-p3.pdf", DocMdpPermission::Annotate),
    ] {
        let p = Restriction::DocMdp(permission);
        check(
            name,
            &[
                (Create("extra"), vec![p]),
                (CreateSignature("Sig2"), vec![]),
                (Change("Sig1"), vec![p, SignedWidget]),
                (Delete("Sig1"), vec![p, SignedWidget]),
                (Order, vec![p]),
            ],
        );
    }
}

#[test]
fn a_form_certified_at_p2_refuses_authoring_its_fields() {
    let p2 = Restriction::DocMdp(DocMdpPermission::FormFill);
    check(
        "form-certified-p2.pdf",
        &[
            (Create("extra"), vec![p2]),
            (Rename("name", "other"), vec![p2]),
            (Delete("address"), vec![p2]),
            (CreateSignature("Sig2"), vec![]),
        ],
    );
}

#[test]
fn a_lock_on_all_fields_covers_a_field_created_after_signing() {
    check(
        "locked.pdf",
        &[
            (Create("extra"), vec![FieldMdp]),
            (CreateSignature("Sig2"), vec![FieldMdp]),
            (Change("Sig1"), vec![SignedWidget, FieldMdp]),
            (Order, vec![]),
        ],
    );
}

#[test]
fn an_include_lock_covers_its_fields_by_every_name_they_have_or_get() {
    check(
        "form-locked-include.pdf",
        &[
            (Create("extra"), vec![]),
            (Create("name.part"), vec![FieldMdp]),
            (Rename("address", "name"), vec![FieldMdp]),
            (Rename("name", "other"), vec![FieldMdp]),
            (Rename("address", "street"), vec![]),
            (Change("agree"), vec![FieldMdp]),
            (Change("address"), vec![]),
            (Delete("name"), vec![FieldMdp]),
            (Delete("address"), vec![]),
            (CreateSignature("Sig2"), vec![]),
            (Order, vec![]),
        ],
    );
}

#[test]
fn an_exclude_lock_covers_every_field_it_does_not_name_new_ones_included() {
    check(
        "form-locked-exclude.pdf",
        &[
            (Create("extra"), vec![FieldMdp]),
            (Create("name.part"), vec![]),
            (Change("name"), vec![]),
            (Change("address"), vec![FieldMdp]),
            (Rename("name", "name2"), vec![FieldMdp]),
            (Rename("address", "name"), vec![FieldMdp]),
            (Delete("name"), vec![]),
            (Delete("address"), vec![FieldMdp]),
            (Order, vec![]),
        ],
    );
}

#[test]
fn usage_rights_without_form_rights_forbid_every_authoring_op() {
    check(
        "usage-rights.pdf",
        &[
            (Create("extra"), vec![UsageRights]),
            (CreateSignature("Sig2"), vec![UsageRights]),
            (Order, vec![UsageRights]),
        ],
    );
}

#[test]
fn form_add_rights_allow_only_creating_fields() {
    check(
        "usage-rights-form-add.pdf",
        &[
            (Create("extra"), vec![]),
            (CreateSignature("Sig2"), vec![]),
            (Rename("Name", "Other"), vec![UsageRights]),
            (Change("Name"), vec![UsageRights]),
            (Delete("Name"), vec![UsageRights]),
            (Order, vec![UsageRights]),
        ],
    );
}

#[test]
fn form_delete_rights_allow_only_deleting_fields() {
    // Signing is granted, but a new signature field still needs `/Form /Add` (Table 260).
    check(
        "usage-rights-form-delete.pdf",
        &[
            (Create("extra"), vec![UsageRights]),
            (CreateSignature("Sig2"), vec![UsageRights]),
            (Rename("Name", "Other"), vec![UsageRights]),
            (Change("Name"), vec![UsageRights]),
            (Delete("Name"), vec![]),
            (Order, vec![UsageRights]),
        ],
    );
}

#[test]
fn the_violation_names_the_authored_field_and_the_lock() {
    let mut doc = signed("form-locked-include.pdf");
    let name = field(&doc, "name");
    let declared = op(&doc, Delete("name"));
    doc.apply_all(Origin::Local, [declared]).unwrap();
    let constraint = SavePlanner::new(&doc).unwrap().constraint().clone();
    let v = &constraint.violations;
    assert_eq!(
        (
            constraint.kind(),
            v.iter()
                .map(|v| (v.class, v.restriction, v.field.clone(), v.signature.kind))
                .collect::<Vec<_>>()
        ),
        (
            ConstraintKind::Conflict,
            vec![(
                ModificationClass::Document,
                FieldMdp,
                Some(name),
                SignatureKind::Approval
            )]
        )
    );
}

#[test]
fn a_rename_into_a_lock_is_refused_without_the_override_and_saved_with_it() {
    let mut doc = signed("form-locked-include.pdf");
    let declared = op(&doc, Rename("address", "name"));
    doc.apply_all(Origin::Local, [declared]).unwrap();
    let refused = doc.save(&mut Vec::new(), &SaveOptions::new()).map_err(|e| e.code());
    let report = doc
        .save(&mut Vec::new(), &SaveOptions::new().with_invalidate_signatures(true))
        .unwrap();
    assert_eq!(
        (refused, report.invalidated().len()),
        (Err("would-invalidate-signatures"), 1)
    );
}

#[test]
fn a_field_created_outside_every_lock_saves_incrementally() {
    let mut doc = signed("form-locked-include.pdf");
    let declared = op(&doc, Create("extra"));
    doc.apply_all(Origin::Local, [declared]).unwrap();
    let report = doc.save(&mut Vec::new(), &SaveOptions::new()).unwrap();
    assert_eq!(report.invalidated().len(), 0);
}

#[test]
fn a_change_to_a_field_whose_name_cant_be_read_is_held_to_every_lock_and_signed_field() {
    let doc = signed("form-locked-include.pdf");
    let unknown: FieldId = "f999.0".parse().unwrap();
    let declared: Op = Declared::authoring(
        InsertBlankPages::new(PagePosition::End, 1, PageSize::LETTER),
        ModificationClass::Document,
        vec![(Some(unknown), Vec::new())],
        None,
    )
    .into();
    let violations = SavePlanner::new(&doc).unwrap().would_violate(&declared);
    assert_eq!(restrictions(&violations), [SignedWidget, FieldMdp]);
}

#[test]
fn undoing_a_field_creation_needs_the_delete_right() {
    // The undo deletes the field it created; a second op keeps the overlay changed, so the
    // pending changes keep both rights.
    let mut doc = signed("usage-rights-form-add.pdf");
    let create = op(&doc, Create("extra"));
    let create_again = op(&doc, Create("other"));
    doc.apply_all(Origin::Local, [create]).unwrap();
    doc.apply_all(Origin::Local, [create_again]).unwrap();
    doc.undo().unwrap();
    let constraint = SavePlanner::new(&doc).unwrap().constraint().clone();
    assert_eq!(restrictions(&constraint.violations), [UsageRights]);
}
