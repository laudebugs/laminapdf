//! Saving the outline and named-destination ops (E3 plan §2.1): each op's verdict on the signed
//! fixtures (class `Document`: every certification level forbids it, approval signatures and
//! field locks don't), undo and redo after a save writing the same bytes, and a save reopened
//! with its outline and everything else as written, `qpdf --check` clean.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use std::process::Command;

use common::fixture;
use common::save::{corpus_doc, editable, facts};
use laminapdf_model::{OutlineItemId, SiblingPosition};
use laminapdf_ops::{
    AddOutlineItem, ChildItems, DeleteNamedDestination, DeleteOutlineItem, DestinationView, Document,
    ExplicitDestination, MoveOutlineItem, Op, Origin, OutlineStyle, OutlineTarget, RenameNamedDestination, Rgb,
    SetNamedDestination, UpdateOutlineItem,
};
use laminapdf_save::{ConstraintKind, DocMdpPermission, Restriction, Save, SaveMode, SaveOptions, SavePlanner};

fn top(doc: &Document) -> Vec<OutlineItemId> {
    doc.outline().unwrap().roots().filter_map(|i| i.id().cloned()).collect()
}

fn fit(doc: &Document, page: usize) -> ExplicitDestination {
    ExplicitDestination::new(
        doc.pages().unwrap().get(page).unwrap().id().clone(),
        DestinationView::Fit,
    )
}

/// One op of each kind, written for `doc` (a signed fixture names its own first page and has
/// no outline: the planner judges ops before they apply, so the IDs need not exist).
fn every_op(doc: &Document) -> Vec<Op> {
    let item: OutlineItemId = "o3.0".parse().unwrap();
    vec![
        AddOutlineItem::new(
            None,
            SiblingPosition::Last,
            "x",
            OutlineTarget::Destination(fit(doc, 0)),
        )
        .into(),
        UpdateOutlineItem::new(item.clone()).into(),
        MoveOutlineItem::new(item.clone(), None, SiblingPosition::First).into(),
        DeleteOutlineItem::new(item, ChildItems::Delete).into(),
        SetNamedDestination::new("a", fit(doc, 0)).into(),
        RenameNamedDestination::new("a", "b").into(),
        DeleteNamedDestination::new("a").into(),
    ]
}

mod would_violate {
    use super::*;

    fn verdicts(name: &str) -> (ConstraintKind, Vec<Vec<Restriction>>) {
        let doc = editable(&fixture(&format!("signed/{name}")));
        let planner = SavePlanner::new(&doc).unwrap();
        let restrictions = every_op(&doc)
            .iter()
            .map(|op| planner.would_violate(op).iter().map(|v| v.restriction).collect())
            .collect();
        (planner.constraint().kind(), restrictions)
    }

    fn none() -> (ConstraintKind, Vec<Vec<Restriction>>) {
        (ConstraintKind::AppendOnly, vec![Vec::new(); 7])
    }

    fn all(restriction: Restriction) -> (ConstraintKind, Vec<Vec<Restriction>>) {
        (ConstraintKind::AppendOnly, vec![vec![restriction]; 7])
    }

    #[test]
    fn an_approval_signature_allows_them_in_an_incremental_update() {
        assert_eq!(verdicts("approval.pdf"), none());
        assert_eq!(verdicts("two-signatures.pdf"), none());
    }

    #[test]
    fn a_fieldmdp_lock_allows_them_since_they_change_no_field() {
        assert_eq!(verdicts("locked.pdf"), none());
    }

    #[test]
    fn every_certification_level_forbids_them() {
        assert_eq!(
            verdicts("certified-p1.pdf"),
            all(Restriction::DocMdp(DocMdpPermission::NoChanges))
        );
        assert_eq!(
            verdicts("certified-p2.pdf"),
            all(Restriction::DocMdp(DocMdpPermission::FormFill))
        );
        assert_eq!(
            verdicts("certified-p3.pdf"),
            all(Restriction::DocMdp(DocMdpPermission::Annotate))
        );
    }
}

/// Ops over `corpus/outline.pdf` that each change something.
fn outline_ops(doc: &Document) -> Vec<Op> {
    let [intro, chapter, appendix] = <[OutlineItemId; 3]>::try_from(top(doc)).unwrap();
    let mut add = AddOutlineItem::new(
        Some(chapter.clone()),
        SiblingPosition::First,
        "Résumé",
        OutlineTarget::Destination(fit(doc, 2)),
    );
    add.style = OutlineStyle::new(Some(Rgb::new(0, 96, 0)), true, false);
    let mut update = UpdateOutlineItem::new(appendix.clone());
    update.title = Some("Appendices".into());
    update.open = Some(true);
    vec![
        add.into(),
        update.into(),
        MoveOutlineItem::new(appendix, Some(intro), SiblingPosition::Last).into(),
        DeleteOutlineItem::new(chapter, ChildItems::Promote).into(),
        SetNamedDestination::new("results", fit(doc, 1)).into(),
        RenameNamedDestination::new("chapter2", "methods").into(),
        DeleteNamedDestination::new("appendix").into(),
    ]
}

fn save(doc: &mut Document) -> Vec<u8> {
    let mut out = Vec::new();
    doc.save(&mut out, &SaveOptions::new().with_mode(SaveMode::Incremental))
        .unwrap();
    out
}

#[test]
fn an_op_undone_after_a_save_writes_the_saved_bytes_and_redone_writes_its_own() {
    let ops = outline_ops(&corpus_doc("outline.pdf"));
    for op in ops {
        let mut doc = corpus_doc("outline.pdf");
        let saved = save(&mut doc);
        doc.apply_all(Origin::Local, [op.clone()]).unwrap();
        let applied = save(&mut corpus_doc_with(&op));
        doc.undo().unwrap();
        assert!(!doc.is_dirty(), "{op:?}");
        assert_eq!(save(&mut doc), saved, "undone: {op:?}");
        doc.redo().unwrap();
        assert_eq!(save(&mut doc), applied, "redone: {op:?}");
    }
}

/// `corpus/outline.pdf`, saved once (as the test above saves it first), with `op` applied.
fn corpus_doc_with(op: &Op) -> Document {
    let mut doc = corpus_doc("outline.pdf");
    save(&mut doc);
    doc.apply_all(Origin::Local, [op.clone()]).unwrap();
    doc
}

#[test]
fn a_saved_outline_reopens_as_written_with_everything_else_kept() {
    for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
        let mut doc = corpus_doc("outline.pdf");
        let before = facts(&doc);
        let ops = outline_ops(&doc);
        doc.apply_all(Origin::Local, ops).unwrap();
        let outline: Vec<(usize, String)> = doc
            .outline()
            .unwrap()
            .iter()
            .map(|i| (i.depth(), i.title().unwrap()))
            .collect();
        let mut out = Vec::new();
        doc.save(&mut out, &SaveOptions::new().with_mode(mode)).unwrap();
        let reopened = laminapdf_model::Document::open(out.clone(), laminapdf_cos::OpenOptions::default()).unwrap();
        assert!(!reopened.was_repaired(), "{mode:?}");
        let again: Vec<(usize, String)> = reopened
            .outline()
            .unwrap()
            .iter()
            .map(|i| (i.depth(), i.title().unwrap()))
            .collect();
        assert_eq!(again, outline, "{mode:?}");
        let after = facts(&reopened);
        assert_eq!(
            (after.rotations, after.title, after.author, after.keywords),
            (
                before.rotations.clone(),
                before.title.clone(),
                before.author.clone(),
                before.keywords.clone()
            ),
            "{mode:?}"
        );
        assert!(
            reopened
                .outline()
                .unwrap()
                .named_destination(b"results")
                .unwrap()
                .is_some()
        );
        qpdf_check(&out, &format!("{mode:?}"));
    }
}

fn qpdf_check(bytes: &[u8], label: &str) {
    let dir = std::env::temp_dir().join("laminapdf-save-outline");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{label}-{}.pdf", std::process::id()));
    std::fs::write(&path, bytes).unwrap();
    match Command::new("qpdf").arg("--check").arg(&path).output() {
        Ok(result) => assert!(
            result.status.success(),
            "qpdf --check {label}: {}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        ),
        Err(_) => eprintln!("qpdf not on PATH: skipping qpdf --check of {label}"),
    }
    let _ = std::fs::remove_file(path);
}
