//! Saving `EditContent`, `EditText` and `SetTextStyle` (ADR 0007),
//! and `SetAlt`: their verdict on the signed fixtures. The content ops are class `Content`:
//! approval signatures and a `FieldMDP` lock allow it, as it changes no field; every
//! certification forbids it, since none of P1–P3 allows changing what a page draws (ISO 32000-2
//! §12.8.2.2 Table 257). `SetAlt` changes a structure element, not what a page draws: class
//! `Document`, which approval signatures and a lock allow and every certification forbids.
#![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::fixture;
use common::save::editable;
use laminapdf_content::{ContentOps, ContentOwner, interpret_page};
use laminapdf_ops::{ContentEdit, EditContent, EditText, ObjectRef, Op, SetAlt, SetTextStyle, TextRunRef};
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

/// `SetAlt` changes a structure element, a `Document` change: the same two verdicts as the content
/// ops, since approval signatures and a lock allow it and each certification forbids it.
#[test]
fn set_alt_is_allowed_by_approval_signatures_and_forbidden_by_every_certification() {
    for (name, expected) in [
        ("approval.pdf", Vec::new()),
        ("two-signatures.pdf", Vec::new()),
        ("locked.pdf", Vec::new()),
        (
            "certified-p1.pdf",
            vec![Restriction::DocMdp(DocMdpPermission::NoChanges)],
        ),
        (
            "certified-p2.pdf",
            vec![Restriction::DocMdp(DocMdpPermission::FormFill)],
        ),
        (
            "certified-p3.pdf",
            vec![Restriction::DocMdp(DocMdpPermission::Annotate)],
        ),
    ] {
        let doc = editable(&fixture(&format!("signed/{name}")));
        let page = doc.pages().unwrap().get(0).unwrap().id().clone();
        let op: Op = SetAlt::new(page, ObjectRef::new(0, 0), "A picture").into();
        let planner = SavePlanner::new(&doc).unwrap();
        let restrictions: Vec<_> = planner.would_violate(&op).iter().map(|v| v.restriction).collect();
        assert_eq!(
            (planner.constraint().kind(), restrictions),
            (ConstraintKind::AppendOnly, expected),
            "{name}"
        );
    }
}

/// The same verdicts for an op that names an object its page really draws, as `content_objects`
/// names it (the fixtures above draw nothing). The planner judges the op before it applies, so
/// the file needn't be tagged.
#[test]
fn set_alt_naming_a_drawn_object_gets_the_same_verdicts() {
    for (name, expected) in [
        ("approval-organised.pdf", Vec::new()),
        ("locked-page1.pdf", Vec::new()),
        (
            "certified-p2-outline.pdf",
            vec![Restriction::DocMdp(DocMdpPermission::FormFill)],
        ),
    ] {
        let doc = editable(&fixture(&format!("signed/{name}")));
        let (page, object) = first_object(&doc);
        assert_ne!(object, ObjectRef::new(0, 0), "{name}: the name has its hash");
        let op: Op = SetAlt::new(page, object, "A picture").into();
        let planner = SavePlanner::new(&doc).unwrap();
        let restrictions: Vec<_> = planner.would_violate(&op).iter().map(|v| v.restriction).collect();
        assert_eq!(
            (planner.constraint().kind(), restrictions),
            (ConstraintKind::AppendOnly, expected),
            "{name}"
        );
    }
}

/// The first page and the name of the first object it draws.
fn first_object(doc: &laminapdf_ops::Document) -> (laminapdf_model::PageId, ObjectRef) {
    let pages = doc.pages().unwrap();
    let page = pages.get(0).unwrap();
    let mut ir = ContentOps::read_page(doc, page.cos(), ContentOwner::Page(page.id().clone())).unwrap();
    interpret_page(doc, &page, &mut ir.recorder(page.frame().unwrap())).unwrap();
    let id = ir.objects().first().unwrap().id();
    (page.id().clone(), ObjectRef::from(&ir.reference(id).unwrap()))
}
