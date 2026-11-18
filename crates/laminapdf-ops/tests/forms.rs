//! `SetFieldValue` over `fixtures/forms/fields.pdf` (written by `tools/fixtures/forms.py`):
//! each kind of field, the refusals, buttons' states, `/I` and `/RV`, undo and redo, and the wire
//! form.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::{open, snapshot};
use laminapdf_build::AnnotationAppearances;
use laminapdf_cos::{Object, PdfString};
use laminapdf_model::{FieldId, FieldValue, TerminalField};
use laminapdf_ops::{Document, Error, ModificationClass, Op, Origin, SetFieldValue, UndoOutcome};
use laminapdf_test_fixtures::fixture;

fn form() -> Document {
    open(&fixture("forms/fields.pdf")).with_appearances(AnnotationAppearances)
}

fn field(doc: &Document, name: &str) -> TerminalField {
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
    let font = doc.add_blob(fixture("fonts/NotoSans-Regular.ttf"));
    let id = field(&doc, "address").id;
    let op = SetFieldValue::new(id, text("Łódź, Gdańsk, Kraków")).with_fonts(vec![font]);
    doc.apply_all(Origin::Local, [Op::from(op)]).unwrap();
    assert_eq!(field(&doc, "address").value, Some(text("Łódź, Gdańsk, Kraków")));
}

#[test]
fn text_no_font_draws_is_refused_before_anything_is_written() {
    let mut doc = form();
    let result = set(&mut doc, "address", text("Тверская"));
    assert_eq!(
        (
            matches!(result, Err(Error::AnnotationGlyphMissing)),
            doc.overlay().is_empty()
        ),
        (true, true)
    );
}

#[test]
fn a_text_fill_without_an_appearance_generator_is_refused() {
    let mut doc = open(&fixture("forms/fields.pdf"));
    assert!(matches!(
        set(&mut doc, "name", text("Grace")),
        Err(Error::AppearanceUnavailable)
    ));
}

#[test]
fn a_check_box_needs_no_generator_since_it_switches_states() {
    let mut doc = open(&fixture("forms/fields.pdf"));
    assert!(set(&mut doc, "agree", FieldValue::Button(None)).is_ok());
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
fn radios_not_in_unison_turn_on_only_the_first_widget_of_the_state() {
    // Both widgets show /round in the file; setting the state leaves only the first on.
    let mut doc = form();
    let before = states(&doc, "shape");
    set(&mut doc, "shape", on("round")).unwrap();
    assert_eq!(
        (before, states(&doc, "shape")),
        (strings(&["round", "round"]), strings(&["round", "Off"]))
    );
}

#[test]
fn a_comb_field_refuses_text_longer_than_its_max_len() {
    let mut doc = form();
    assert_eq!(reason(&set(&mut doc, "code", text("123456"))), Some("max-len"));
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

#[test]
fn a_check_box_with_no_appearance_gains_on_and_off_states() {
    let mut doc = open(&fixture("forms/text-layout.pdf")).with_appearances(AnnotationAppearances);
    set(&mut doc, "tick", on("Yes")).unwrap();
    let tick = field(&doc, "tick");
    assert_eq!(
        (tick.value.clone(), tick.widgets[0].state.clone(), tick.on_states()),
        (Some(on("Yes")), Some("Yes".to_owned()), vec!["Yes"])
    );
}

/// A fill writes nothing but its field's widget (here merged with the field), new objects (the
/// appearance, the subset font) and the form dictionary, whose `/DR` gains the font.
#[test]
fn a_fill_writes_only_its_widget_new_objects_and_the_forms_resources() {
    let mut doc = open(&fixture("forms/text-layout.pdf")).with_appearances(AnnotationAppearances);
    let noto = doc.add_blob(fixture("fonts/NotoSans-Regular.ttf"));
    let latin = field(&doc, "latin");
    let op = SetFieldValue::new(latin.id.clone(), text("Łódź")).with_fonts(vec![noto]);
    doc.apply_all(Origin::Local, [Op::from(op)]).unwrap();
    let field_object = latin.id.as_lamina_id().obj_ref().unwrap().num;
    let floor = doc.size_floor();
    let mut old: Vec<u32> = doc
        .overlay()
        .iter()
        .map(|(num, _)| num)
        .filter(|num| i64::from(*num) < floor)
        .collect();
    old.sort_unstable();
    // Object 5 is the form dictionary, which holds `/DR`.
    assert_eq!(old, [5, field_object]);
}

#[test]
fn a_right_to_left_value_is_drawn_in_its_fallback_with_a_unicode_map() {
    let mut doc = open(&fixture("forms/text-layout.pdf")).with_appearances(AnnotationAppearances);
    let hebrew = doc.add_blob(fixture("fonts/NotoSansHebrew-Regular.ttf"));
    let op = SetFieldValue::new(field(&doc, "latin").id, text("שלום עולם")).with_fonts(vec![hebrew]);
    doc.apply_all(Origin::Local, [Op::from(op)]).unwrap();
    let mut out = Vec::new();
    laminapdf_save::Save::save(&mut doc, &mut out, &laminapdf_save::SaveOptions::new()).unwrap();
    let reopened = Document::open(out, laminapdf_cos::OpenOptions::default()).unwrap();
    let latin = field(&reopened, "latin");
    // Its appearance's fonts: each a dictionary; the embedded one maps glyphs back to text.
    let resolve = |o: &Object| reopened.cos().deref(o).unwrap().into_owned();
    let widget = resolve(&Object::Ref(latin.widgets[0].id.as_lamina_id().obj_ref().unwrap()));
    let normal = resolve(widget.as_dict().unwrap().get("AP").unwrap())
        .as_dict()
        .unwrap()
        .get("N")
        .cloned()
        .unwrap();
    let Object::Stream(form) = resolve(&normal) else {
        panic!("no appearance stream")
    };
    let resources = resolve(form.dict.get("Resources").unwrap());
    let fonts = resolve(resources.as_dict().unwrap().get("Font").unwrap());
    let mapped = fonts.as_dict().unwrap().iter().any(|(_, font)| {
        resolve(font)
            .as_dict()
            .is_some_and(|f| f.get_name("Subtype") == Some(b"Type0".as_slice()) && f.get("ToUnicode").is_some())
    });
    assert_eq!((latin.value, mapped), (Some(text("שלום עולם")), true));
}

#[test]
fn off_is_never_an_on_state_and_a_radio_group_without_appearances_is_refused() {
    let mut doc = open(&fixture("forms/text-layout.pdf")).with_appearances(AnnotationAppearances);
    assert_eq!(reason(&set(&mut doc, "tick", on("Off"))), Some("state"));
}

#[test]
fn a_refill_replaces_the_fonts_an_earlier_fill_added_to_the_forms_resources() {
    let mut doc = open(&fixture("forms/text-layout.pdf")).with_appearances(AnnotationAppearances);
    let noto = doc.add_blob(fixture("fonts/NotoSans-Regular.ttf"));
    for value in ["Łódź", "Gdańsk"] {
        let op = SetFieldValue::new(field(&doc, "latin").id, text(value)).with_fonts(vec![noto]);
        doc.apply_all(Origin::Local, [Op::from(op)]).unwrap();
    }
    let acro = doc
        .cos()
        .deref(&Object::Ref(laminapdf_cos::ObjRef::new(5, 0)))
        .unwrap()
        .into_owned();
    let dr = acro.as_dict().unwrap().get("DR").unwrap().as_dict().unwrap().clone();
    let fonts = dr.get("Font").unwrap().as_dict().unwrap().clone();
    let ours = fonts
        .iter()
        .filter(|(name, _)| name.as_bytes().starts_with(b"LmnaFont"))
        .count();
    assert_eq!(ours, 1);
}

/// The content of the normal appearance of `name`'s first widget.
fn drawn(doc: &Document, name: &str) -> String {
    let widget = field(doc, name).widgets.first().unwrap().id.as_lamina_id().obj_ref().unwrap();
    let resolve = |o: &Object| doc.cos().deref(o).unwrap().into_owned();
    let dict = resolve(&Object::Ref(widget));
    let normal = resolve(dict.as_dict().unwrap().get("AP").unwrap())
        .as_dict()
        .unwrap()
        .get("N")
        .cloned()
        .unwrap();
    let Object::Stream(form) = resolve(&normal) else {
        panic!("no appearance stream")
    };
    String::from_utf8_lossy(&doc.cos().decode_stream(&form).unwrap()).into_owned()
}

#[test]
fn a_kid_widgets_own_da_sets_its_colour_and_size() {
    let mut doc = open(&fixture("forms/text-layout.pdf")).with_appearances(AnnotationAppearances);
    set(&mut doc, "styled", text("Red")).unwrap();
    let content = drawn(&doc, "styled");
    assert!(content.contains("1 0 0 rg") && content.contains(" 9 Tf"), "{content}");
}

#[test]
fn an_auto_sized_line_shrinks_to_fit_its_width() {
    let mut doc = open(&fixture("forms/text-layout.pdf")).with_appearances(AnnotationAppearances);
    // `auto` is 250 pt wide and 24 tall: by height 14.8 pt, which this text overflows.
    set(&mut doc, "auto", text(&"Wide ".repeat(12))).unwrap();
    let content = drawn(&doc, "auto");
    let size: f64 = content
        .split(" Tf")
        .next()
        .and_then(|before| before.rsplit(' ').next())
        .and_then(|s| s.parse().ok())
        .unwrap();
    assert!(size < 14.0, "{size}");
}
