//! `SetInitialView` through a save: the planner's verdict on the signed fixtures (ADR 0007
//! §2), the change surviving a save and a reopen, and undo writing the base bytes again
//! (ADR 0006 §3).
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "tests: a panic is a test failure"
)]

mod common;

use common::fixture;
use common::save::{corpus_doc, editable};
use laminapdf_model::{OpenAction, PageLayout, PageMode, View};
use laminapdf_ops::{Document, Op, OpenAt, OpenZoom, Origin, SetInitialView, WindowPreferencesPatch};
use laminapdf_save::{DocMdpPermission, Restriction, Save, SaveMode, SaveOptions, SavePlanner};

fn op(doc: &Document) -> Op {
    let page = doc.pages().unwrap().get(0).unwrap().id().clone();
    SetInitialView::default()
        .with_open(Some(OpenAt::new(page, OpenZoom::FitWidth)))
        .with_page_mode(Some(PageMode::Outlines))
        .with_page_layout(Some(PageLayout::OneColumn))
        .with_preferences(WindowPreferencesPatch::default().with_display_doc_title(true))
        .into()
}

#[test]
fn the_initial_view_survives_a_save_and_a_reopen() {
    for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
        let mut doc = corpus_doc("outline.pdf");
        let op = op(&doc);
        doc.apply_all(Origin::Local, [op]).unwrap();
        let mut out = Vec::new();
        doc.save(&mut out, &SaveOptions::new().with_mode(mode)).unwrap();
        let reopened = editable(&out);
        let view = reopened.initial_view().unwrap();
        let OpenAction::Destination(dest) = view.open else {
            panic!("{mode:?}: no destination");
        };
        assert_eq!(
            (dest.page_index, dest.view),
            (Some(0), View::FitH { top: None }),
            "{mode:?}"
        );
        assert_eq!(
            (view.page_mode, view.page_layout, view.preferences.display_doc_title),
            (Some(PageMode::Outlines), Some(PageLayout::OneColumn), true),
            "{mode:?}"
        );
        // Everything else is kept: the pages and the bookmarks.
        assert_eq!(reopened.pages().unwrap().len(), doc.pages().unwrap().len(), "{mode:?}");
        assert_eq!(
            reopened.outline().unwrap().len(),
            doc.outline().unwrap().len(),
            "{mode:?}"
        );
    }
}

#[test]
fn undoing_it_writes_the_base_unchanged() {
    for name in ["hello.pdf", "hello-xrefstm.pdf", "outline.pdf", "pdflib.pdf"] {
        let mut doc = corpus_doc(name);
        let base = doc.data().unwrap().to_vec();
        let op = op(&doc);
        doc.apply_all(Origin::Local, [op]).unwrap();
        doc.undo().unwrap();
        assert!(!doc.is_dirty(), "{name}");
        let mut out = Vec::new();
        doc.save(&mut out, &SaveOptions::new()).unwrap();
        assert_eq!(out, base, "{name}");
    }
}

/// A document-wide change: a conflict under every certification level and no conflict with
/// approval signatures or a field lock (ADR 0007 §2).
#[test]
fn the_planner_checks_it_against_each_signed_fixture() {
    let cases: [(&str, Vec<Restriction>); 5] = [
        ("approval.pdf", vec![]),
        ("locked.pdf", vec![]),
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
    ];
    for (name, expected) in cases {
        let doc = editable(&fixture(&format!("signed/{name}")));
        let restrictions: Vec<Restriction> = SavePlanner::new(&doc)
            .unwrap()
            .would_violate(&op(&doc))
            .into_iter()
            .map(|v| v.restriction)
            .collect();
        assert_eq!(restrictions, expected, "{name}");
    }
}
