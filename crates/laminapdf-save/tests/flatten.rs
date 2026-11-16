//! Saving `FlattenAnnots` (todo E5e, E5 plan §5): its verdict on the signed fixtures (class
//! `Content`: approval signatures and a `FieldMDP` lock allow it, as it touches no field; every
//! certification forbids it, since none of P1–P3 allows changing what a page draws, ISO 32000-2
//! §12.8.2.2; usage rights grant no content change), and `flatten.pdf` flattened, saved in each
//! mode, reopened without the annotations, `qpdf --check` clean, undone and redone byte for byte.
#![allow(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use std::process::Command;

use common::fixture;
use common::save::editable;
use laminapdf_build::AnnotationAppearances;
use laminapdf_cos::OpenOptions;
use laminapdf_model::AnnotId;
use laminapdf_ops::{FlattenAnnots, FlattenIntent, Op, Origin};
use laminapdf_save::{ConstraintKind, DocMdpPermission, Restriction, Save as _, SaveMode, SaveOptions, SavePlanner};

/// The op flattens annotation 10 of any file: the planner judges an op before it applies, so the
/// annotation need not exist.
fn op() -> Op {
    FlattenAnnots::new(vec!["a10.0".parse::<AnnotId>().unwrap()], FlattenIntent::View).into()
}

fn verdict(name: &str) -> (ConstraintKind, Vec<Restriction>) {
    let doc = editable(&fixture(&format!("signed/{name}")));
    let planner = SavePlanner::new(&doc).unwrap();
    let restrictions = planner.would_violate(&op()).iter().map(|v| v.restriction).collect();
    (planner.constraint().kind(), restrictions)
}

#[test]
fn approval_signatures_and_a_lock_allow_flattening() {
    for name in ["approval.pdf", "two-signatures.pdf", "locked.pdf"] {
        assert_eq!(verdict(name), (ConstraintKind::AppendOnly, Vec::new()), "{name}");
    }
}

#[test]
fn every_certification_forbids_flattening() {
    for (name, permission) in [
        ("certified-p1.pdf", DocMdpPermission::NoChanges),
        ("certified-p2.pdf", DocMdpPermission::FormFill),
        ("certified-p3.pdf", DocMdpPermission::Annotate),
    ] {
        assert_eq!(
            verdict(name),
            (ConstraintKind::AppendOnly, vec![Restriction::DocMdp(permission)]),
            "{name}"
        );
    }
}

#[test]
fn usage_rights_forbid_flattening() {
    assert_eq!(verdict("usage-rights.pdf").1, vec![Restriction::UsageRights]);
}

#[test]
fn an_unsigned_file_has_nothing_to_forbid() {
    let doc = editable(&fixture("annotations/flatten.pdf"));
    let planner = SavePlanner::new(&doc).unwrap();
    assert_eq!(
        (planner.constraint().kind(), planner.would_violate(&op()).len()),
        (ConstraintKind::Free, 0)
    );
}

/// Every annotation of `flatten.pdf` but the widget, on both pages.
fn flatten_all() -> Op {
    let ids = [
        "a10.0", "a11.0", "a12.0", "a13.0", "a14.0", "a15.0", "a16.0", "a17.0", "a19.0", "a22.0", "a60.0",
    ];
    FlattenAnnots::new(ids.iter().map(|i| i.parse().unwrap()).collect(), FlattenIntent::View).into()
}

fn qpdf_accepts(bytes: &[u8]) -> bool {
    let dir = std::env::temp_dir().join(format!("lamina-flatten-{}", std::process::id()));
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

/// The annotations each page lists, by object number.
fn listed(doc: &laminapdf_model::Document) -> Vec<Vec<u32>> {
    doc.pages()
        .unwrap()
        .iter()
        .map(|page| {
            doc.annotation_links(&page)
                .unwrap()
                .iter()
                .filter_map(|a| a.id.as_lamina_id().obj_ref())
                .map(|r| r.num)
                .collect()
        })
        .collect()
}

#[test]
fn a_flattened_file_saves_in_both_modes_and_reopens_without_the_annotations() {
    for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
        let mut doc = editable(&fixture("annotations/flatten.pdf")).with_appearances(AnnotationAppearances);
        doc.apply_all(Origin::Local, [flatten_all()]).unwrap();
        let mut bytes = Vec::new();
        doc.save(&mut bytes, &SaveOptions::new().with_mode(mode)).unwrap();
        let reopened = laminapdf_model::Document::open(bytes.clone(), OpenOptions::default()).unwrap();
        // Left: hidden, print-only, in a layer that is off, an attachment with no appearance, the
        // widget.
        assert_eq!(
            (listed(&reopened), reopened.was_repaired(), qpdf_accepts(&bytes)),
            (vec![vec![12, 13, 15, 17, 18], Vec::new()], false, true),
            "{mode:?}"
        );
    }
}

#[test]
fn undoing_writes_the_base_and_redoing_writes_the_same_bytes_again() {
    let incremental = SaveOptions::new().with_mode(SaveMode::Incremental);
    let open = || editable(&fixture("annotations/flatten.pdf")).with_appearances(AnnotationAppearances);
    let mut once = open();
    once.apply_all(Origin::Local, [flatten_all()]).unwrap();
    let mut applied = Vec::new();
    once.save(&mut applied, &incremental).unwrap();

    let mut doc = open();
    let base = doc.data().unwrap().to_vec();
    doc.apply_all(Origin::Local, [flatten_all()]).unwrap();
    doc.undo().unwrap();
    let empty = doc.overlay().is_empty();
    let mut undone = Vec::new();
    doc.save(&mut undone, &incremental).unwrap();
    doc.redo().unwrap();
    let mut redone = Vec::new();
    doc.save(&mut redone, &incremental).unwrap();
    assert_eq!((empty, undone == base, redone == applied), (true, true, true));
}
