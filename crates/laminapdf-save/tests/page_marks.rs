//! Saving page marks (E6 plan §2.3 and §5, todo E6d): the verdict of each op on the signed
//! fixtures (class `Content`), a mark saved in both modes and found again after reopening, a mark
//! changed and deleted after a save, a hideable mark's group in the saved file, and undo and
//! redo that give back the same bytes.
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
use laminapdf_model::PdfDate;
use laminapdf_ops::{
    AddBackground, AddWatermark, ApplyBates, ContentId, DeletePageMark, Document, MarkContent, Op, Origin, PageMark,
    SetHeaderFooter, Slot, UpdatePageMark, WatermarkSpec, page_marks,
};
use laminapdf_save::{ConstraintKind, DocMdpPermission, Restriction, Save as _, SaveMode, SaveOptions, SavePlanner};

fn id(s: &str) -> ContentId {
    ContentId::new(s).unwrap()
}

fn date() -> PdfDate {
    "D:20261118120000Z".parse().unwrap()
}

fn text(s: &str) -> MarkContent {
    MarkContent::Text {
        text: s.to_owned(),
        fonts: Vec::new(),
        size: 48.0,
        color: None,
    }
}

fn pages(doc: &Document) -> Vec<laminapdf_model::PageId> {
    doc.pages().unwrap().iter().map(|p| p.id().clone()).collect()
}

fn save(doc: &mut Document, mode: SaveMode) -> Vec<u8> {
    let mut bytes = Vec::new();
    doc.save(&mut bytes, &SaveOptions::new().with_mode(mode)).unwrap();
    bytes
}

fn qpdf_accepts(bytes: &[u8]) -> bool {
    let path = std::env::temp_dir().join(format!("lamina-page-marks-{}.pdf", std::process::id()));
    std::fs::write(&path, bytes).unwrap();
    // No qpdf on the machine: the round trip runs it.
    Command::new("qpdf")
        .arg("--check")
        .arg(&path)
        .output()
        .map_or(true, |o| o.status.success())
}

/// A three-page file with every kind of mark on it.
fn marked() -> Document {
    let mut doc = editable(&fixture("corpus/outline.pdf")).with_appearances(AnnotationAppearances);
    let all = pages(&doc);
    let mut slots: [Option<String>; 6] = Default::default();
    slots[4] = Some("Page {page} of {pages}".to_owned());
    let ops: Vec<Op> = vec![
        AddWatermark::new(id("draft"), all.clone(), text("DRAFT"), date())
            .with_angle(45.0)
            .hideable()
            .into(),
        AddBackground::new(id("paper"), all.clone(), text("PAPER"), date()).into(),
        SetHeaderFooter::new(id("folio"), all.clone(), slots, date()).into(),
        ApplyBates::new(id("bates"), all, 1, Slot::BottomRight, date()).into(),
    ];
    doc.apply_all(Origin::Local, ops).unwrap();
    doc
}

mod would_violate {
    use super::*;

    fn verdicts(name: &str) -> (ConstraintKind, Vec<Restriction>) {
        let doc = editable(&fixture(&format!("signed/{name}")));
        let all = pages(&doc);
        let page = all[..1].to_vec();
        let mut slots: [Option<String>; 6] = Default::default();
        slots[0] = Some("x".to_owned());
        let ops: [Op; 6] = [
            AddWatermark::new(id("m"), page.clone(), text("D"), date()).into(),
            AddBackground::new(id("m"), page.clone(), text("D"), date()).into(),
            SetHeaderFooter::new(id("m"), page.clone(), slots, date()).into(),
            ApplyBates::new(id("m"), page.clone(), 1, Slot::TopLeft, date()).into(),
            UpdatePageMark::new(
                id("m"),
                page.clone(),
                PageMark::Watermark(WatermarkSpec::new(text("D"))),
                date(),
            )
            .into(),
            DeletePageMark::new(id("m"), page).into(),
        ];
        let planner = SavePlanner::new(&doc).unwrap();
        // The six ops' verdicts are the same: a mark is page content in every way.
        let verdicts: Vec<Vec<Restriction>> = ops
            .iter()
            .map(|op| {
                assert_eq!(op.class(), laminapdf_ops::ModificationClass::Content);
                assert!(!op.requires_rewrite());
                planner.would_violate(op).iter().map(|v| v.restriction).collect()
            })
            .collect();
        assert!(verdicts.iter().all(|v| *v == verdicts[0]), "{name}: {verdicts:?}");
        (planner.constraint().kind(), verdicts[0].clone())
    }

    #[test]
    fn an_unsigned_file_and_approval_signatures_and_a_field_lock_allow_marks() {
        for name in ["approval.pdf", "two-signatures.pdf", "locked.pdf"] {
            assert_eq!(verdicts(name), (ConstraintKind::AppendOnly, Vec::new()), "{name}");
        }
        let plain = editable(&fixture("corpus/hello.pdf"));
        let planner = SavePlanner::new(&plain).unwrap();
        let op: Op = AddWatermark::new(id("m"), pages(&plain)[..1].to_vec(), text("D"), date()).into();
        assert_eq!(planner.would_violate(&op), []);
        assert_ne!(planner.constraint().kind(), ConstraintKind::Conflict);
    }

    #[test]
    fn every_certification_level_forbids_them() {
        for (name, level) in [
            ("certified-p1.pdf", DocMdpPermission::NoChanges),
            ("certified-p2.pdf", DocMdpPermission::FormFill),
            ("certified-p3.pdf", DocMdpPermission::Annotate),
        ] {
            assert_eq!(
                verdicts(name),
                (ConstraintKind::AppendOnly, vec![Restriction::DocMdp(level)]),
                "{name}"
            );
        }
    }
}

#[test]
fn every_mark_saves_in_both_modes_and_is_found_again_after_reopening() {
    for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
        let mut doc = marked();
        let bytes = save(&mut doc, mode);
        assert!(qpdf_accepts(&bytes), "{mode:?}");
        let reopened = editable(&bytes);
        for page in pages(&reopened) {
            let found: Vec<String> = page_marks(&reopened, &page)
                .unwrap()
                .iter()
                .map(|m| m.id.to_string())
                .collect();
            assert_eq!(found, ["paper", "draft", "folio", "bates"], "{mode:?}");
        }
        // The hideable watermark's group is in the saved file's catalog, once.
        let catalog = reopened.catalog().unwrap();
        let props = reopened.deref(catalog.get("OCProperties").unwrap()).unwrap();
        let groups = props
            .as_dict()
            .unwrap()
            .get("OCGs")
            .and_then(laminapdf_cos::Object::as_array);
        assert_eq!(groups.map(<[laminapdf_cos::Object]>::len), Some(1), "{mode:?}");
    }
}

#[test]
fn a_mark_added_before_a_save_is_changed_and_deleted_after_it() {
    let mut doc = marked();
    let bytes = save(&mut doc, SaveMode::Incremental);
    let mut doc = editable(&bytes).with_appearances(AnnotationAppearances);
    let all = pages(&doc);
    doc.apply_all(
        Origin::Local,
        [
            UpdatePageMark::new(
                id("draft"),
                all.clone(),
                PageMark::Watermark(WatermarkSpec::new(text("FINAL"))),
                date(),
            )
            .into(),
            DeletePageMark::new(id("bates"), all.clone()).into(),
        ],
    )
    .unwrap();
    let bytes = save(&mut doc, SaveMode::Incremental);
    assert!(qpdf_accepts(&bytes));
    let reopened = editable(&bytes);
    let marks = page_marks(&reopened, &all[0]).unwrap();
    assert_eq!(
        marks.iter().map(|m| m.id.to_string()).collect::<Vec<_>>(),
        ["paper", "draft", "folio"]
    );
    // The update made the watermark plain: it left the layers panel.
    assert!(!marks[1].hideable);
    let catalog = reopened.catalog().unwrap();
    assert!(catalog.get("OCProperties").is_none());
}

#[test]
fn undo_leaves_the_overlay_empty_and_saves_what_the_file_was_and_redo_the_edit() {
    let mut undone = marked();
    // One undo for each of the four ops.
    for _ in 0..4 {
        undone.undo().unwrap();
    }
    assert!(undone.cos().overlay().is_empty());
    let mut untouched = editable(&fixture("corpus/outline.pdf"));
    assert_eq!(
        save(&mut undone, SaveMode::Incremental),
        save(&mut untouched, SaveMode::Incremental)
    );
    let mut redone = marked();
    for _ in 0..4 {
        redone.undo().unwrap();
    }
    for _ in 0..4 {
        redone.redo().unwrap();
    }
    assert_eq!(
        save(&mut redone, SaveMode::Incremental),
        save(&mut marked(), SaveMode::Incremental)
    );
}

#[test]
fn a_tagged_file_stays_tagged_and_the_marks_join_no_structure() {
    let mut doc = editable(&fixture("golden/tagged-report.pdf")).with_appearances(AnnotationAppearances);
    let all = pages(&doc);
    let before = {
        let catalog = doc.catalog().unwrap();
        format!("{:?}", doc.deref(catalog.get("StructTreeRoot").unwrap()).unwrap())
    };
    doc.apply_all(
        Origin::Local,
        [AddWatermark::new(id("draft"), all[..1].to_vec(), text("DRAFT"), date()).into()],
    )
    .unwrap();
    let bytes = save(&mut doc, SaveMode::Incremental);
    assert!(qpdf_accepts(&bytes));
    let reopened = laminapdf_model::Document::open(bytes, laminapdf_cos::OpenOptions::default()).unwrap();
    let catalog = reopened.catalog().unwrap();
    // The structure tree is the one the file had: the watermark is an artifact.
    assert_eq!(
        format!("{:?}", reopened.deref(catalog.get("StructTreeRoot").unwrap()).unwrap()),
        before
    );
}

fn replace(doc: &mut Document) {
    let new = PageMark::Watermark(WatermarkSpec::new(text("FINAL")));
    let op = UpdatePageMark::new(id("draft"), pages(doc), new, date());
    doc.apply_all(Origin::Local, [op.into()]).unwrap();
}

fn delete(doc: &mut Document) {
    let op = DeletePageMark::new(id("bates"), pages(doc));
    doc.apply_all(Origin::Local, [op.into()]).unwrap();
}

/// The marked file saved and reopened, then `change` applied to it.
fn changed(change: fn(&mut Document)) -> (Document, Vec<u8>) {
    let base = save(&mut marked(), SaveMode::Incremental);
    let mut doc = editable(&base).with_appearances(AnnotationAppearances);
    change(&mut doc);
    (doc, base)
}

#[test]
fn undoing_a_replaced_or_a_deleted_mark_saves_what_the_file_was_and_redo_the_edit() {
    for change in [replace as fn(&mut Document), delete] {
        let edited = save(&mut changed(change).0, SaveMode::Incremental);
        let (mut doc, base) = changed(change);
        doc.undo().unwrap();
        let empty = doc.cos().overlay().is_empty();
        let unchanged = save(&mut editable(&base), SaveMode::Incremental);
        let undone = save(&mut doc, SaveMode::Incremental);
        let (mut doc, _) = changed(change);
        doc.undo().unwrap();
        doc.redo().unwrap();
        let redone = save(&mut doc, SaveMode::Incremental);
        assert_eq!((empty, undone == unchanged, redone == edited), (true, true, true));
    }
}
