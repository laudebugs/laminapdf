//! Saving the drawing marks (E4 plan §6, todo E4d): `AddAnnot`'s verdict for ink, a square, a
//! line and a polyline on the signed fixtures (class `Annotation`, as for text markup), each saved
//! in both modes and reopened with its appearance, `qpdf --check` clean, and a pressure stroke
//! that keeps its widths through a colour change.
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
use laminapdf_cos::{Object, OpenOptions};
use laminapdf_model::{UserSpacePoint, UserSpaceRect};
use laminapdf_ops::{
    AddAnnot, AnnotColor, AnnotSpec, Border, Document, Ink, InkStroke, LineEnding, LineMark, Op, Origin, Poly,
    PolyKind, Shape, ShapeKind, UpdateAnnot,
};
use laminapdf_save::{ConstraintKind, DocMdpPermission, Restriction, Save as _, SaveMode, SaveOptions, SavePlanner};

fn p(x: f64, y: f64) -> UserSpacePoint {
    UserSpacePoint::new(x, y)
}

/// Ink, a cloudy square, an arrow and a polyline on `doc`'s first page.
fn every_drawing(doc: &Document) -> Vec<Op> {
    let page = doc.pages().unwrap().get(0).unwrap().id().clone();
    let add = |spec| Op::from(AddAnnot::new(page.clone(), spec));
    vec![
        add(AnnotSpec::Ink(Ink::new(vec![InkStroke::new(vec![
            p(72.0, 600.0),
            p(90.0, 620.0),
            p(110.0, 600.0),
        ])]))),
        add(AnnotSpec::Shape(
            Shape::new(
                ShapeKind::Square,
                UserSpaceRect::from_corners(200.0, 580.0, 280.0, 630.0),
            )
            .with_border(Border::cloudy(1.0, 1.0)),
        )),
        add(AnnotSpec::Line(
            LineMark::new(p(72.0, 540.0), p(250.0, 520.0)).with_ends(LineEnding::None, LineEnding::ClosedArrow),
        )),
        add(AnnotSpec::Poly(Poly::new(
            PolyKind::PolyLine,
            vec![p(300.0, 520.0), p(340.0, 560.0), p(380.0, 520.0)],
        ))),
    ]
}

mod would_violate {
    use super::*;

    fn verdicts(name: &str) -> (ConstraintKind, Vec<Vec<Restriction>>) {
        let doc = editable(&fixture(&format!("signed/{name}")));
        let planner = SavePlanner::new(&doc).unwrap();
        let restrictions = every_drawing(&doc)
            .iter()
            .map(|op| planner.would_violate(op).iter().map(|v| v.restriction).collect())
            .collect();
        (planner.constraint().kind(), restrictions)
    }

    fn none() -> (ConstraintKind, Vec<Vec<Restriction>>) {
        (ConstraintKind::AppendOnly, vec![Vec::new(); 4])
    }

    #[test]
    fn approval_signatures_and_a_certification_that_allows_annotations_allow_drawing() {
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
    fn a_certification_without_annotations_forbids_drawing() {
        let all = |p| (ConstraintKind::AppendOnly, vec![vec![Restriction::DocMdp(p)]; 4]);
        assert_eq!(
            [verdicts("certified-p1.pdf"), verdicts("certified-p2.pdf")],
            [all(DocMdpPermission::NoChanges), all(DocMdpPermission::FormFill)]
        );
    }
}

fn qpdf_accepts(bytes: &[u8]) -> bool {
    let dir = std::env::temp_dir().join(format!("lamina-drawing-{}", std::process::id()));
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
fn every_drawing_saves_in_both_modes_and_reopens_with_its_appearance() {
    for name in ["golden/examples/hello.pdf", "golden/tagged-report.pdf"] {
        for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
            let mut doc = editable(&fixture(name)).with_appearances(AnnotationAppearances);
            let ops = every_drawing(&doc);
            doc.apply_all(Origin::Local, ops).unwrap();
            let mut bytes = Vec::new();
            doc.save(&mut bytes, &SaveOptions::new().with_mode(mode)).unwrap();
            let reopened = laminapdf_model::Document::open(bytes.clone(), OpenOptions::default()).unwrap();
            let pages = reopened.pages().unwrap();
            let annots = reopened.annotations(&pages.get(0).unwrap()).unwrap();
            let drawn: Vec<(&[u8], bool)> = annots
                .iter()
                .map(|a| (a.subtype.name(), a.has_appearance))
                .filter(|(s, _)| [b"Ink".as_slice(), b"Square", b"Line", b"PolyLine"].contains(s))
                .collect();
            assert_eq!(
                (drawn, qpdf_accepts(&bytes)),
                (
                    vec![
                        (b"Ink".as_slice(), true),
                        (b"Square", true),
                        (b"Line", true),
                        (b"PolyLine", true)
                    ],
                    true
                ),
                "{name} {mode:?}"
            );
        }
    }
}

#[test]
fn a_pressure_stroke_keeps_its_widths_through_a_colour_change() {
    let mut doc = editable(&fixture("golden/examples/hello.pdf")).with_appearances(AnnotationAppearances);
    let page = doc.pages().unwrap().get(0).unwrap().id().clone();
    let stroke =
        InkStroke::new(vec![p(72.0, 600.0), p(100.0, 620.0), p(130.0, 600.0)]).with_widths(vec![1.0, 6.0, 1.0]);
    let add = AddAnnot::new(page, AnnotSpec::Ink(Ink::new(vec![stroke])));
    let id = doc
        .apply_all(Origin::Local, [Op::from(add)])
        .unwrap()
        .created_annotations[0]
        .clone();
    let widths = |doc: &Document| {
        let pages = doc.pages().unwrap();
        let annot = doc
            .annotations(&pages.get(0).unwrap())
            .unwrap()
            .into_iter()
            .find(|a| a.id == id)
            .unwrap();
        let dict = doc.resolve(annot.id.as_lamina_id().obj_ref().unwrap()).unwrap();
        dict.as_dict().unwrap().get("LMNA_Widths").cloned()
    };
    let before = widths(&doc);
    let mut recolour = UpdateAnnot::new(id.clone());
    recolour.color = Some(Some(AnnotColor::Rgb(0.0, 0.0, 1.0)));
    doc.apply_all(Origin::Local, [Op::from(recolour)]).unwrap();
    // One stroke of three widths, the same after the redraw as before it.
    let count = |w: Option<Object>| w.and_then(|w| w.as_array()?.first()?.as_array().map(<[Object]>::len));
    assert_eq!((count(widths(&doc)), widths(&doc) == before), (Some(3), true));
}
