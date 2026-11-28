//! `RemoveXfa` over `fixtures/forms/xfa-static.pdf` and `xfa-dynamic.pdf` (written by
//! `tools/fixtures/forms.py`): what it removes, what it refuses, undo and redo, and the wire form.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::{open, snapshot};
use laminapdf_ops::{Document, ModificationClass, Op, Origin, RemoveXfa};
use laminapdf_test_fixtures::fixture;

fn form(name: &str) -> Document {
    open(&fixture(name))
}

fn remove(doc: &mut Document) -> Result<laminapdf_ops::Applied, laminapdf_ops::Error> {
    doc.apply_all(Origin::Local, [RemoveXfa::new().into()])
}

fn packets(doc: &Document) -> Vec<(String, Vec<u8>)> {
    doc.xfa()
        .unwrap()
        .map(|x| x.packets.into_iter().map(|p| (p.name, p.bytes)).collect())
        .unwrap_or_default()
}

#[test]
fn a_static_form_loses_its_xfa_and_keeps_its_fields() {
    let mut doc = form("forms/xfa-static.pdf");
    let fields = doc.form().unwrap().fields.len();
    let applied = remove(&mut doc).unwrap();
    let after = doc.form().unwrap();
    assert_eq!(
        (
            doc.xfa().unwrap(),
            after.has_xfa,
            after.fields.len(),
            applied.xfa_removed
        ),
        (None, false, fields, false)
    );
    assert!(after.field_named("name").unwrap().value.is_some());
}

#[test]
fn a_dynamic_form_loses_its_needs_rendering_flag_too() {
    let mut doc = form("forms/xfa-dynamic.pdf");
    assert!(doc.catalog().unwrap().contains_key("NeedsRendering"));
    remove(&mut doc).unwrap();
    assert_eq!(
        (
            doc.xfa().unwrap(),
            doc.catalog().unwrap().contains_key("NeedsRendering")
        ),
        (None, false)
    );
}

#[test]
fn a_form_without_xfa_is_refused() {
    let mut doc = form("forms/fields.pdf");
    let err = remove(&mut doc).unwrap_err();
    assert_eq!((err.code(), doc.overlay().is_empty()), ("invalid-op", true));
}

#[test]
fn removing_it_twice_is_refused_the_second_time() {
    let mut doc = form("forms/xfa-static.pdf");
    remove(&mut doc).unwrap();
    assert_eq!(remove(&mut doc).unwrap_err().code(), "invalid-op");
}

#[test]
fn undo_restores_the_packets_byte_for_byte_and_redo_removes_them_again() {
    for name in ["forms/xfa-static.pdf", "forms/xfa-dynamic.pdf"] {
        let mut doc = form(name);
        let (before, held) = (snapshot(&doc), packets(&doc));
        remove(&mut doc).unwrap();
        let after = snapshot(&doc);
        doc.undo().unwrap();
        let undone = (snapshot(&doc), doc.overlay().is_empty(), packets(&doc));
        doc.redo().unwrap();
        assert_eq!(
            (undone, snapshot(&doc), packets(&doc).is_empty()),
            ((before, true, held), after, true),
            "{name}"
        );
    }
}

#[test]
fn it_is_a_document_change_that_needs_no_rewrite() {
    let op = Op::from(RemoveXfa::new());
    assert_eq!(
        (op.class(), op.requires_rewrite()),
        (ModificationClass::Document, false)
    );
}

#[test]
fn the_wire_form_has_no_arguments() {
    let json = r#"{"op":"removeXfa","args":{}}"#;
    let op: Op = serde_json::from_str(json).unwrap();
    assert_eq!(serde_json::to_string(&op).unwrap(), json);
    assert!(serde_json::from_str::<Op>(r#"{"op":"removeXfa","args":{"all":true}}"#).is_err());
}

#[test]
fn the_frozen_sample_applies() {
    let mut doc = form("forms/xfa-static.pdf");
    let op: Op = serde_json::from_slice(&fixture("ops/v1/remove-xfa.json")).unwrap();
    doc.apply_all(Origin::Local, [op]).unwrap();
    assert!(!doc.form().unwrap().has_xfa);
}
