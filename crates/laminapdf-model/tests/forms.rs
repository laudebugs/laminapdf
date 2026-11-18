//! `Document::form` and `Document::tab_order` over `fixtures/forms/` (written by
//! `tools/fixtures/forms.py`): every field kind, inherited values, shared names, the
//! reconciliation report and each `/Tabs` value.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf_model::{ChoiceOption, Document, Field, FieldFlags, FieldKind, FieldValue, Form, OpenOptions, Tabs};
use laminapdf_test_fixtures::fixture;

fn open(name: &str) -> Document {
    Document::open(fixture(&format!("forms/{name}")), OpenOptions::default()).unwrap()
}

fn form() -> Form {
    open("fields.pdf").form().unwrap()
}

fn field<'f>(form: &'f Form, name: &str) -> &'f Field {
    form.field_named(name).unwrap()
}

fn num(id: &impl ToString) -> String {
    id.to_string()
}

#[test]
fn form_should_list_every_typed_terminal_field_in_tree_order() {
    let form = form();
    let names: Vec<&str> = form.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "name",
            "address",
            "agree",
            "colour",
            "size",
            "country",
            "toppings",
            "submit",
            "signature",
            "person.first",
            "person.last",
            "total",
            "notes",
            "hidden",
        ]
    );
}

#[test]
fn form_should_give_each_field_its_kind() {
    let form = form();
    let kinds: Vec<(&str, FieldKind)> = form.fields.iter().map(|f| (f.name.as_str(), f.kind)).collect();
    assert_eq!(
        kinds,
        [
            ("name", FieldKind::Text),
            ("address", FieldKind::Text),
            ("agree", FieldKind::CheckBox),
            ("colour", FieldKind::RadioGroup),
            ("size", FieldKind::RadioGroup),
            ("country", FieldKind::ComboBox),
            ("toppings", FieldKind::ListBox),
            ("submit", FieldKind::PushButton),
            ("signature", FieldKind::Signature),
            ("person.first", FieldKind::Text),
            ("person.last", FieldKind::Text),
            ("total", FieldKind::Text),
            ("notes", FieldKind::Text),
            ("hidden", FieldKind::Text),
        ]
    );
}

#[test]
fn form_should_read_each_kind_of_value() {
    let form = form();
    let value = |name| field(&form, name).value.clone();
    assert_eq!(value("name"), Some(FieldValue::Text("Ada Lovelace".into())));
    assert_eq!(value("agree"), Some(FieldValue::Button(Some("Yes".into()))));
    assert_eq!(value("colour"), Some(FieldValue::Button(Some("red".into()))));
    assert_eq!(value("country"), Some(FieldValue::Choice(vec!["US".into()])));
    assert_eq!(
        value("toppings"),
        Some(FieldValue::Choice(vec!["Cheese".into(), "Olives".into()]))
    );
    assert_eq!(value("submit"), None);
    assert_eq!(value("notes"), Some(FieldValue::Text("Remember the milk".into())));
}

#[test]
fn form_should_read_text_field_entries() {
    let form = form();
    let name = field(&form, "name");
    assert_eq!(
        (name.tooltip.as_deref(), name.max_len, name.required(), name.read_only()),
        (Some("Your name"), Some(40), true, false)
    );
    assert!(field(&form, "address").flags.has(FieldFlags::MULTILINE));
    assert_eq!(field(&form, "address").quadding, 1);
    assert_eq!(
        field(&form, "name").default_appearance.as_deref(),
        Some("/Helv 0 Tf 0 g")
    );
}

#[test]
fn form_should_pass_inherited_flags_and_appearance_to_kids() {
    let form = form();
    let first = field(&form, "person.first");
    assert_eq!(
        (first.read_only(), first.default_appearance.as_deref()),
        (true, Some("/Helv 10 Tf 0 g"))
    );
}

#[test]
fn form_should_read_options_with_and_without_display_text() {
    let form = form();
    assert_eq!(
        field(&form, "country").options,
        [
            ChoiceOption::new("UK".into(), "UK".into()),
            ChoiceOption::new("US".into(), "US".into()),
            ChoiceOption::new("FR".into(), "France".into()),
        ]
    );
}

#[test]
fn form_should_list_a_radio_groups_widgets_and_on_states() {
    let form = form();
    let colour = field(&form, "colour");
    assert_eq!(colour.widgets.len(), 3);
    assert_eq!(colour.on_states(), ["red", "green"]);
    let states: Vec<Option<&str>> = colour.widgets.iter().map(|w| w.state.as_deref()).collect();
    assert_eq!(states, [Some("red"), Some("Off"), Some("red")]);
}

#[test]
fn form_should_merge_fields_that_share_a_name() {
    let form = form();
    let total = field(&form, "total");
    assert_eq!(
        (
            num(&total.id),
            total.shared.iter().map(num).collect::<Vec<_>>(),
            total.widgets.len()
        ),
        ("f30.0".into(), vec!["f31.0".to_owned()], 2)
    );
    assert_eq!(
        form.field(&"f31.0".parse().unwrap()).map(|f| f.name.as_str()),
        Some("total")
    );
}

#[test]
fn form_should_put_each_widget_on_the_page_that_lists_it() {
    let doc = open("fields.pdf");
    let form = doc.form().unwrap();
    let pages = doc.pages().unwrap();
    let second = pages.get(1).unwrap().id().clone();
    assert_eq!(
        field(&form, "total")
            .widgets
            .iter()
            .filter(|w| w.page.as_ref() == Some(&second))
            .count(),
        2
    );
}

#[test]
fn form_should_report_a_signature_field_unsigned() {
    let form = form();
    assert!(!field(&form, "signature").signed);
}

#[test]
fn form_should_report_a_field_whose_widget_no_page_lists() {
    let form = form();
    assert_eq!(form.widgetless.iter().map(num).collect::<Vec<_>>(), ["f33.0"]);
}

#[test]
fn form_should_report_a_widget_no_field_reaches_with_its_parent() {
    let form = open("reconcile.pdf").form().unwrap();
    let orphans: Vec<(String, Option<String>)> = form
        .orphan_widgets
        .iter()
        .map(|o| (num(&o.widget), o.field.as_ref().map(num)))
        .collect();
    assert_eq!(orphans, [("a20.0".to_owned(), Some("f21.0".to_owned()))]);
}

#[test]
fn form_should_report_a_kid_whose_parent_names_another_field() {
    let form = open("reconcile.pdf").form().unwrap();
    // The tree, not the kid's own /Parent, gives its name.
    assert_eq!(
        (
            form.parent_mismatches.iter().map(num).collect::<Vec<_>>(),
            form.field_named("person.last").is_some()
        ),
        (vec!["f12.0".to_owned()], true)
    );
}

#[test]
fn form_should_report_nothing_amiss_in_a_consistent_form() {
    let form = form();
    assert_eq!((form.orphan_widgets.len(), form.parent_mismatches.len()), (0, 0));
}

#[test]
fn form_should_leave_out_a_field_without_a_type_and_report_it() {
    let form = form();
    assert_eq!(
        (
            form.field_named("untyped").is_none(),
            form.untyped.iter().map(num).collect::<Vec<_>>()
        ),
        (true, vec!["f36.0".to_owned()])
    );
}

#[test]
fn form_should_be_empty_for_a_document_without_one() {
    let doc = Document::open(fixture("golden/hello.pdf"), OpenOptions::default()).unwrap();
    assert_eq!(doc.form().unwrap(), Form::default());
}

/// The fields of the widgets `tabs.pdf`'s page `index` lists, in tab order, by corner.
fn tab_corners(index: usize) -> (Option<Tabs>, Vec<String>) {
    let doc = open("tabs.pdf");
    let pages = doc.pages().unwrap();
    let page = pages.get(index).unwrap();
    let order = doc.tab_order(page.id()).unwrap();
    let corners = order
        .widgets
        .iter()
        .map(|w| {
            let name = doc.field_name(w.field).unwrap().unwrap();
            name.rsplit_once('-').unwrap().1.to_owned()
        })
        .collect();
    (order.tabs, corners)
}

/// capability: E.09 core read
#[test]
fn tab_order_should_visit_rows_top_down_each_left_to_right() {
    assert_eq!(tab_corners(0), (Some(Tabs::Row), corners(["tl", "tr", "bl", "br"])));
}

#[test]
fn tab_order_should_visit_columns_left_to_right_each_top_down() {
    assert_eq!(tab_corners(1), (Some(Tabs::Column), corners(["tl", "bl", "tr", "br"])));
}

#[test]
fn tab_order_should_follow_the_structure_tree() {
    assert_eq!(
        tab_corners(2),
        (Some(Tabs::Structure), corners(["tr", "bl", "tl", "br"]))
    );
}

#[test]
fn tab_order_should_follow_annots_for_annotation_and_widget_order_and_none() {
    let annots = corners(["br", "tl", "bl", "tr"]);
    assert_eq!(
        [tab_corners(3), tab_corners(4), tab_corners(5)],
        [
            (Some(Tabs::Annotations), annots.clone()),
            (Some(Tabs::Widgets), annots.clone()),
            (None, annots),
        ]
    );
}

fn corners<const N: usize>(list: [&str; N]) -> Vec<String> {
    list.iter().map(|s| (*s).to_owned()).collect()
}
