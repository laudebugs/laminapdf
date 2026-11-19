//! Form behaviour in `SetFieldValue` (todo E8f, ADR 0021) over `fixtures/forms/scripts.pdf`:
//! values committed through their keystroke and validate helpers, appearances drawn through
//! their formats, and calculations run in `/CO` order in the same transaction, skipping locked
//! totals.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

mod common;

use common::{drawn, open};
use laminapdf_build::AnnotationAppearances;
use laminapdf_model::{FieldValue, TerminalField};
use laminapdf_ops::{Document, Error, FieldLocks, Op, Origin, SetFieldValue};
use laminapdf_test_fixtures::fixture;

fn form() -> Document {
    open(&fixture("forms/scripts.pdf")).with_appearances(AnnotationAppearances)
}

fn value(doc: &Document, name: &str) -> Option<FieldValue> {
    doc.form().unwrap().field_named(name).unwrap().value.clone()
}

fn set(doc: &mut Document, name: &str, typed: &str) -> laminapdf_ops::Result<laminapdf_ops::Applied> {
    let id = doc.form().unwrap().field_named(name).unwrap().id.clone();
    doc.apply_all(Origin::Local, [Op::from(SetFieldValue::new(id, text(typed)))])
}

fn text(s: &str) -> FieldValue {
    FieldValue::Text(s.to_owned())
}

#[test]
fn a_change_recalculates_every_total_in_order_in_the_same_transaction() {
    let mut doc = form();
    set(&mut doc, "qty", "4").unwrap();
    // total = price × qty; sum = price + qty + total, after total.
    assert_eq!(
        [value(&doc, "total"), value(&doc, "sum")],
        [Some(text("4938")), Some(text("6176.5"))]
    );
}

#[test]
fn a_comma_point_total_calculates_and_stores_its_number_with_a_point() {
    let mut doc = form();
    set(&mut doc, "qty", "4").unwrap();
    // euros = price + qty, shown as pdf.js shows it (scripts.pdfjs.json).
    let shown = drawn(&doc, "euros");
    assert!(
        value(&doc, "euros") == Some(text("1238.5")) && shown.contains("(1.238,50 EUR) Tj"),
        "{shown}"
    );
}

#[test]
fn undo_reverts_a_change_and_its_totals_together() {
    let mut doc = form();
    set(&mut doc, "qty", "4").unwrap();
    doc.undo().unwrap();
    assert_eq!(
        [value(&doc, "qty"), value(&doc, "total"), value(&doc, "sum")],
        [Some(text("3")), None, None]
    );
}

#[test]
fn a_total_is_drawn_through_its_format_and_keeps_its_number_in_v() {
    let mut doc = form();
    set(&mut doc, "qty", "4").unwrap();
    let shown = drawn(&doc, "total");
    assert!(shown.contains("($4,938.00) Tj"), "{shown}");
}

#[test]
fn a_negative_red_style_draws_the_text_red() {
    let mut doc = form();
    set(&mut doc, "owed", "-2").unwrap();
    let shown = drawn(&doc, "owed");
    // £2.00, without its minus sign.
    assert!(
        shown.contains("1 0 0 rg") && shown.contains("<A3322E3030> Tj"),
        "{shown}"
    );
}

#[test]
fn a_value_its_helpers_refuse_is_never_committed() {
    let mut doc = form();
    let reason = |r: laminapdf_ops::Result<_>| match r {
        Err(Error::FieldValueInvalid { reason }) => Some(reason),
        _ => None,
    };
    assert_eq!(
        [
            reason(set(&mut doc, "price", "20000")),
            reason(set(&mut doc, "price", "12a")),
            reason(set(&mut doc, "due", "someday")),
            reason(set(&mut doc, "zip", "1234")),
        ],
        [Some("range"), Some("format"), Some("format"), Some("format")]
    );
    assert_eq!(value(&doc, "price"), Some(text("1234.5")));
}

#[test]
fn a_value_without_a_keystroke_check_is_stored_as_typed_and_shown_formatted() {
    let mut doc = form();
    // `balance` formats `1.234,56` in parentheses and has no keystroke script.
    set(&mut doc, "balance", "-12,5").unwrap();
    let shown = drawn(&doc, "balance");
    assert!(
        value(&doc, "balance") == Some(text("-12,5")) && shown.contains("((12,50)) Tj"),
        "{shown}"
    );
}

/// Locks the field named `total`.
struct TotalLocked;

impl FieldLocks for TotalLocked {
    fn locked(
        &self,
        _: &laminapdf_model::Document,
        fields: &[&TerminalField],
    ) -> laminapdf_cos::Result<std::collections::BTreeSet<laminapdf_model::FieldId>> {
        Ok(fields
            .iter()
            .filter(|f| f.name == "total")
            .map(|f| f.id.clone())
            .collect())
    }
}

#[test]
fn a_locked_total_is_skipped_and_listed_and_the_rest_still_calculate() {
    let mut doc = form().with_field_locks(TotalLocked);
    let applied = set(&mut doc, "qty", "4").unwrap();
    let total = doc.form().unwrap().field_named("total").unwrap().id.clone();
    // sum reads total's stored value, none, as 0: price + qty.
    assert_eq!(
        (applied.skipped_calculations, value(&doc, "total"), value(&doc, "sum")),
        (vec![total], None, Some(text("1238.5")))
    );
}

#[test]
fn a_remote_change_carries_its_totals_and_calculates_nothing() {
    let mut doc = form();
    let id = doc.form().unwrap().field_named("qty").unwrap().id.clone();
    let peer = Origin::Remote(laminapdf_ops::PeerId::new("peer"));
    doc.apply_all(peer, [Op::from(SetFieldValue::new(id, text("4")))])
        .unwrap();
    assert_eq!(value(&doc, "total"), None);
}

#[test]
fn hiding_a_field_sets_its_widgets_hidden_flag_and_showing_clears_it() {
    let mut doc = form();
    let id = doc.form().unwrap().field_named("price").unwrap().id.clone();
    let flags = |doc: &Document| doc.form().unwrap().field_named("price").unwrap().widgets[0].flags.0;
    doc.apply_all(
        Origin::Local,
        [Op::from(laminapdf_ops::SetFieldHidden::new(id.clone(), true))],
    )
    .unwrap();
    let hidden = flags(&doc);
    doc.apply_all(Origin::Local, [Op::from(laminapdf_ops::SetFieldHidden::new(id, false))])
        .unwrap();
    // The fixture's widgets print (/F 4).
    assert_eq!([hidden, flags(&doc)], [6, 4]);
}

#[test]
fn a_calculated_value_is_marked_on_the_wire_and_typed_text_still_takes_the_keystroke_check() {
    let mut doc = form();
    let id = doc.form().unwrap().field_named("euros").unwrap().id.clone();
    let typed = SetFieldValue::new(id, text("1238.5"));
    let mut calculated = typed.clone();
    calculated.calculated = true;
    let json = serde_json::to_string(&Op::from(calculated.clone())).unwrap();
    // A comma-point field refuses typed `1238.5`; the same value from a calculation passes.
    assert_eq!(
        (
            json.contains(r#""calculated":true"#),
            doc.apply_all(Origin::Local, [Op::from(typed)]).is_err(),
            doc.apply_all(Origin::Local, [Op::from(calculated)]).is_ok(),
        ),
        (true, true, true)
    );
}

#[test]
fn set_field_hidden_round_trips_on_the_wire() {
    let json = r#"{"op":"setFieldHidden","args":{"field":"f20.0","hidden":true}}"#;
    let op: Op = serde_json::from_str(json).unwrap();
    assert_eq!(serde_json::to_string(&op).unwrap(), json);
}

/// SBA Form 413 (`fixtures/forms/sba-413.pdf`, a US federal form, public domain): the steps of
/// `sba-413.steps.json` applied one by one, as a person fills it, then every field pdf.js shows
/// compared with what Lamina PDF shows (`sba-413.pdfjs.json`, from `tools/differential/forms.mjs`).
#[test]
fn a_real_form_shows_what_pdf_js_shows_after_the_same_fill() {
    let mut doc = open(&fixture("forms/sba-413.pdf")).with_appearances(AnnotationAppearances);
    let reference: serde_json::Value = serde_json::from_slice(&fixture("forms/sba-413.pdfjs.json")).unwrap();
    let steps = reference["steps"].as_array().unwrap();
    for step in steps {
        let name = step["field"].as_str().unwrap();
        let typed = step["value"].as_str().unwrap();
        // A value its keystroke check refuses (`18,5` in a point field) isn't committed; pdf.js
        // empties the field instead (E8 plan §2.3).
        if set(&mut doc, name, typed).is_err() {
            assert_eq!(step["shown"][name], "", "{name}: pdf.js refused {typed:?} too");
        }
    }
    let form = doc.form().unwrap();
    let last = &steps.last().unwrap()["shown"];
    let mut compared = 0;
    for (name, shown) in last.as_object().unwrap() {
        // Net Worth is a custom script (`TotalAssets - TotalLiabilities`): not a helper call, so
        // Lamina PDF doesn't calculate it (ADR 0021 §2); refused values are checked above.
        if name == "Net Worth" || shown == "" {
            continue;
        }
        let field = form.field_named(name).unwrap();
        let value = match &field.value {
            Some(FieldValue::Text(t)) => t.clone(),
            _ => String::new(),
        };
        let ours = doc.field_behaviour(field).unwrap().show(&value).text;
        assert_eq!(ours, shown.as_str().unwrap(), "{name}");
        compared += 1;
    }
    assert_eq!(compared, 11);
}

#[test]
fn the_real_forms_scripts_are_almost_all_handled() {
    let scripts = open(&fixture("forms/sba-413.pdf")).form_scripts().unwrap();
    let count = |state| scripts.iter().filter(|s| s.state == state).count();
    // Every format, keystroke and both sums are handled; not handled are Net Worth's custom
    // calculation and seven push buttons' click scripts on page 2.
    assert_eq!(
        (
            count(laminapdf_model::ScriptState::Handled),
            count(laminapdf_model::ScriptState::NotHandled)
        ),
        (164, 8)
    );
}
