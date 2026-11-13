//! The planner's field-level checks of page ops (E2 plan §6, ADR 0007 §11):
//! a page op that deletes or reshows the widget of a signed signature field, or of a field a
//! `FieldMDP` lock covers, breaks the signature's restriction; moving the page breaks nothing.
//! The ops are stood in for by `laminapdf_ops::test_support::Declared`, which declares the
//! widgets it touches around an edit that touches none (a blank page added at the end).
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]
#![cfg(feature = "page-checks")]

mod common;

use common::fixture;
use common::save::{editable, page_ids};
use laminapdf_model::{FieldId, PageId, PagePosition, PageSize};
use laminapdf_ops::test_support::Declared;
use laminapdf_ops::{Document, InsertBlankPages, ModificationClass, Op, Origin, PageChange, WidgetChange};
use laminapdf_save::{ConstraintKind, Restriction, Save, SaveOptions, SavePlanner, Violation};

fn signed(name: &str) -> Document {
    editable(&fixture(&format!("signed/{name}")))
}

/// The signed field (`Sig1`) and the page its widget is on.
fn sig1(doc: &Document) -> (FieldId, PageId) {
    let planner = SavePlanner::new(doc).unwrap();
    let field = planner.constraint().signatures[0].field.clone().unwrap();
    (field, page_ids(doc).remove(0))
}

/// A blank page added at the end (an edit that touches no widget itself) that declares
/// `field`'s widget on `page` changed by `change`.
fn declared(page: &PageId, field: &FieldId, change: WidgetChange) -> Op {
    Declared::new(
        InsertBlankPages::new(PagePosition::End, 1, PageSize::LETTER),
        vec![(field.clone(), change)],
        vec![PageChange::new(page.clone(), change)],
    )
    .into()
}

fn apply(doc: &mut Document, change: WidgetChange) -> FieldId {
    let (field, page) = sig1(doc);
    doc.apply_all(Origin::Local, [declared(&page, &field, change)]).unwrap();
    field
}

fn restrictions(violations: &[Violation]) -> Vec<(Restriction, Option<FieldId>)> {
    violations.iter().map(|v| (v.restriction, v.field.clone())).collect()
}

#[test]
fn deleting_a_signed_widget_is_a_conflict_on_an_approval_signed_file() {
    let mut doc = signed("approval.pdf");
    let field = apply(&mut doc, WidgetChange::Deleted);
    let planner = SavePlanner::new(&doc).unwrap();
    assert_eq!(
        (
            planner.constraint().kind(),
            restrictions(&planner.constraint().violations)
        ),
        (ConstraintKind::Conflict, vec![(Restriction::SignedWidget, Some(field))])
    );
}

#[test]
fn a_deleted_signed_widget_is_refused_without_the_override_and_saved_with_it() {
    let mut doc = signed("approval.pdf");
    apply(&mut doc, WidgetChange::Deleted);
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
fn reshowing_a_locked_fields_widget_breaks_the_lock() {
    let mut doc = signed("locked.pdf");
    let field = apply(&mut doc, WidgetChange::Reshown);
    let planner = SavePlanner::new(&doc).unwrap();
    assert!(
        restrictions(&planner.constraint().violations).contains(&(Restriction::FieldMdp, Some(field))),
        "{:?}",
        planner.constraint().violations
    );
}

#[test]
fn moving_a_locked_fields_page_breaks_nothing() {
    let mut doc = signed("locked.pdf");
    apply(&mut doc, WidgetChange::Moved);
    let planner = SavePlanner::new(&doc).unwrap();
    assert_eq!(
        (planner.constraint().kind(), planner.constraint().violations.len()),
        (ConstraintKind::AppendOnly, 0)
    );
}

#[test]
fn moving_a_signed_widgets_page_saves_incrementally() {
    let mut doc = signed("approval.pdf");
    apply(&mut doc, WidgetChange::Moved);
    let report = doc.save(&mut Vec::new(), &SaveOptions::new()).unwrap();
    assert_eq!(report.invalidated().len(), 0);
}

#[test]
fn would_violate_gives_the_save_time_verdict_before_the_op_applies() {
    for (name, change, expected) in [
        ("approval.pdf", WidgetChange::Deleted, vec![Restriction::SignedWidget]),
        ("approval.pdf", WidgetChange::Reshown, vec![Restriction::SignedWidget]),
        ("approval.pdf", WidgetChange::Moved, vec![]),
        (
            "locked.pdf",
            WidgetChange::Reshown,
            vec![Restriction::SignedWidget, Restriction::FieldMdp],
        ),
        ("locked.pdf", WidgetChange::Moved, vec![]),
    ] {
        let doc = signed(name);
        let (field, page) = sig1(&doc);
        let before: Vec<Restriction> = SavePlanner::new(&doc)
            .unwrap()
            .would_violate(&declared(&page, &field, change))
            .iter()
            .map(|v| v.restriction)
            .collect();
        let mut applied = signed(name);
        apply(&mut applied, change);
        let after: Vec<Restriction> = SavePlanner::new(&applied)
            .unwrap()
            .constraint()
            .violations
            .iter()
            .map(|v| v.restriction)
            .collect();
        assert_eq!((&before, &after), (&expected, &expected), "{name} {change:?}");
    }
}

#[test]
fn the_violation_carries_the_class_of_the_op() {
    let mut doc = signed("approval.pdf");
    apply(&mut doc, WidgetChange::Deleted);
    let planner = SavePlanner::new(&doc).unwrap();
    assert_eq!(
        planner.constraint().violations[0].class,
        ModificationClass::PageStructure
    );
}

#[test]
fn a_widget_change_on_an_unsigned_file_restricts_nothing() {
    let mut doc = editable(&fixture("corpus/hello.pdf"));
    let page = page_ids(&doc).remove(0);
    let field = FieldId::object(laminapdf_cos::ObjRef::new(99, 0));
    doc.apply_all(Origin::Local, [declared(&page, &field, WidgetChange::Deleted)])
        .unwrap();
    assert_eq!(
        SavePlanner::new(&doc).unwrap().constraint().kind(),
        ConstraintKind::Free
    );
}

#[test]
fn the_applied_report_names_the_touched_widgets() {
    let mut doc = signed("approval.pdf");
    let (field, page) = sig1(&doc);
    let applied = doc
        .apply_all(Origin::Local, [declared(&page, &field, WidgetChange::Deleted)])
        .unwrap();
    assert_eq!(
        applied.widgets.iter().map(|w| (&w.field, w.change)).collect::<Vec<_>>(),
        [(&field, WidgetChange::Deleted)]
    );
}

#[test]
fn undoing_every_change_forgets_the_widget_changes() {
    // Pending changes err on the side of reporting, but an empty overlay clears them.
    let mut doc = signed("approval.pdf");
    apply(&mut doc, WidgetChange::Deleted);
    doc.undo().unwrap();
    let constraint = SavePlanner::new(&doc).unwrap().constraint().clone();
    assert_eq!((constraint.unchanged, constraint.violations.len()), (true, 0));
}

#[test]
fn a_lock_given_twice_is_reported_once() {
    // pyHanko writes the lock both as the field's `/Lock` and as a `FieldMDP` transform.
    let doc = signed("locked.pdf");
    let violations = SavePlanner::new(&doc)
        .unwrap()
        .would_violate_class(ModificationClass::FormFill);
    assert_eq!(restrictions(&violations), [(Restriction::FieldMdp, None)]);
}
