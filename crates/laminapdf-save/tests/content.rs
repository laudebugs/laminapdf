//! Saving `EditContent` (todo E7d) and `EditText` (todo E7e, ADR 0007): their verdict on the signed fixtures (class
//! `Content`: approval signatures and a `FieldMDP` lock allow it, as it changes no field; every
//! certification forbids it, since none of P1–P3 allows changing what a page draws, ISO 32000-2
//! §12.8.2.2 Table 257).
#![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::fixture;
use common::save::editable;
use laminapdf_ops::{ContentEdit, EditContent, EditText, ObjectRef, Op, TextRunRef};
use laminapdf_save::{ConstraintKind, DocMdpPermission, Restriction, SavePlanner};

/// Each signed fixture's verdict on deleting an object of its first page, or on rewriting a run
/// of its text (the planner judges an op before it applies, so the object need not exist).
fn verdict(name: &str, text: bool) -> (ConstraintKind, Vec<Restriction>) {
    let doc = editable(&fixture(&format!("signed/{name}")));
    let page = doc.pages().unwrap().get(0).unwrap().id().clone();
    let object = ObjectRef::new(0, 0);
    let op: Op = if text {
        EditText::new(page, TextRunRef::new(object, 0, 1), "x".to_owned()).into()
    } else {
        EditContent::new(page, vec![ContentEdit::Delete { object }]).into()
    };
    let planner = SavePlanner::new(&doc).unwrap();
    let restrictions = planner.would_violate(&op).iter().map(|v| v.restriction).collect();
    (planner.constraint().kind(), restrictions)
}

#[test]
fn approval_signatures_and_a_lock_allow_content_edits() {
    for name in ["approval.pdf", "two-signatures.pdf", "locked.pdf"] {
        for text in [false, true] {
            assert_eq!(
                verdict(name, text),
                (ConstraintKind::AppendOnly, Vec::new()),
                "{name} {text}"
            );
        }
    }
}

#[test]
fn every_certification_forbids_content_edits() {
    for (name, permission) in [
        ("certified-p1.pdf", DocMdpPermission::NoChanges),
        ("certified-p2.pdf", DocMdpPermission::FormFill),
        ("certified-p3.pdf", DocMdpPermission::Annotate),
    ] {
        for text in [false, true] {
            assert_eq!(
                verdict(name, text),
                (ConstraintKind::AppendOnly, vec![Restriction::DocMdp(permission)]),
                "{name} {text}"
            );
        }
    }
}
