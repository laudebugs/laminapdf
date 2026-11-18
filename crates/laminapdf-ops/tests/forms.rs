//! `SetFieldValue` over `fixtures/forms/fields.pdf` (written by `tools/fixtures/forms.py`):
//! each kind of field, the refusals, buttons' states, `/I` and `/RV`, undo and redo, and the wire
//! form.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::{open, snapshot};
use laminapdf_cos::{Object, PdfString};
use laminapdf_model::{Field, FieldId, FieldValue};
use laminapdf_ops::{Document, Error, ModificationClass, Op, Origin, SetFieldValue, UndoOutcome};
use laminapdf_test_fixtures::fixture;

fn form() -> Document {
    open(&fixture("forms/fields.pdf"))
}

fn field(doc: &Document, name: &str) -> Field {
    doc.form().unwrap().field_named(name).cloned().unwrap()
}

fn set(doc: &mut Document, name: &str, value: FieldValue) -> laminapdf_ops::Result<()> {
    let id = field(doc, name).id;
    doc.apply_all(Origin::Local, [Op::from(SetFieldValue::new(id, value))])
        .map(|_| ())
}

fn reason(result: &laminapdf_ops::Result<()>) -> Option<&'static str> {
    match *result {
        Err(Error::FieldValueInvalid { reason }) => Some(reason),
        _ => None,
    }
}

fn text(s: &str) -> FieldValue {
    FieldValue::Text(s.to_owned())
}

fn on(s: &str) -> FieldValue {
    FieldValue::Button(Some(s.to_owned()))
}

fn choice(values: &[&str]) -> FieldValue {
    FieldValue::Choice(values.iter().map(|s| (*s).to_owned()).collect())
}

/// The states `name`'s widgets show.
fn states(doc: &Document, name: &str) -> Vec<String> {
    field(doc, name)
        .widgets
        .iter()
        .map(|w| w.state.clone().unwrap())
        .collect()
}

/// Entry `key` of the field object `id`.
fn entry(doc: &Document, id: &FieldId, key: &str) -> Option<Object> {
    let r = id.as_lamina_id().obj_ref().unwrap();
    let reference = Object::Ref(r);
    let object = doc.cos().deref(&reference).unwrap();
    object.as_dict().unwrap().get(key).cloned()
}

#[test]
fn a_text_value_reads_back() {
    let mut doc = form();
    set(&mut doc, "name", text("Grace Hopper")).unwrap();
    assert_eq!(field(&doc, "name").value, Some(text("Grace Hopper")));
}

#[test]
fn text_outside_pdfdocencoding_is_written_as_utf16_and_reads_back() {
    let mut doc = form();
    set(&mut doc, "address", text("Łódź, ул. Тверская")).unwrap();
    assert_eq!(field(&doc, "address").value, Some(text("Łódź, ул. Тверская")));
}

#[test]
fn text_longer_than_max_len_is_refused() {
    let mut doc = form();
    assert_eq!(reason(&set(&mut doc, "name", text(&"x".repeat(41)))), Some("max-len"));
}

#[test]
fn text_of_max_len_characters_is_accepted() {
    let mut doc = form();
    assert!(set(&mut doc, "name", text(&"é".repeat(40))).is_ok());
}

#[test]
fn setting_text_removes_the_rich_text_value() {
    let mut doc = form();
    set(&mut doc, "notes", text("Buy bread")).unwrap();
    let notes = field(&doc, "notes");
    assert_eq!(
        (notes.value, entry(&doc, &notes.id, "RV")),
        (Some(text("Buy bread")), None)
    );
}

#[test]
fn a_value_of_another_kind_is_refused() {
    let mut doc = form();
    assert_eq!(reason(&set(&mut doc, "name", on("Yes"))), Some("kind"));
}

#[test]
fn a_push_button_takes_no_value() {
    let mut doc = form();
    assert_eq!(reason(&set(&mut doc, "submit", FieldValue::Clear)), Some("kind"));
}

#[test]
fn a_signature_field_is_refused() {
    let mut doc = form();
    assert!(matches!(
        set(&mut doc, "signature", text("forged")),
        Err(Error::FieldIsSignature(_))
    ));
}

#[test]
fn an_unknown_field_is_refused() {
    let mut doc = form();
    let op = SetFieldValue::new("f999.0".parse().unwrap(), text("x"));
    assert!(matches!(
        doc.apply_all(Origin::Local, [Op::from(op)]),
        Err(Error::UnknownField(_))
    ));
}

#[test]
fn an_orphan_widgets_parent_is_no_field_of_the_form() {
    let mut doc = open(&fixture("forms/reconcile.pdf"));
    let op = SetFieldValue::new("f21.0".parse().unwrap(), text("x"));
    assert!(matches!(
        doc.apply_all(Origin::Local, [Op::from(op)]),
        Err(Error::UnknownField(_))
    ));
}

#[test]
fn a_check_box_turns_off_and_on() {
    let mut doc = form();
    set(&mut doc, "agree", FieldValue::Button(None)).unwrap();
    let off = (field(&doc, "agree").value, states(&doc, "agree"));
    set(&mut doc, "agree", on("Yes")).unwrap();
    assert_eq!(
        (off, field(&doc, "agree").value, states(&doc, "agree")),
        (
            (Some(FieldValue::Button(None)), vec!["Off".to_owned()]),
            Some(on("Yes")),
            vec!["Yes".to_owned()]
        )
    );
}

#[test]
fn a_button_state_no_widget_has_is_refused() {
    let mut doc = form();
    assert_eq!(reason(&set(&mut doc, "agree", on("Maybe"))), Some("state"));
}

#[test]
fn radios_in_unison_turn_on_every_widget_of_the_state() {
    let mut doc = form();
    set(&mut doc, "colour", on("green")).unwrap();
    let green = states(&doc, "colour");
    set(&mut doc, "colour", on("red")).unwrap();
    assert_eq!(
        (green, states(&doc, "colour")),
        (strings(&["Off", "green", "Off"]), strings(&["red", "Off", "red"]))
    );
}

#[test]
fn a_radio_group_with_opt_takes_an_export_value() {
    let mut doc = form();
    set(&mut doc, "size", on("Small")).unwrap();
    assert_eq!(
        (field(&doc, "size").value, states(&doc, "size")),
        (Some(on("0")), strings(&["0", "Off"]))
    );
}

#[test]
fn a_combo_box_takes_an_option() {
    let mut doc = form();
    set(&mut doc, "country", choice(&["FR"])).unwrap();
    assert_eq!(field(&doc, "country").value, Some(choice(&["FR"])));
}

#[test]
fn an_editable_combo_box_takes_any_text() {
    let mut doc = form();
    set(&mut doc, "country", choice(&["Kenya"])).unwrap();
    assert_eq!(field(&doc, "country").value, Some(choice(&["Kenya"])));
}

#[test]
fn a_combo_box_refuses_several_choices_or_none() {
    let mut doc = form();
    assert_eq!(
        [
            reason(&set(&mut doc, "country", choice(&["UK", "US"]))),
            reason(&set(&mut doc, "country", choice(&[])))
        ],
        [Some("count"), Some("count")]
    );
}

#[test]
fn a_multi_select_list_keeps_its_indices_in_step() {
    let mut doc = form();
    set(&mut doc, "toppings", choice(&["Olives", "Ham"])).unwrap();
    let toppings = field(&doc, "toppings");
    assert_eq!(
        (toppings.value, entry(&doc, &toppings.id, "I")),
        (
            Some(choice(&["Olives", "Ham"])),
            Some(Object::Array(vec![Object::Integer(1), Object::Integer(2)]))
        )
    );
}

#[test]
fn a_list_box_refuses_an_option_it_lacks() {
    let mut doc = form();
    assert_eq!(
        reason(&set(&mut doc, "toppings", choice(&["Pineapple"]))),
        Some("option")
    );
}

#[test]
fn clearing_a_list_box_removes_its_value_and_indices() {
    let mut doc = form();
    set(&mut doc, "toppings", FieldValue::Clear).unwrap();
    let toppings = field(&doc, "toppings");
    assert_eq!((toppings.value, entry(&doc, &toppings.id, "I")), (None, None));
}

#[test]
fn clearing_a_radio_group_turns_every_widget_off() {
    let mut doc = form();
    set(&mut doc, "colour", FieldValue::Clear).unwrap();
    assert_eq!(
        (field(&doc, "colour").value, states(&doc, "colour")),
        (None, strings(&["Off", "Off", "Off"]))
    );
}

#[test]
fn fields_that_share_a_name_get_the_value_together() {
    let mut doc = form();
    set(&mut doc, "total", text("43")).unwrap();
    let total = field(&doc, "total");
    let values: Vec<Option<Object>> = std::iter::once(&total.id)
        .chain(&total.shared)
        .map(|id| entry(&doc, id, "V"))
        .collect();
    let forty_three = Some(Object::String(PdfString::literal(b"43".to_vec())));
    assert_eq!(values, [forty_three.clone(), forty_three]);
}

#[test]
fn setting_the_value_a_field_holds_writes_nothing() {
    let mut doc = form();
    set(&mut doc, "name", text("Ada Lovelace")).unwrap();
    assert!(doc.overlay().is_empty());
}

#[test]
fn the_op_is_a_form_fill_change_naming_its_field() {
    let mut doc = form();
    let id = field(&doc, "name").id;
    set(&mut doc, "name", text("Grace")).unwrap();
    let pending = doc.pending_changes();
    assert_eq!(
        (
            pending.classes().iter().copied().collect::<Vec<_>>(),
            pending.filled_fields().iter().cloned().collect::<Vec<_>>()
        ),
        (vec![ModificationClass::FormFill], vec![id])
    );
}

#[test]
fn undo_leaves_the_overlay_empty_and_redo_writes_the_same_objects() {
    let mut doc = form();
    set(&mut doc, "colour", on("green")).unwrap();
    let applied = snapshot(&doc);
    let outcome = doc.undo().unwrap().unwrap().outcome;
    let empty = doc.overlay().is_empty();
    doc.redo().unwrap();
    assert_eq!(
        (outcome, empty, snapshot(&doc) == applied),
        (UndoOutcome::Applied, true, true)
    );
}

#[test]
fn the_wire_form_round_trips() {
    let json = r#"{"op":"setFieldValue","args":{"field":"f12.0","value":{"type":"button","value":null}}}"#;
    let op: Op = serde_json::from_str(json).unwrap();
    assert_eq!(
        (serde_json::to_string(&op).unwrap(), op),
        (
            json.to_owned(),
            Op::from(SetFieldValue::new("f12.0".parse().unwrap(), FieldValue::Button(None)))
        )
    );
}

#[test]
fn the_wire_form_of_clear_has_no_value() {
    let op = Op::from(SetFieldValue::new("f12.0".parse().unwrap(), FieldValue::Clear));
    assert_eq!(
        serde_json::to_string(&op).unwrap(),
        r#"{"op":"setFieldValue","args":{"field":"f12.0","value":{"type":"clear"}}}"#
    );
}

#[test]
fn the_wire_form_refuses_an_unknown_argument_and_a_cleared_value_with_content() {
    let refused = [
        r#"{"op":"setFieldValue","args":{"field":"f12.0","value":{"type":"clear"},"page":"p3.0"}}"#,
        r#"{"op":"setFieldValue","args":{"field":"f12.0","value":{"type":"clear","value":"x"}}}"#,
        r#"{"op":"setFieldValue","args":{"field":"p12.0","value":{"type":"clear"}}}"#,
    ];
    assert!(refused.iter().all(|json| serde_json::from_str::<Op>(json).is_err()));
}

#[test]
fn the_wire_form_refuses_an_unknown_value_type() {
    let json = r#"{"op":"setFieldValue","args":{"field":"f12.0","value":{"type":"date","value":"2026"}}}"#;
    assert!(serde_json::from_str::<Op>(json).is_err());
}

/// The frozen v1 sample (`fixtures/ops/v1/set-field-value.json`, written for `fields.pdf`'s
/// `name` field) applies, and writes back the same.
#[test]
fn the_v1_sample_applies_and_round_trips() {
    let sample = String::from_utf8(fixture("ops/v1/set-field-value.json")).unwrap();
    let op: Op = serde_json::from_str(&sample).unwrap();
    let frozen: serde_json::Value = serde_json::from_str(&sample).unwrap();
    let mut doc = form();
    doc.apply_all(Origin::Local, [op.clone()]).unwrap();
    assert_eq!(
        (serde_json::to_value(&op).unwrap(), field(&doc, "name").value),
        (frozen, Some(text("Grace Hopper")))
    );
}

fn strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}
