//! The page ops on the signed fixtures (E2 plan §6, ADR 0007 §11): every op's class is
//! `PageStructure`, which no `DocMDP` level permits; on an approval-signed file, deleting or
//! rotating the page that holds the signed widget is a `SignedWidget` conflict, and the same
//! ops on another page save as an incremental update that keeps the signed bytes; on the locked
//! file, deleting or rotating the locked field's page breaks the lock, and moving, reversing,
//! inserting and duplicating don't (a copy's widgets belong to new fields). A misclassified op is a security bug, so each op has its verdict here.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::fixture;
use common::save::{editable, page_ids};
use laminapdf_model::{PageId, PagePosition, PageSize, Rotation};
use laminapdf_ops::{
    DeletePages, Document, DuplicatePages, InsertBlankPages, ModificationClass, MovePages, Op, Origin, ReversePages,
    RotatePages,
};
use laminapdf_save::{ConstraintKind, Restriction, Save, SaveMode, SaveOptions, SavePlanner};

fn signed(name: &str) -> Document {
    editable(&fixture(&format!("signed/{name}")))
}

/// `doc` with a blank page added at the end, so there is a page without the signed widget.
fn with_second_page(mut doc: Document) -> (Document, PageId) {
    doc.apply_all(
        Origin::Local,
        [InsertBlankPages::new(PagePosition::End, 1, PageSize::LETTER).into()],
    )
    .unwrap();
    let second = page_ids(&doc)[1].clone();
    (doc, second)
}

/// One of each page op, on `page`.
fn ops(page: &PageId) -> Vec<(&'static str, Op)> {
    vec![
        (
            "insert",
            InsertBlankPages::new(PagePosition::After(page.clone()), 1, PageSize::LETTER).into(),
        ),
        ("delete", DeletePages::new(vec![page.clone()]).into()),
        ("move", MovePages::new(vec![page.clone()], PagePosition::End).into()),
        ("rotate", RotatePages::set(vec![page.clone()], Rotation::Half).into()),
        ("reverse", ReversePages::new(vec![page.clone()]).into()),
        (
            "duplicate",
            DuplicatePages::new(vec![page.clone()], PagePosition::End).into(),
        ),
    ]
}

fn restrictions(doc: &Document, op: &Op) -> Vec<Restriction> {
    SavePlanner::new(doc)
        .unwrap()
        .would_violate(op)
        .iter()
        .map(|v| v.restriction)
        .collect()
}

#[test]
fn every_page_op_is_page_structure_and_needs_no_rewrite() {
    let page: PageId = "p3.0".parse().unwrap();
    for (name, op) in ops(&page) {
        assert_eq!(
            (op.class(), op.requires_rewrite()),
            (ModificationClass::PageStructure, false),
            "{name}"
        );
    }
}

#[test]
fn every_page_op_breaks_every_docmdp_level() {
    for name in ["certified-p1.pdf", "certified-p2.pdf", "certified-p3.pdf"] {
        let (doc, second) = with_second_page(signed(name));
        for (op_name, op) in ops(&second) {
            let found = restrictions(&doc, &op);
            assert!(
                found.iter().any(|r| matches!(r, Restriction::DocMdp(_))),
                "{name} {op_name}: {found:?}"
            );
        }
    }
}

#[test]
fn on_an_unsigned_file_no_page_op_restricts_anything() {
    let doc = editable(&fixture("pages/inherited.pdf"));
    let page = page_ids(&doc)[0].clone();
    for (name, op) in ops(&page) {
        assert_eq!(restrictions(&doc, &op), [], "{name}");
    }
}

#[test]
fn deleting_or_rotating_the_signed_widgets_page_is_a_conflict() {
    let (doc, _) = with_second_page(signed("approval.pdf"));
    let first = page_ids(&doc)[0].clone();
    for (name, op) in ops(&first) {
        let expected: &[Restriction] = match name {
            "delete" | "rotate" => &[Restriction::SignedWidget],
            _ => &[],
        };
        assert_eq!(restrictions(&doc, &op), expected, "{name}");
    }
}

#[test]
fn a_conflicting_page_op_is_refused_without_the_override() {
    let (mut doc, _) = with_second_page(signed("approval.pdf"));
    let first = page_ids(&doc)[0].clone();
    doc.apply_all(Origin::Local, [DeletePages::new(vec![first]).into()])
        .unwrap();
    let kind = SavePlanner::new(&doc).unwrap().constraint().kind();
    let refused = doc.save(&mut Vec::new(), &SaveOptions::new()).map_err(|e| e.code());
    assert_eq!(
        (kind, refused.err()),
        (ConstraintKind::Conflict, Some("would-invalidate-signatures"))
    );
}

#[test]
fn page_ops_on_another_page_save_incrementally_and_keep_the_signed_bytes() {
    let bytes = fixture("signed/approval.pdf");
    for op_name in ["delete", "rotate", "move", "reverse", "insert"] {
        let (mut doc, second) = with_second_page(editable(&bytes));
        let op = ops(&second).into_iter().find(|(n, _)| *n == op_name).unwrap().1;
        doc.apply_all(Origin::Local, [op]).unwrap();
        let kind = SavePlanner::new(&doc).unwrap().constraint().kind();
        let mut out = Vec::new();
        let report = doc.save(&mut out, &SaveOptions::new()).unwrap();
        assert_eq!(
            (kind, report.mode(), out.starts_with(&bytes), report.invalidated().len()),
            (ConstraintKind::AppendOnly, SaveMode::Incremental, true, 0),
            "{op_name}"
        );
    }
}

#[test]
fn deleting_or_rotating_the_locked_fields_page_breaks_the_lock() {
    let (doc, _) = with_second_page(signed("locked.pdf"));
    let first = page_ids(&doc)[0].clone();
    for (name, op) in ops(&first) {
        let found = restrictions(&doc, &op);
        let breaks_lock = found.contains(&Restriction::FieldMdp);
        assert_eq!(breaks_lock, matches!(name, "delete" | "rotate"), "{name}: {found:?}");
    }
}
