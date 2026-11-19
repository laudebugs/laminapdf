//! Form behaviour without JavaScript (todo E8f, ADR 0021) over `fixtures/forms/scripts.pdf`:
//! each field's helper calls read as data, what each format shows against what pdf.js shows
//! (`fixtures/forms/scripts.pdfjs.json`, from `tools/differential/forms.mjs`), and the inventory
//! of every script with its state.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf_model::{
    Behaviour, Document, FieldTrigger, FormScript, Limits, OpenOptions, Owner, ScriptLocation, ScriptState, Trigger,
};
use laminapdf_test_fixtures::fixture;

fn open() -> Document {
    Document::open(fixture("forms/scripts.pdf"), OpenOptions::default()).unwrap()
}

fn behaviour(doc: &Document, name: &str) -> Behaviour {
    let form = doc.form().unwrap();
    doc.field_behaviour(form.field_named(name).unwrap()).unwrap()
}

/// Where Lamina PDF shows something else than pdf.js, and why.
const NOT_LIKE_PDFJS: [(&str, &str); 7] = [
    (
        "at",
        "pdf.js's fallback date reader skips `h` hours, so 14:05 under h:MM tt reads as 12:14",
    ),
    (
        "custom",
        "not a helper call: pdf.js runs it, Lamina PDF never runs a script (ADR 0021 §1)",
    ),
    ("twice", "two statements: not a helper call"),
    ("chained", "a /Next chain: not a helper call"),
    ("added", "an expression argument: not a helper call, so not calculated"),
    ("doubled", "this.getField: not a helper call, so not calculated"),
    // pdf.js empties a field whose value fails validation; the plan keeps the last valid one.
    (
        "refused",
        "a value that fails validation is never committed (E8 plan §2.3)",
    ),
];

#[test]
fn every_format_shows_what_pdf_js_shows_for_each_fields_own_value() {
    let doc = open();
    let reference: serde_json::Value = serde_json::from_slice(&fixture("forms/scripts.pdfjs.json")).unwrap();
    let skipped: Vec<&str> = NOT_LIKE_PDFJS.iter().map(|(name, _)| *name).collect();
    let mut compared = 0;
    for step in reference["steps"].as_array().unwrap() {
        let name = step["field"].as_str().unwrap();
        let typed = step["value"].as_str().unwrap();
        let expected = step["shown"][name].as_str().unwrap_or(typed);
        let b = behaviour(&doc, name);
        if skipped.contains(&name) || b.commit(typed).is_err() {
            continue;
        }
        let value = b.commit(typed).unwrap();
        assert_eq!(b.show(&value).text, expected, "{name} = {typed:?}");
        compared += 1;
    }
    assert_eq!(compared, 24);
}

#[test]
fn a_value_outside_the_range_or_not_a_number_is_refused_at_commit() {
    let doc = open();
    let price = behaviour(&doc, "price");
    assert_eq!(
        [
            price.commit("20000").is_err(),
            price.commit("12a").is_err(),
            price.commit("99.5").is_ok()
        ],
        [true, true, true]
    );
}

#[test]
fn a_negative_red_style_draws_red() {
    let doc = open();
    assert_eq!(
        [
            behaviour(&doc, "owed").show("-0.5").red,
            behaviour(&doc, "owed").show("0.5").red
        ],
        [true, false]
    );
}

#[test]
fn scripts_that_are_not_helper_calls_are_listed_by_trigger() {
    let doc = open();
    let not_handled = |name| behaviour(&doc, name).not_handled;
    assert_eq!(
        [
            not_handled("custom"),
            not_handled("chained"),
            not_handled("doubled"),
            not_handled("price")
        ],
        [
            vec![FieldTrigger::Format],
            vec![FieldTrigger::Format],
            vec![FieldTrigger::Calculate],
            vec![]
        ]
    );
}

#[test]
fn a_kid_widgets_own_format_is_read_from_the_widget_up() {
    let doc = open();
    assert_eq!(behaviour(&doc, "kid").show("7").text, "7.0");
}

#[test]
fn a_push_buttons_click_script_is_listed_and_not_handled() {
    let doc = open();
    let go = doc
        .form()
        .unwrap()
        .field_named("go")
        .unwrap()
        .id
        .as_lamina_id()
        .obj_ref()
        .unwrap()
        .num;
    let scripts = doc.form_scripts().unwrap();
    // The button is merged with its widget: its click is that annotation's `/A`.
    let go = scripts.iter().find(|s| {
        matches!(
            &s.location,
            ScriptLocation::Action {
                trigger: Trigger::Activate,
                owner: Owner::Annotation { object: Some(n), .. },
            } if *n == go
        )
    });
    assert_eq!(go.map(|s| s.state), Some(ScriptState::NotHandled));
}

#[test]
fn a_percent_shows_percent_for_text_with_no_number_and_nothing_for_an_empty_field() {
    let doc = open();
    let rate = behaviour(&doc, "rate");
    assert_eq!([rate.show("abc").text, rate.show("").text], ["%", ""]);
}

#[test]
fn calculations_read_their_fields_by_name() {
    let doc = open();
    let form = doc.form().unwrap();
    let total = behaviour(&doc, "total").calculate.unwrap();
    // price 1234.5 times qty 3.
    assert_eq!(total.compute(&laminapdf_model::FormValues::of(&form)), "3703.5");
}

/// Each script's location, state and size, as `(owner or packet, trigger, state, bytes)`.
fn listed(scripts: &[FormScript]) -> Vec<(String, String, ScriptState, u64)> {
    scripts
        .iter()
        .map(|s| match &s.location {
            ScriptLocation::Action { trigger, owner } => {
                let owner = match owner {
                    Owner::Document => "document".to_owned(),
                    Owner::Page(i) => format!("page {i}"),
                    Owner::FormField(name) => name.clone(),
                    other => format!("{other:?}"),
                };
                let trigger = match trigger {
                    Trigger::Additional(key) => key.clone(),
                    Trigger::NamedScript(name) => format!("named {name}"),
                    other => format!("{other:?}"),
                };
                (owner, trigger, s.state, s.bytes)
            }
            ScriptLocation::Xfa { packet } => (packet.clone(), String::new(), s.state, s.bytes),
            other => (format!("{other:?}"), String::new(), s.state, s.bytes),
        })
        .collect()
}

#[test]
fn the_inventory_lists_every_script_once_with_its_state() {
    let scripts = listed(&open().form_scripts().unwrap());
    let count = |state| scripts.iter().filter(|s| s.2 == state).count();
    let document = (
        "document".to_owned(),
        "named hello".to_owned(),
        ScriptState::NotHandled,
        19,
    );
    let page = ("page 0".to_owned(), "O".to_owned(), ScriptState::NotHandled, 19);
    let chained: Vec<_> = scripts.iter().filter(|s| s.0 == "chained").collect();
    assert_eq!(
        (
            scripts.contains(&document),
            scripts.contains(&page),
            chained.len(),
            count(ScriptState::Handled),
            count(ScriptState::NotHandled),
        ),
        (true, true, 2, 32, 11)
    );
}

#[test]
fn a_script_longer_than_the_limit_is_not_handled() {
    let doc = Document::open(
        fixture("forms/scripts.pdf"),
        OpenOptions::default().with_limits(Limits::default().with_max_script_length(10)),
    )
    .unwrap();
    assert_eq!(
        behaviour(&doc, "zip").not_handled,
        [FieldTrigger::Keystroke, FieldTrigger::Format]
    );
}
