//! Saving the annotation ops that make marks (E4 plan §6): `AddAnnot`'s verdict on the signed
//! fixtures (class `Annotation`: a certification that allows annotations (P3) and approval
//! signatures allow it, P1 and P2 forbid it), and text markup, notes and carets saved in both
//! modes, reopened with their appearances, `qpdf --check` clean.
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use std::process::Command;

use common::fixture;
use common::save::editable;
use laminapdf_build::AnnotationAppearances;
use laminapdf_cos::OpenOptions;
use laminapdf_model::{UserSpacePoint, UserSpaceRect};
use laminapdf_ops::{AddAnnot, AnnotSpec, Caret, Document, MarkupKind, Note, NoteIcon, Op, Origin, TextMarkup};
use laminapdf_save::{ConstraintKind, DocMdpPermission, Restriction, Save as _, SaveMode, SaveOptions, SavePlanner};

/// A highlight, a note and a caret on `doc`'s first page.
fn every_mark(doc: &Document) -> Vec<Op> {
    let page = doc.pages().unwrap().get(0).unwrap().id().clone();
    let p = UserSpacePoint::new;
    let line = [p(72.0, 714.0), p(180.0, 714.0), p(72.0, 700.0), p(180.0, 700.0)];
    vec![
        AddAnnot::new(
            page.clone(),
            AnnotSpec::TextMarkup(TextMarkup::new(MarkupKind::Highlight, vec![line])),
        )
        .into(),
        AddAnnot::new(
            page.clone(),
            AnnotSpec::Note(Note::new(p(200.0, 714.0), NoteIcon::Comment)),
        )
        .into(),
        AddAnnot::new(
            page,
            AnnotSpec::Caret(Caret::new(UserSpaceRect::from_corners(178.0, 698.0, 184.0, 708.0))),
        )
        .into(),
    ]
}

mod would_violate {
    use super::*;

    fn verdicts(name: &str) -> (ConstraintKind, Vec<Vec<Restriction>>) {
        let doc = editable(&fixture(&format!("signed/{name}")));
        let planner = SavePlanner::new(&doc).unwrap();
        let restrictions = every_mark(&doc)
            .iter()
            .map(|op| planner.would_violate(op).iter().map(|v| v.restriction).collect())
            .collect();
        (planner.constraint().kind(), restrictions)
    }

    fn none() -> (ConstraintKind, Vec<Vec<Restriction>>) {
        (ConstraintKind::AppendOnly, vec![Vec::new(); 3])
    }

    #[test]
    fn approval_signatures_and_a_certification_that_allows_annotations_allow_marks() {
        assert_eq!(
            [
                verdicts("approval.pdf"),
                verdicts("two-signatures.pdf"),
                verdicts("certified-p3.pdf")
            ],
            [none(), none(), none()]
        );
    }

    #[test]
    fn a_fieldmdp_lock_allows_marks_since_they_change_no_field() {
        assert_eq!(verdicts("locked.pdf"), none());
    }

    #[test]
    fn a_certification_without_annotations_forbids_marks() {
        let all = |p| (ConstraintKind::AppendOnly, vec![vec![Restriction::DocMdp(p)]; 3]);
        assert_eq!(
            [verdicts("certified-p1.pdf"), verdicts("certified-p2.pdf")],
            [all(DocMdpPermission::NoChanges), all(DocMdpPermission::FormFill)]
        );
    }
}

fn qpdf_accepts(bytes: &[u8]) -> bool {
    let dir = std::env::temp_dir().join(format!("lamina-markup-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("out.pdf");
    std::fs::write(&path, bytes).unwrap();
    // No qpdf on the machine: the nightly round trip runs it.
    Command::new("qpdf")
        .arg("--check")
        .arg(&path)
        .output()
        .map_or(true, |o| o.status.success())
}

#[test]
fn every_mark_saves_in_both_modes_and_reopens_with_its_appearance() {
    for name in ["golden/examples/hello.pdf", "golden/tagged-report.pdf"] {
        for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
            let mut doc = editable(&fixture(name)).with_appearances(AnnotationAppearances);
            let ops = every_mark(&doc);
            doc.apply_all(Origin::Local, ops).unwrap();
            let mut bytes = Vec::new();
            doc.save(&mut bytes, &SaveOptions::new().with_mode(mode)).unwrap();
            let reopened = laminapdf_model::Document::open(bytes.clone(), OpenOptions::default()).unwrap();
            let pages = reopened.pages().unwrap();
            let annots = reopened.annotations(&pages.get(0).unwrap()).unwrap();
            let subtypes: Vec<&[u8]> = annots.iter().map(|a| a.subtype.name()).collect();
            // The file's own links have no appearance; every mark has one.
            let marks = [b"Highlight".as_slice(), b"Text", b"Caret"];
            let drawn = annots
                .iter()
                .filter(|a| marks.contains(&a.subtype.name()))
                .all(|a| a.has_appearance);
            assert_eq!(
                (
                    subtypes.ends_with(&[b"Highlight".as_slice(), b"Text", b"Popup", b"Caret"]),
                    drawn,
                    qpdf_accepts(&bytes)
                ),
                (true, true, true),
                "{name} {mode:?}"
            );
        }
    }
}
