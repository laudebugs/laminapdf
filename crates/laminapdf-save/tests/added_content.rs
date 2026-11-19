//! Saving objects added to pages as content (E6 plan §5, todo E6b): each op's verdict on the
//! signed fixtures (class `Content`), every kind saved in both modes and found again after the
//! file is reopened, changed and deleted after the save, `qpdf --check` clean, undo and redo that
//! give back the same bytes, and the page's own content unchanged.
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
use laminapdf_model::{Matrix, PdfDate, Pt, RectPt};
use laminapdf_ops::{
    AddContentObject, Alt, AnnotColor, ContentId, ContentLayer, ContentObjectSpec, DeleteContentObject, Document, Op,
    Opacity, Origin, PathSpec, Stroke, UpdateContentObject, added_objects,
};
use laminapdf_save::{ConstraintKind, DocMdpPermission, Restriction, Save as _, SaveMode, SaveOptions, SavePlanner};

fn id(s: &str) -> ContentId {
    ContentId::new(s).unwrap()
}

fn date() -> PdfDate {
    "D:20261118120000Z".parse().unwrap()
}

fn at(x: f64, y: f64, w: f64, h: f64) -> Matrix {
    Matrix::new([w, 0.0, 0.0, h, x, y])
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> RectPt {
    RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
}

/// An image, a translucent shape behind the content and a page of another PDF on `doc`'s first
/// page, their bytes added to its session.
fn every_kind(doc: &mut Document) -> Vec<Op> {
    let page = doc.pages().unwrap().get(0).unwrap().id().clone();
    let photo = ContentObjectSpec::Image {
        blob: doc.add_blob(fixture("images/photo-rgb.jpg")),
        crop: None,
    };
    let logo = ContentObjectSpec::Form {
        blob: doc.add_blob(fixture("content/add/logo.pdf")),
        page: 0,
    };
    let band = ContentObjectSpec::Shape {
        path: PathSpec::Rect {
            rect: rect(36.0, 36.0, 300.0, 120.0),
            radius: 8.0,
        },
        fill: Some(AnnotColor::Rgb(1.0, 0.9, 0.6)),
        stroke: Some(Stroke::new(AnnotColor::Rgb(0.8, 0.5, 0.0), 1.5)),
    };
    let identity = Matrix::new([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);
    vec![
        AddContentObject::new(id("photo"), page.clone(), photo, at(320.0, 40.0, 144.0, 96.0), date())
            .with_alt(Alt::Text("The office".into()))
            .into(),
        AddContentObject::new(id("band"), page.clone(), band, identity, date())
            .with_layer(ContentLayer::Behind)
            .with_opacity(Opacity::new(0.6).unwrap())
            .with_alt(Alt::Decorative)
            .into(),
        AddContentObject::new(id("logo"), page, logo, at(40.0, 160.0, 120.0, 60.0), date())
            .with_alt(Alt::Text("Northwind".into()))
            .into(),
    ]
}

mod would_violate {
    use super::*;

    fn verdicts(name: &str) -> (ConstraintKind, Vec<Vec<Restriction>>) {
        let mut doc = editable(&fixture(&format!("signed/{name}")));
        let mut ops = every_kind(&mut doc);
        let page = doc.pages().unwrap().get(0).unwrap().id().clone();
        ops.push(UpdateContentObject::new(id("photo"), page.clone(), date()).into());
        ops.push(DeleteContentObject::new(id("photo"), page).into());
        let planner = SavePlanner::new(&doc).unwrap();
        let restrictions = ops
            .iter()
            .map(|op| planner.would_violate(op).iter().map(|v| v.restriction).collect())
            .collect();
        (planner.constraint().kind(), restrictions)
    }

    #[test]
    fn approval_signatures_and_a_field_lock_allow_added_content() {
        let none = (ConstraintKind::AppendOnly, vec![Vec::new(); 5]);
        assert_eq!(
            [
                verdicts("approval.pdf"),
                verdicts("two-signatures.pdf"),
                verdicts("locked.pdf")
            ],
            [none.clone(), none.clone(), none]
        );
    }

    #[test]
    fn every_certification_level_forbids_it() {
        let all = |p| (ConstraintKind::AppendOnly, vec![vec![Restriction::DocMdp(p)]; 5]);
        assert_eq!(
            [
                verdicts("certified-p1.pdf"),
                verdicts("certified-p2.pdf"),
                verdicts("certified-p3.pdf")
            ],
            [
                all(DocMdpPermission::NoChanges),
                all(DocMdpPermission::FormFill),
                all(DocMdpPermission::Annotate)
            ]
        );
    }
}

fn qpdf_accepts(bytes: &[u8]) -> bool {
    let dir = std::env::temp_dir().join(format!("lamina-added-content-{}", std::process::id()));
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

fn save(doc: &mut Document, mode: SaveMode) -> Vec<u8> {
    let mut bytes = Vec::new();
    doc.save(&mut bytes, &SaveOptions::new().with_mode(mode)).unwrap();
    bytes
}

fn listed(doc: &Document) -> Vec<(String, ContentLayer)> {
    let page = doc.pages().unwrap().get(0).unwrap().id().clone();
    added_objects(doc, &page)
        .unwrap()
        .into_iter()
        .map(|o| (o.id.to_string(), o.layer))
        .collect()
}

#[test]
fn every_kind_saves_in_both_modes_and_is_found_again_after_reopening() {
    for name in ["golden/examples/hello.pdf", "golden/tagged-report.pdf"] {
        for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
            let mut doc = editable(&fixture(name)).with_appearances(AnnotationAppearances);
            let ops = every_kind(&mut doc);
            doc.apply_all(Origin::Local, ops).unwrap();
            let bytes = save(&mut doc, mode);
            let reopened = editable(&bytes).with_appearances(AnnotationAppearances);
            assert_eq!(
                (listed(&reopened), qpdf_accepts(&bytes)),
                (
                    vec![
                        ("band".to_owned(), ContentLayer::Behind),
                        ("photo".to_owned(), ContentLayer::Front),
                        ("logo".to_owned(), ContentLayer::Front)
                    ],
                    true
                ),
                "{name} {mode:?}"
            );
        }
    }
}

#[test]
fn an_object_added_before_a_save_is_changed_and_deleted_after_it() {
    let mut doc = editable(&fixture("golden/examples/hello.pdf")).with_appearances(AnnotationAppearances);
    let ops = every_kind(&mut doc);
    doc.apply_all(Origin::Local, ops).unwrap();
    let bytes = save(&mut doc, SaveMode::Incremental);
    // A new session holds none of the bytes the objects were made from: a move, a fade, a crop
    // and a restack need none.
    let mut doc = editable(&bytes).with_appearances(AnnotationAppearances);
    let page = doc.pages().unwrap().get(0).unwrap().id().clone();
    let change = UpdateContentObject::new(id("photo"), page.clone(), date())
        .with_matrix(at(300.0, 300.0, 72.0, 48.0))
        .with_opacity(Opacity::new(0.5).unwrap())
        .with_crop(Some(rect(0.0, 0.0, 0.5, 1.0)))
        .with_order(-1);
    doc.apply_all(
        Origin::Local,
        [change.into(), DeleteContentObject::new(id("logo"), page.clone()).into()],
    )
    .unwrap();
    let bytes = save(&mut doc, SaveMode::Incremental);
    let reopened = editable(&bytes);
    let objects = added_objects(&reopened, &page).unwrap();
    assert_eq!(
        listed(&reopened),
        [
            ("band".to_owned(), ContentLayer::Behind),
            ("photo".to_owned(), ContentLayer::Front)
        ]
    );
    assert_eq!(
        (objects[1].bounds, objects[1].opacity, qpdf_accepts(&bytes)),
        (rect(300.0, 300.0, 336.0, 348.0), Opacity::new(0.5).unwrap(), true)
    );
    // Drawing it again needs its bytes, which the session doesn't hold.
    let mut doc = editable(&bytes).with_appearances(AnnotationAppearances);
    let blob = laminapdf_ops::BlobId::of(b"no such bytes");
    let replace = UpdateContentObject::new(id("photo"), page, date()).with_blob(blob);
    assert_eq!(
        doc.apply_all(Origin::Local, [replace.into()]).unwrap_err().code(),
        "unknown-blob"
    );
}

/// The fixture opened for editing, with every kind applied when `apply` says so.
fn hello(apply: bool) -> Document {
    let mut doc = editable(&fixture("golden/examples/hello.pdf")).with_appearances(AnnotationAppearances);
    let ops = every_kind(&mut doc);
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
    assert_eq!(
        (empty, save(&mut undone, SaveMode::Incremental)),
        (true, save(&mut hello(false), SaveMode::Incremental))
    );
}

#[test]
fn redo_writes_the_same_bytes_as_the_edit() {
    let mut redone = hello(true);
    redone.undo().unwrap();
    redone.redo().unwrap();
    assert_eq!(
        save(&mut redone, SaveMode::Incremental),
        save(&mut hello(true), SaveMode::Incremental)
    );
}

#[test]
fn the_pages_own_content_streams_keep_their_bytes() {
    let original = editable(&fixture("golden/examples/hello.pdf"));
    let page = |doc: &Document| doc.pages().unwrap().get(0).unwrap().cos().clone();
    let own = original.cos().page_content(&page(&original)).unwrap();
    let mut doc = hello(true);
    let bytes = save(&mut doc, SaveMode::FullRewrite);
    let reopened = editable(&bytes);
    let content = String::from_utf8_lossy(&reopened.cos().page_content(&page(&reopened)).unwrap()).into_owned();
    let own = String::from_utf8_lossy(&own).into_owned();
    assert!(content.contains(own.trim()), "{content}");
}
