//! Form behaviour through the façade (todo E8f): a value checked and shown as its field would,
//! calculations after a change and on request, a signature's lock keeping its total, and the
//! inventory of the document's scripts.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf::{Document, FieldId, FieldValue, InvalidValue, Op, OpenOptions, ScriptState, SetFieldValue};
use laminapdf_test_fixtures::fixture;

fn open(name: &str) -> Document {
    Document::open_bytes(fixture(name), OpenOptions::new()).unwrap()
}

fn id(doc: &Document, name: &str) -> FieldId {
    doc.form()
        .unwrap()
        .fields
        .into_iter()
        .find(|f| f.name == name)
        .unwrap()
        .id
        .parse()
        .unwrap()
}

fn value(doc: &Document, name: &str) -> Option<FieldValue> {
    doc.form()
        .unwrap()
        .fields
        .into_iter()
        .find(|f| f.name == name)
        .unwrap()
        .value
}

fn text(s: &str) -> FieldValue {
    FieldValue::Text(s.to_owned())
}

#[test]
fn validate_shows_a_value_as_its_field_would_and_names_a_refusal() {
    let doc = open("forms/scripts.pdf");
    let price = id(&doc, "price");
    let ok = doc.validate(&price, "99.999").unwrap();
    let refused = doc.validate(&price, "20000").unwrap();
    assert_eq!(
        (
            ok.committed.as_deref().ok(),
            ok.shown.text.as_str(),
            matches!(refused.committed, Err(InvalidValue::OutOfRange(_)))
        ),
        (Some("99.999"), "$100.00", true)
    );
}

#[test]
fn field_accepts_takes_a_number_while_it_is_typed() {
    let doc = open("forms/scripts.pdf");
    let price = id(&doc, "price");
    assert_eq!(
        [
            doc.field_accepts(&price, "12.").unwrap(),
            doc.field_accepts(&price, "12a").unwrap()
        ],
        [true, false]
    );
}

#[test]
fn a_change_through_the_facade_recalculates_its_totals() {
    let mut doc = open("forms/scripts.pdf");
    let qty = id(&doc, "qty");
    let edited = doc.apply(vec![Op::from(SetFieldValue::new(qty, text("4")))]).unwrap();
    assert_eq!(
        (value(&doc, "total"), edited.skipped_calculations.is_empty()),
        (Some(text("4938")), true)
    );
}

#[test]
fn calculate_brings_stale_totals_up_to_date() {
    let mut doc = open("forms/scripts.pdf");
    doc.calculate().unwrap();
    assert_eq!(
        [value(&doc, "total"), value(&doc, "sum")],
        [Some(text("3703.5")), Some(text("4941"))]
    );
}

#[test]
fn a_total_a_signature_locks_is_left_as_it_is_and_the_save_has_no_conflict() {
    let mut doc = open("signed/scripts-locked-total.pdf");
    let qty = id(&doc, "qty");
    let total = id(&doc, "total");
    let edited = doc.apply(vec![Op::from(SetFieldValue::new(qty, text("4")))]).unwrap();
    let plan = doc.plan(&laminapdf::SaveOptions::new()).unwrap();
    assert_eq!(
        (
            edited.skipped_calculations,
            value(&doc, "total"),
            value(&doc, "sum"),
            plan.violations.len()
        ),
        (vec![total.to_string()], None, Some(text("1238.5")), 0)
    );
}

#[test]
fn form_scripts_lists_every_script_with_its_state() {
    let scripts = open("forms/scripts.pdf").form_scripts().unwrap();
    let count = |state| scripts.iter().filter(|s| s.state == state).count();
    assert_eq!((count(ScriptState::Handled), count(ScriptState::NotHandled)), (32, 11));
}
