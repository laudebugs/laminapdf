//! `FlattenForm` over `fixtures/forms/fields.pdf` and `xfa-static.pdf` (written by
//! `tools/fixtures/forms.py`): what it draws and removes, the form tree it edits, what it refuses
//! and leaves, the XFA it removes, undo and redo, and the wire form.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::{open, snapshot};
use laminapdf_build::AnnotationAppearances;
use laminapdf_cos::{ObjRef, Object};
use laminapdf_model::FieldId;
use laminapdf_ops::{Applied, Document, Error, FlattenForm, FlattenIntent, ModificationClass, Op, Origin};
use laminapdf_test_fixtures::fixture;

fn form(name: &str) -> Document {
    open(&fixture(name)).with_appearances(AnnotationAppearances)
}

fn id(doc: &Document, name: &str) -> FieldId {
    doc.form().unwrap().field_named(name).unwrap().id.clone()
}

fn flatten(doc: &mut Document, names: &[&str]) -> Result<Applied, Error> {
    let fields = names.iter().map(|n| id(doc, n)).collect();
    doc.apply_all(Origin::Local, [FlattenForm::new(fields, FlattenIntent::View).into()])
}

/// The object numbers of the annotations page `index` lists.
fn listed(doc: &Document, index: usize) -> Vec<u32> {
    let pages = doc.pages().unwrap();
    let page = pages.iter().nth(index).unwrap();
    doc.annotation_links(&page)
        .unwrap()
        .iter()
        .filter_map(|a| a.id.as_lamina_id().obj_ref())
        .map(|r| r.num)
        .collect()
}

/// Page `index`'s content, every stream joined.
fn content(doc: &Document, index: usize) -> String {
    let pages = doc.pages().unwrap();
    let page = pages.iter().nth(index).unwrap();
    String::from_utf8_lossy(&doc.cos().page_content(page.cos()).unwrap()).into_owned()
}

/// The names of the form's terminal fields.
fn names(doc: &Document) -> Vec<String> {
    doc.form().unwrap().fields.iter().map(|f| f.name.clone()).collect()
}

/// The form dictionary.
fn acro_form(doc: &Document) -> laminapdf_cos::Dict {
    let catalog = doc.catalog().unwrap();
    let form = catalog.get("AcroForm").cloned().unwrap();
    doc.deref(&form).unwrap().as_dict().cloned().unwrap()
}

/// The object numbers `/AcroForm /Fields` lists.
fn top_level(doc: &Document) -> Vec<u32> {
    let fields = acro_form(doc).get("Fields").cloned().unwrap();
    doc.deref(&fields)
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Object::as_ref)
        .map(|r| r.num)
        .collect()
}

/// Whether object `num` is still a dictionary (a deleted one reads as null).
fn exists(doc: &Document, num: u32) -> bool {
    laminapdf_cos::tolerant(doc.resolve(ObjRef::new(num, 0)))
        .unwrap()
        .is_some_and(|o| o.as_dict().is_some())
}

#[test]
fn a_text_field_is_drawn_into_its_page_and_leaves_the_form() {
    let mut doc = form("forms/fields.pdf");
    let before = content(&doc, 0).matches(" Do").count();
    flatten(&mut doc, &["name"]).unwrap();
    assert_eq!(
        (
            content(&doc, 0).matches(" Do").count() - before,
            listed(&doc, 0).contains(&10),
            names(&doc).contains(&"name".to_owned()),
            top_level(&doc).contains(&10),
            exists(&doc, 10),
        ),
        (1, false, false, false, false)
    );
}

#[test]
fn a_parent_left_without_kids_goes_too() {
    let mut doc = form("forms/fields.pdf");
    flatten(&mut doc, &["person.first", "person.last"]).unwrap();
    assert_eq!((top_level(&doc).contains(&24), exists(&doc, 24)), (false, false));
}

#[test]
fn a_parent_keeps_the_kids_left() {
    let mut doc = form("forms/fields.pdf");
    flatten(&mut doc, &["person.first"]).unwrap();
    assert_eq!(
        (names(&doc).contains(&"person.last".to_owned()), top_level(&doc).contains(&24)),
        (true, true)
    );
}

#[test]
fn a_radio_group_draws_each_widget_in_its_state() {
    let mut doc = form("forms/fields.pdf");
    let before = content(&doc, 0).matches(" Do").count();
    flatten(&mut doc, &["colour"]).unwrap();
    let gone: Vec<bool> = [14, 15, 16].iter().map(|n| listed(&doc, 0).contains(n)).collect();
    assert_eq!(
        (content(&doc, 0).matches(" Do").count() - before, gone),
        (3, vec![false, false, false])
    );
}

#[test]
fn a_signature_field_is_refused_and_nothing_changes() {
    let mut doc = form("forms/fields.pdf");
    let before = snapshot(&doc);
    let err = flatten(&mut doc, &["name", "signature"]).unwrap_err();
    assert_eq!((err.code(), snapshot(&doc) == before), ("field-is-signature", true));
}

#[test]
fn an_unknown_field_is_refused() {
    let mut doc = form("forms/fields.pdf");
    let op = FlattenForm::new(vec!["f99.0".parse().unwrap()], FlattenIntent::View);
    let err = doc.apply_all(Origin::Local, [op.into()]).unwrap_err();
    assert_eq!(err.code(), "unknown-field");
}

#[test]
fn a_push_button_without_an_appearance_stays_and_is_reported() {
    let mut doc = form("forms/fields.pdf");
    let applied = flatten(&mut doc, &["submit", "name"]).unwrap();
    assert_eq!(
        (
            applied.not_flattened.iter().map(ToString::to_string).collect::<Vec<_>>(),
            names(&doc).contains(&"submit".to_owned()),
            names(&doc).contains(&"name".to_owned()),
        ),
        (vec!["a22.0".to_owned()], true, false)
    );
}

#[test]
fn without_a_generator_a_field_without_an_appearance_is_refused() {
    let mut doc = open(&fixture("forms/fields.pdf"));
    let err = flatten(&mut doc, &["name"]).unwrap_err();
    assert_eq!(err.code(), "appearance-unavailable");
}

#[test]
fn with_no_field_left_the_form_loses_its_xfa() {
    let mut doc = form("forms/xfa-static.pdf");
    let all: Vec<String> = names(&doc);
    let all: Vec<&str> = all.iter().map(String::as_str).collect();
    flatten(&mut doc, &all).unwrap();
    assert_eq!(
        (acro_form(&doc).get("XFA").is_some(), names(&doc)),
        (false, Vec::<String>::new())
    );
}

#[test]
fn it_is_a_content_change_that_stays_in_earlier_revisions() {
    let mut doc = form("forms/fields.pdf");
    let op: Op = FlattenForm::new(vec![id(&doc, "name")], FlattenIntent::View).into();
    let applied = doc.apply_all(Origin::Local, [op.clone()]).unwrap();
    assert_eq!(
        (op.class(), op.form_fields().len(), applied.still_in_earlier_revisions),
        (ModificationClass::Content, 1, true)
    );
}

#[test]
fn undo_leaves_the_overlay_empty_and_redo_writes_the_same_again() {
    let mut doc = form("forms/fields.pdf");
    let before = snapshot(&doc);
    flatten(&mut doc, &["name", "agree", "colour", "person.first", "person.last", "total"]).unwrap();
    let after = snapshot(&doc);
    doc.undo().unwrap();
    let undone = (snapshot(&doc), doc.overlay().is_empty());
    doc.redo().unwrap();
    assert_eq!((undone, snapshot(&doc)), ((before, true), after));
}

#[test]
fn the_wire_form_reads_back() {
    let op: Op = serde_json::from_str(r#"{ "op": "flattenForm", "args": { "fields": ["f10.0"], "intent": "print" } }"#)
        .unwrap();
    let back: Op = serde_json::from_str(&serde_json::to_string(&op).unwrap()).unwrap();
    assert_eq!(
        (op.clone(), back),
        (FlattenForm::new(vec!["f10.0".parse().unwrap()], FlattenIntent::Print).into(), op)
    );
}

#[test]
fn the_wire_form_refuses_unknown_arguments() {
    let err = serde_json::from_str::<Op>(r#"{ "op": "flattenForm", "args": { "fields": [], "intent": "view", "all": true } }"#)
        .unwrap_err();
    assert!(err.to_string().contains("all"), "{err}");
}
