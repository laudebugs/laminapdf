//! Saving stamps and file attachments (E5 plan §5, todo E5c): `AddAnnot`'s verdict for each kind
//! of stamp and for a file attachment on the signed fixtures (class `Annotation`), each saved in
//! both modes and reopened with its appearance, `qpdf --check` clean, undo and redo that give
//! back the same bytes, and on a PDF/A file, only the dynamic stamp's Helvetica warns.
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
use laminapdf_ops::{
    AddAnnot, AnnotSpec, AttachmentIcon, Document, DynamicStamp, FileAttachment, FileToAttach, Op, Origin, Stamp,
    StampKind, StampName, StampStyle,
};
use laminapdf_save::{ConstraintKind, DocMdpPermission, Restriction, Save as _, SaveMode, SaveOptions, SavePlanner};

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> UserSpaceRect {
    UserSpaceRect::from_corners(x0, y0, x1, y1)
}

/// A standard, an image and a dynamic stamp and a file attachment on `doc`'s first page, their
/// bytes added to its session.
fn every_mark(doc: &mut Document) -> Vec<Op> {
    let jpeg = doc.add_blob(fixture("images/photo-rgb.jpg"));
    let csv = doc.add_blob(b"day,reading\n1,12.5\n".to_vec());
    let page = doc.pages().unwrap().get(0).unwrap().id().clone();
    let add = |spec| Op::from(AddAnnot::new(page.clone(), spec));
    let stamp = |r, kind| AnnotSpec::Stamp(Stamp::new(r, kind));
    let dynamic = DynamicStamp::new(vec!["RECEIVED".into(), "1 Dec 2026".into()], StampStyle::Blue);
    let file = FileToAttach::new("readings.csv", csv, "text/csv");
    vec![
        add(stamp(
            rect(72.0, 600.0, 272.0, 650.0),
            StampKind::Standard(StampName::Approved),
        )),
        add(stamp(rect(300.0, 600.0, 400.0, 680.0), StampKind::Image(jpeg))),
        add(stamp(rect(72.0, 500.0, 272.0, 560.0), StampKind::Dynamic(dynamic))),
        add(AnnotSpec::FileAttachment(FileAttachment::new(
            UserSpacePoint::new(40.0, 760.0),
            AttachmentIcon::Paperclip,
            file,
        ))),
    ]
}

mod would_violate {
    use super::*;

    fn verdicts(name: &str) -> (ConstraintKind, Vec<Vec<Restriction>>) {
        let mut doc = editable(&fixture(&format!("signed/{name}")));
        let ops = every_mark(&mut doc);
        let planner = SavePlanner::new(&doc).unwrap();
        let restrictions = ops
            .iter()
            .map(|op| planner.would_violate(op).iter().map(|v| v.restriction).collect())
            .collect();
        (planner.constraint().kind(), restrictions)
    }

    fn none() -> (ConstraintKind, Vec<Vec<Restriction>>) {
        (ConstraintKind::AppendOnly, vec![Vec::new(); 4])
    }

    #[test]
    fn approval_signatures_a_field_lock_and_a_certification_that_allows_annotations_allow_them() {
        assert_eq!(
            [
                verdicts("approval.pdf"),
                verdicts("two-signatures.pdf"),
                verdicts("certified-p3.pdf"),
                verdicts("locked.pdf")
            ],
            [none(), none(), none(), none()]
        );
    }

    #[test]
    fn a_certification_without_annotations_forbids_them() {
        let all = |p| (ConstraintKind::AppendOnly, vec![vec![Restriction::DocMdp(p)]; 4]);
        assert_eq!(
            [verdicts("certified-p1.pdf"), verdicts("certified-p2.pdf")],
            [all(DocMdpPermission::NoChanges), all(DocMdpPermission::FormFill)]
        );
    }
}

fn qpdf_accepts(bytes: &[u8]) -> bool {
    let dir = std::env::temp_dir().join(format!("lamina-stamps-{}", std::process::id()));
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
            let ops = every_mark(&mut doc);
            doc.apply_all(Origin::Local, ops).unwrap();
            let mut bytes = Vec::new();
            doc.save(&mut bytes, &SaveOptions::new().with_mode(mode)).unwrap();
            let reopened = laminapdf_model::Document::open(bytes.clone(), OpenOptions::default()).unwrap();
            let pages = reopened.pages().unwrap();
            let marks: Vec<(String, bool)> = reopened
                .annotations(&pages.get(0).unwrap())
                .unwrap()
                .iter()
                .filter(|a| matches!(a.subtype.name(), b"Stamp" | b"FileAttachment"))
                .map(|a| (String::from_utf8_lossy(a.subtype.name()).into_owned(), a.has_appearance))
                .collect();
            let expected = ["Stamp", "Stamp", "Stamp", "FileAttachment"].map(|s| (s.to_owned(), true));
            assert_eq!(
                (marks, qpdf_accepts(&bytes)),
                (expected.to_vec(), true),
                "{name} {mode:?}"
            );
            if name.contains("tagged") {
                assert_eq!(tagged_marks(&reopened), 4, "{name} {mode:?}");
            }
        }
    }
}

/// How many stamps and file attachments on the first page are tagged: each the object
/// reference of an `Annot` structure element.
fn tagged_marks(doc: &laminapdf_model::Document) -> usize {
    use laminapdf_model::StructKid;
    let pages = doc.pages().unwrap();
    let tree = doc.structure().unwrap().unwrap();
    let elements = tree.elements();
    doc.annotations(&pages.get(0).unwrap())
        .unwrap()
        .iter()
        .filter(|a| matches!(a.subtype.name(), b"Stamp" | b"FileAttachment"))
        .filter(|a| {
            let r = a.id.as_lamina_id().obj_ref().unwrap();
            elements.iter().any(|e| {
                e.kind == b"Annot"
                    && e.kids
                        .iter()
                        .any(|k| matches!(k, StructKid::Object { object, .. } if *object == r))
            })
        })
        .count()
}

/// The incremental save of `doc` as it is now.
fn saved(doc: &mut Document) -> Vec<u8> {
    let mut bytes = Vec::new();
    doc.save(&mut bytes, &SaveOptions::new().with_mode(SaveMode::Incremental))
        .unwrap();
    bytes
}

/// The fixture opened for editing, with every mark applied when `apply` says so.
fn hello(apply: bool) -> Document {
    let mut doc = editable(&fixture("golden/examples/hello.pdf")).with_appearances(AnnotationAppearances);
    let ops = every_mark(&mut doc);
    if apply {
        doc.apply_all(Origin::Local, ops).unwrap();
    }
    doc
}

#[test]
fn undo_leaves_the_overlay_empty_and_saves_what_the_file_was() {
    let mut undone = hello(true);
    undone.undo().unwrap();
    let empty = undone.cos().overlay().is_empty();
    assert_eq!((empty, saved(&mut undone)), (true, saved(&mut hello(false))));
}

#[test]
fn redo_writes_the_same_bytes_as_the_edit() {
    let mut redone = hello(true);
    redone.undo().unwrap();
    redone.redo().unwrap();
    assert_eq!(saved(&mut redone), saved(&mut hello(true)));
}

/// The conformance warnings' codes after `kind` is stamped on the PDF/A-2b example.
fn pdfa_with(kind: StampKind) -> Vec<String> {
    let mut doc = editable(&fixture("golden/examples/pdf_a_archive-2b.pdf")).with_appearances(AnnotationAppearances);
    let page = doc.pages().unwrap().get(0).unwrap().id().clone();
    let spec = AnnotSpec::Stamp(Stamp::new(rect(72.0, 600.0, 272.0, 650.0), kind));
    doc.apply_all(Origin::Local, [Op::from(AddAnnot::new(page, spec))])
        .unwrap();
    SavePlanner::new(&doc)
        .unwrap()
        .conformance_warnings()
        .iter()
        .map(|w| w.code().to_owned())
        .collect()
}

#[test]
fn a_standard_stamp_keeps_a_pdfa_files_fonts_embedded_and_a_dynamic_one_doesnt() {
    let standard = pdfa_with(StampKind::Standard(StampName::Final));
    let dynamic = pdfa_with(StampKind::Dynamic(DynamicStamp::new(
        vec!["FINAL".into()],
        StampStyle::Green,
    )));
    let font = "pdfa-font-not-embedded".to_owned();
    assert_eq!((standard.contains(&font), dynamic.contains(&font)), (false, true));
}
