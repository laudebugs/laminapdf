//! Saving the link ops (E3 plan §2.1, §6): each op's verdict on the signed fixtures (class
//! `Annotation`: a certification that allows annotations (P3) and approval signatures allow
//! them, P1 and P2 forbid them), undo and redo after a save writing the same bytes, and a save
//! reopened with its links as written, `qpdf --check` clean, tagged and untagged.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use std::process::Command;

use common::fixture;
use common::save::editable;
use laminapdf_model::{AnnotId, LinkAction, UserSpacePoint, UserSpaceRect};
use laminapdf_ops::{
    AddLink, DeleteLink, DestinationView, Document, ExplicitDestination, Op, Origin, OutlineTarget, UpdateLink,
};
use laminapdf_save::{ConstraintKind, DocMdpPermission, Restriction, Save, SaveMode, SaveOptions, SavePlanner};

fn rect() -> UserSpaceRect {
    UserSpaceRect::from_corners(36.0, 380.0, 236.0, 394.0)
}

/// One op of each kind for `doc`'s first page (the planner judges ops before they apply, so
/// the link they name need not exist).
fn every_op(doc: &Document) -> Vec<Op> {
    let page = doc.pages().unwrap().get(0).unwrap().id().clone();
    let link: AnnotId = "a5.0".parse().unwrap();
    vec![
        AddLink::new(page, rect(), OutlineTarget::Uri("https://example.com/".into())).into(),
        UpdateLink::new(link.clone()).into(),
        DeleteLink::new(link).into(),
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
        (ConstraintKind::AppendOnly, vec![Vec::new(); 3])
    }

    #[test]
    fn approval_signatures_and_a_certification_that_allows_annotations_allow_them() {
        assert_eq!(verdicts("approval.pdf"), none());
        assert_eq!(verdicts("two-signatures.pdf"), none());
        assert_eq!(verdicts("certified-p3.pdf"), none());
    }

    #[test]
    fn a_fieldmdp_lock_allows_them_since_they_change_no_field() {
        assert_eq!(verdicts("locked.pdf"), none());
    }

    #[test]
    fn a_certification_without_annotations_forbids_them() {
        let all = |p| (ConstraintKind::AppendOnly, vec![vec![Restriction::DocMdp(p)]; 3]);
        assert_eq!(verdicts("certified-p1.pdf"), all(DocMdpPermission::NoChanges));
        assert_eq!(verdicts("certified-p2.pdf"), all(DocMdpPermission::FormFill));
    }
}

/// Ops over `golden/links-and-outlines.pdf` (first page `p9.0`, links `a5.0` to `a8.0`) that
/// each change something.
fn link_ops(doc: &Document) -> Vec<Op> {
    let pages = doc.pages().unwrap();
    let first = pages.get(0).unwrap().id().clone();
    let second = pages.get(1).unwrap().id().clone();
    let mut on_text = AddLink::new(first, rect(), OutlineTarget::Named("chapter-2".into()));
    on_text.quads = vec![[
        UserSpacePoint::new(36.0, 394.0),
        UserSpacePoint::new(236.0, 394.0),
        UserSpacePoint::new(36.0, 380.0),
        UserSpacePoint::new(236.0, 380.0),
    ]];
    on_text.contents = Some("Chapter 2".into());
    let mut retarget = UpdateLink::new("a5.0".parse().unwrap());
    retarget.target = Some(OutlineTarget::Destination(ExplicitDestination::new(
        second,
        DestinationView::Fit,
    )));
    retarget.contents = Some(None);
    vec![
        on_text.into(),
        retarget.into(),
        DeleteLink::new("a7.0".parse().unwrap()).into(),
    ]
}

fn links_doc() -> Document {
    editable(&fixture("golden/links-and-outlines.pdf"))
}

fn save(doc: &mut Document) -> Vec<u8> {
    let mut out = Vec::new();
    doc.save(&mut out, &SaveOptions::new().with_mode(SaveMode::Incremental))
        .unwrap();
    out
}

#[test]
fn an_op_undone_after_a_save_writes_the_saved_bytes_and_redone_writes_its_own() {
    for op in link_ops(&links_doc()) {
        let mut doc = links_doc();
        let saved = save(&mut doc);
        doc.apply_all(Origin::Local, [op.clone()]).unwrap();
        let mut once = links_doc();
        save(&mut once);
        once.apply_all(Origin::Local, [op.clone()]).unwrap();
        let applied = save(&mut once);
        doc.undo().unwrap();
        assert!(!doc.is_dirty(), "{op:?}");
        assert_eq!(save(&mut doc), saved, "undone: {op:?}");
        doc.redo().unwrap();
        assert_eq!(save(&mut doc), applied, "redone: {op:?}");
    }
}

/// Each page's links as (rectangle, quads, action, description).
type Seen = Vec<Vec<(UserSpaceRect, Vec<[UserSpacePoint; 4]>, LinkAction, Option<String>)>>;

fn seen(doc: &laminapdf_model::Document) -> Seen {
    let pages = doc.pages().unwrap();
    pages
        .iter()
        .map(|page| {
            pages
                .links(&page)
                .unwrap()
                .into_iter()
                .map(|l| (l.rect, l.quads, l.action, l.contents))
                .collect()
        })
        .collect()
}

#[test]
fn saved_links_reopen_as_written() {
    for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
        let mut doc = links_doc();
        let ops = link_ops(&doc);
        doc.apply_all(Origin::Local, ops).unwrap();
        let links = seen(&doc);
        let mut out = Vec::new();
        doc.save(&mut out, &SaveOptions::new().with_mode(mode)).unwrap();
        let reopened = laminapdf_model::Document::open(out.clone(), laminapdf_cos::OpenOptions::default()).unwrap();
        assert!(!reopened.was_repaired(), "{mode:?}");
        assert_eq!(seen(&reopened), links, "{mode:?}");
        qpdf_check(&out, &format!("{mode:?}"));
    }
}

#[test]
fn a_tagged_link_saves_with_its_structure() {
    for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
        let mut doc = editable(&fixture("golden/tagged-report.pdf"));
        let page = doc.pages().unwrap().get(0).unwrap().id().clone();
        let mut add = AddLink::new(page, rect(), OutlineTarget::Uri("https://example.com/".into()));
        add.marked = vec![0];
        doc.apply_all(Origin::Local, [Op::from(add)]).unwrap();
        let mut out = Vec::new();
        doc.save(&mut out, &SaveOptions::new().with_mode(mode)).unwrap();
        let reopened = laminapdf_model::Document::open(out.clone(), laminapdf_cos::OpenOptions::default()).unwrap();
        let tree = reopened.structure().unwrap().unwrap();
        let kinds: Vec<Vec<u8>> = tree
            .roots
            .iter()
            .flat_map(|r| std::iter::once(r).chain(r.descendants()))
            .map(|e| e.kind.clone())
            .collect();
        assert!(kinds.iter().any(|k| k == b"Link"), "{mode:?}: {kinds:?}");
        qpdf_check(&out, &format!("tagged-{mode:?}"));
    }
}

fn qpdf_check(bytes: &[u8], label: &str) {
    let dir = std::env::temp_dir().join("laminapdf-save-links");
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
