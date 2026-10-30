//! The overlay seen through the public API (ADR 0005 §3): writes over corrupt base objects,
//! generations, enumeration and the incremental writer.
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] fns: a panic here is a test failure"
)]

mod common;

use common::{classic, open, open_with};
use lamina_cos::{Document, ObjRef, Object, OpenOptions, Warning};
use lamina_save::{XrefEol, incremental_update};

/// Catalog 1, pages 2, and object 3 that does not parse.
fn with_corrupt_object() -> Document {
    open(&classic(
        &[
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [] /Count 0 >>",
            b"<< /A [ >>",
        ],
        "<< /Size 4 /Root 1 0 R >>",
    ))
    .unwrap()
}

fn simple() -> Vec<u8> {
    classic(
        &[
            b"<< /Type /Catalog /Pages 2 0 R >>",
            b"<< /Type /Pages /Kids [] /Count 0 >>",
            b"(three)",
        ],
        "<< /Size 4 /Root 1 0 R >>",
    )
}

const CORRUPT: ObjRef = ObjRef::new(3, 0);

#[test]
fn a_corrupt_base_object_can_be_overwritten() {
    let mut doc = with_corrupt_object();
    doc.edit().set(CORRUPT, Object::Integer(1)).unwrap();
    assert_eq!(doc.resolve(CORRUPT).unwrap().as_int(), Some(1));
}

#[test]
fn a_corrupt_base_object_can_be_deleted() {
    let mut doc = with_corrupt_object();
    doc.edit().delete(CORRUPT).unwrap();
    assert_eq!(*doc.resolve(CORRUPT).unwrap(), Object::Null);
}

#[test]
fn incremental_update_ignores_the_overlay_trailer() {
    let mut doc = open(&simple()).unwrap();
    let mut edit = doc.edit();
    let info = edit.create(Object::Integer(1)).unwrap();
    let mut trailer = edit.trailer().clone();
    trailer.insert("Info", info);
    edit.set_trailer(trailer);
    let out = incremental_update(&doc, &[], XrefEol::CrLf).unwrap();
    let reopened = open(&out).unwrap();
    assert!(reopened.trailer().get("Info").is_none());
}

#[test]
fn object_numbers_include_created_and_exclude_deleted_objects() {
    let mut doc = open(&simple()).unwrap();
    let created = doc.edit().create(Object::Null).unwrap();
    doc.edit().delete(ObjRef::new(3, 0)).unwrap();
    assert_eq!(doc.object_numbers().collect::<Vec<_>>(), [1, 2, created.num]);
}

#[test]
fn a_created_object_named_with_another_generation_warns_stale_generation() {
    let mut doc = open(&simple()).unwrap();
    let created = doc.edit().create(Object::Integer(1)).unwrap();
    doc.resolve(ObjRef::new(created.num, 1)).unwrap();
    assert!(
        doc.warnings()
            .iter()
            .any(|w| matches!(w, Warning::StaleGeneration { .. }))
    );
}

#[test]
fn a_created_object_named_with_another_generation_is_refused_in_strict_mode() {
    let mut doc = open_with(&simple(), OpenOptions::default().with_strict(true)).unwrap();
    let created = doc.edit().create(Object::Integer(1)).unwrap();
    assert!(doc.resolve(ObjRef::new(created.num, 1)).is_err());
}

#[test]
fn len_counts_a_changed_trailer_like_is_empty_does() {
    let mut doc = open(&simple()).unwrap();
    let mut trailer = doc.trailer().clone();
    trailer.insert("Extra", 1);
    doc.edit().set_trailer(trailer);
    assert_eq!((doc.overlay().is_empty(), doc.overlay().len()), (false, 1));
}

mod empty_document {
    use super::*;

    #[test]
    fn has_no_base_objects_and_numbers_new_ones_from_one() {
        let mut doc = Document::empty(Some("1.7"), &OpenOptions::default());
        let first = doc.edit().create(Object::Null).unwrap();
        assert_eq!(
            (doc.source().is_empty(), doc.header_version(), first),
            (true, Some("1.7"), ObjRef::new(1, 0))
        );
    }

    #[test]
    fn reads_the_catalog_the_overlay_sets() {
        let mut doc = Document::empty(None, &OpenOptions::default());
        let mut cos = doc.edit();
        let pages = cos
            .create(Object::Dict(
                [
                    ("Type", Object::name("Pages")),
                    ("Kids", Object::Array(vec![])),
                    ("Count", 0.into()),
                ]
                .into_iter()
                .collect(),
            ))
            .unwrap();
        let root = cos
            .create(Object::Dict(
                [("Type", Object::name("Catalog")), ("Pages", pages.into())]
                    .into_iter()
                    .collect(),
            ))
            .unwrap();
        cos.set_trailer([("Root", Object::from(root))].into_iter().collect());
        assert_eq!(doc.pages().unwrap().len(), 0);
    }
}
