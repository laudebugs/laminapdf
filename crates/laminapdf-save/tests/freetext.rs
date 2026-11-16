//! Saving text boxes (E4 plan §6, todo E4e): `AddAnnot`'s verdict for a text box, a callout and
//! typewriter text on the signed fixtures (class `Annotation`, as for the other marks), each saved
//! in both modes and reopened with its appearance, `qpdf --check` clean, and undo and redo that
//! give back the same bytes.
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
    AddAnnot, AnnotSpec, Callout, Document, FontRef, FreeText, FreeTextIntent, Op, Origin, RichText, StandardFamily,
    TextSpan,
};
use laminapdf_save::{ConstraintKind, DocMdpPermission, Restriction, Save as _, SaveMode, SaveOptions, SavePlanner};

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> UserSpaceRect {
    UserSpaceRect::from_corners(x0, y0, x1, y1)
}

/// A text box with bold text, a callout and typewriter text on `doc`'s first page.
fn every_box(doc: &Document) -> Vec<Op> {
    let page = doc.pages().unwrap().get(0).unwrap().id().clone();
    let add = |spec| Op::from(AddAnnot::new(page.clone(), AnnotSpec::FreeText(spec)));
    let rich = RichText {
        paragraphs: vec![vec![TextSpan::plain("Q2 is "), TextSpan::plain("late").with_bold(true)]],
    };
    let line = vec![UserSpacePoint::new(80.0, 500.0), UserSpacePoint::new(200.0, 560.0)];
    vec![
        add(FreeText::new(rect(72.0, 600.0, 272.0, 640.0), rich)),
        add(
            FreeText::new(rect(200.0, 540.0, 360.0, 570.0), RichText::plain("A callout"))
                .with_intent(FreeTextIntent::Callout(Callout::new(line))),
        ),
        add(
            FreeText::new(rect(72.0, 400.0, 272.0, 420.0), RichText::plain("Typed in"))
                .with_intent(FreeTextIntent::Typewriter)
                .with_font(FontRef::Standard(StandardFamily::Courier), 11.0),
        ),
    ]
}

mod would_violate {
    use super::*;

    fn verdicts(name: &str) -> (ConstraintKind, Vec<Vec<Restriction>>) {
        let doc = editable(&fixture(&format!("signed/{name}")));
        let planner = SavePlanner::new(&doc).unwrap();
        let restrictions = every_box(&doc)
            .iter()
            .map(|op| planner.would_violate(op).iter().map(|v| v.restriction).collect())
            .collect();
        (planner.constraint().kind(), restrictions)
    }

    fn none() -> (ConstraintKind, Vec<Vec<Restriction>>) {
        (ConstraintKind::AppendOnly, vec![Vec::new(); 3])
    }

    #[test]
    fn approval_signatures_and_a_certification_that_allows_annotations_allow_text_boxes() {
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
    fn a_certification_without_annotations_forbids_text_boxes() {
        let all = |p| (ConstraintKind::AppendOnly, vec![vec![Restriction::DocMdp(p)]; 3]);
        assert_eq!(
            [verdicts("certified-p1.pdf"), verdicts("certified-p2.pdf")],
            [all(DocMdpPermission::NoChanges), all(DocMdpPermission::FormFill)]
        );
    }
}

fn qpdf_accepts(bytes: &[u8]) -> bool {
    let dir = std::env::temp_dir().join(format!("lamina-freetext-{}", std::process::id()));
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
fn every_box_saves_in_both_modes_and_reopens_with_its_appearance() {
    for name in ["golden/examples/hello.pdf", "golden/tagged-report.pdf"] {
        for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
            let mut doc = editable(&fixture(name)).with_appearances(AnnotationAppearances);
            let ops = every_box(&doc);
            doc.apply_all(Origin::Local, ops).unwrap();
            let mut bytes = Vec::new();
            doc.save(&mut bytes, &SaveOptions::new().with_mode(mode)).unwrap();
            let reopened = laminapdf_model::Document::open(bytes.clone(), OpenOptions::default()).unwrap();
            let pages = reopened.pages().unwrap();
            let boxes: Vec<bool> = reopened
                .annotations(&pages.get(0).unwrap())
                .unwrap()
                .iter()
                .filter(|a| a.subtype.name() == b"FreeText")
                .map(|a| a.has_appearance)
                .collect();
            assert_eq!((boxes, qpdf_accepts(&bytes)), (vec![true; 3], true), "{name} {mode:?}");
        }
    }
}

/// The incremental save of `doc` as it is now.
fn saved(doc: &mut Document) -> Vec<u8> {
    let mut bytes = Vec::new();
    doc.save(&mut bytes, &SaveOptions::new().with_mode(SaveMode::Incremental))
        .unwrap();
    bytes
}

/// The fixture opened for editing, with every box applied when `apply` says so.
fn hello(apply: bool) -> Document {
    let mut doc = editable(&fixture("golden/examples/hello.pdf")).with_appearances(AnnotationAppearances);
    if apply {
        let ops = every_box(&doc);
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

/// The PDF/A-2b example with a text box in `font` on its first page.
fn pdfa_with_box(font: impl FnOnce(&mut Document) -> FontRef) -> Vec<String> {
    let mut doc = editable(&fixture("golden/examples/pdf_a_archive-2b.pdf")).with_appearances(AnnotationAppearances);
    let font = font(&mut doc);
    let page = doc.pages().unwrap().get(0).unwrap().id().clone();
    let spec = FreeText::new(rect(72.0, 600.0, 272.0, 640.0), RichText::plain("Archived")).with_font(font, 12.0);
    doc.apply_all(
        Origin::Local,
        [Op::from(AddAnnot::new(page, AnnotSpec::FreeText(spec)))],
    )
    .unwrap();
    SavePlanner::new(&doc)
        .unwrap()
        .conformance_warnings()
        .iter()
        .map(|w| w.code().to_owned())
        .collect()
}

#[test]
fn a_box_in_a_standard_font_on_a_pdfa_file_warns_and_one_in_an_embedded_font_doesn_t() {
    let standard = pdfa_with_box(|_| FontRef::Standard(StandardFamily::Helvetica));
    let embedded = pdfa_with_box(|doc| FontRef::Font(doc.add_blob(fixture("fonts/NotoSans-Regular.ttf"))));
    assert_eq!(
        (
            standard.contains(&"pdfa-font-not-embedded".to_owned()),
            embedded.contains(&"pdfa-font-not-embedded".to_owned())
        ),
        (true, false)
    );
}
