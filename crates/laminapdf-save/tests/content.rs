//! Saving `EditContent` (todo E7d), `EditText` (todo E7e) and `SetTextStyle` (todo E7f, ADR 0007): their verdict on the signed fixtures (class
//! `Content`: approval signatures and a `FieldMDP` lock allow it, as it changes no field; every
//! certification forbids it, since none of P1–P3 allows changing what a page draws, ISO 32000-2
//! §12.8.2.2 Table 257).
#![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::fixture;
use common::save::editable;
use laminapdf_ops::{ContentEdit, EditContent, EditText, ObjectRef, Op, SetTextStyle, TextRunRef};
use laminapdf_save::{ConstraintKind, DocMdpPermission, Restriction, SavePlanner};

/// The content ops the verdicts are for.
#[derive(Debug, Clone, Copy)]
enum Kind {
    Delete,
    Text,
    Style,
}

const KINDS: [Kind; 3] = [Kind::Delete, Kind::Text, Kind::Style];

/// Each signed fixture's verdict on deleting an object of its first page, rewriting a run of its
/// text or restyling it (the planner judges an op before it applies, so the object need not exist).
fn verdict(name: &str, kind: Kind) -> (ConstraintKind, Vec<Restriction>) {
    let doc = editable(&fixture(&format!("signed/{name}")));
    let page = doc.pages().unwrap().get(0).unwrap().id().clone();
    let object = ObjectRef::new(0, 0);
    let run = TextRunRef::new(object, 0, 1);
    let op: Op = match kind {
        Kind::Text => EditText::new(page, run, "x".to_owned()).into(),
        Kind::Delete => EditContent::new(page, vec![ContentEdit::Delete { object }]).into(),
        Kind::Style => {
            let mut style = SetTextStyle::new(page, vec![run]);
            style.bold = Some(true);
            style.into()
        }
    };
    let planner = SavePlanner::new(&doc).unwrap();
    let restrictions = planner.would_violate(&op).iter().map(|v| v.restriction).collect();
    (planner.constraint().kind(), restrictions)
}

#[test]
fn approval_signatures_and_a_lock_allow_content_edits() {
    for name in ["approval.pdf", "two-signatures.pdf", "locked.pdf"] {
        for kind in KINDS {
            assert_eq!(
                verdict(name, kind),
                (ConstraintKind::AppendOnly, Vec::new()),
                "{name} {kind:?}"
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
        for kind in KINDS {
            assert_eq!(
                verdict(name, kind),
                (ConstraintKind::AppendOnly, vec![Restriction::DocMdp(permission)]),
                "{name} {kind:?}"
            );
        }
    }
}
