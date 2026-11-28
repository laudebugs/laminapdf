//! `Document::form_actions` over `fixtures/forms/actions.pdf` (written by
//! `tools/fixtures/forms.py`): each action typed, `/Next` chains, field selections and Hide
//! targets.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf_model::{
    ActionTrigger, Document, FieldSelection, FormAction, HideTarget, NamedAction, OpenOptions, SubmitFlags,
};
use laminapdf_test_fixtures::fixture;

fn open() -> Document {
    Document::open(fixture("forms/actions.pdf"), OpenOptions::default()).unwrap()
}

/// The actions of the field named `name`.
fn actions(doc: &Document, name: &str) -> Vec<FormAction> {
    let form = doc.form().unwrap();
    let field = form.field_named(name).unwrap();
    doc.form_actions(field).unwrap().into_iter().map(|a| a.action).collect()
}

#[test]
fn page_moves_are_typed() {
    let doc = open();
    assert_eq!(actions(&doc, "next"), [FormAction::GoTo { page_index: Some(1) }]);
    assert_eq!(actions(&doc, "nextpage"), [FormAction::Named(NamedAction::NextPage)]);
}

#[test]
fn an_address_is_read_as_stored() {
    let doc = open();
    assert_eq!(
        actions(&doc, "web"),
        [FormAction::Uri("https://example.com/help".to_owned())]
    );
}

#[test]
fn a_click_is_the_trigger_of_a_buttons_action() {
    let doc = open();
    let form = doc.form().unwrap();
    let found = doc.form_actions(form.field_named("web").unwrap()).unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].trigger, ActionTrigger::Click);
}

#[test]
fn reset_selections_name_fields_by_name_or_reference_and_keep_the_exclude_bit() {
    let doc = open();
    let FormAction::ResetForm(all) = &actions(&doc, "reset")[0] else {
        panic!("not a reset")
    };
    assert_eq!(all, &FieldSelection::default());
    let FormAction::ResetForm(by_name) = &actions(&doc, "resetname")[0] else {
        panic!("not a reset")
    };
    assert_eq!(
        (by_name.names.as_slice(), by_name.exclude),
        (&["name".to_owned()][..], false)
    );
    let FormAction::ResetForm(rest) = &actions(&doc, "resetrest")[0] else {
        panic!("not a reset")
    };
    assert_eq!((rest.names.as_slice(), rest.exclude), (&["name".to_owned()][..], true));
    let form = doc.form().unwrap();
    let names: Vec<&str> = rest.fields(&form).iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["email", "notes", "secret", "empty"]);
}

#[test]
fn hide_names_a_field_and_an_annotation_that_is_not_a_widget() {
    let doc = open();
    let form = doc.form().unwrap();
    let FormAction::Hide { targets, hide } = &actions(&doc, "hide")[0] else {
        panic!("not a hide")
    };
    assert!(*hide);
    assert_eq!(targets.len(), 2);
    assert_eq!(
        targets[0],
        HideTarget::Field(form.field_named("notes").unwrap().id.clone())
    );
    assert!(matches!(targets[1], HideTarget::Annotation(_)));
}

#[test]
fn submit_actions_carry_the_address_the_flags_and_the_selection() {
    let doc = open();
    let FormAction::SubmitForm(fdf) = &actions(&doc, "send")[0] else {
        panic!("not a submit")
    };
    assert_eq!(fdf.url, "https://forms.example.com/submit");
    assert_eq!(fdf.flags, SubmitFlags(0));
    let FormAction::SubmitForm(xfdf) = &actions(&doc, "sendxfdf")[0] else {
        panic!("not a submit")
    };
    assert_eq!(xfdf.url, "https://forms.example.com/submit", "a URL file specification");
    assert!(xfdf.flags.has(SubmitFlags::XFDF));
    let FormAction::SubmitForm(some) = &actions(&doc, "sendsome")[0] else {
        panic!("not a submit")
    };
    assert_eq!(some.selection.names, ["secret", "empty"]);
    assert!(some.selection.exclude);
}

#[test]
fn actions_that_are_never_run_are_still_named() {
    let doc = open();
    assert_eq!(actions(&doc, "script"), [FormAction::JavaScript]);
    assert_eq!(actions(&doc, "launch"), [FormAction::Launch]);
    assert_eq!(actions(&doc, "import"), [FormAction::ImportData]);
}

#[test]
fn next_chains_are_followed_in_order() {
    let doc = open();
    let found = actions(&doc, "chain");
    assert_eq!(found.len(), 2);
    assert!(matches!(found[0], FormAction::ResetForm(_)));
    assert!(matches!(found[1], FormAction::Hide { .. }));
}

#[test]
fn a_field_without_actions_has_none() {
    let doc = open();
    assert_eq!(actions(&doc, "name"), []);
}

#[test]
fn a_selection_covers_the_fields_below_a_name() {
    let selection = FieldSelection::new(vec!["person".to_owned()], false);
    assert!(selection.covers("person"));
    assert!(selection.covers("person.first"));
    assert!(!selection.covers("personal"));
    let all_but = FieldSelection::new(selection.names, true);
    assert!(!all_but.covers("person.first"));
    assert!(all_but.covers("other"));
}
