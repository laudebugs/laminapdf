//! The façade's form records over `fixtures/forms/` (`tools/fixtures/forms.py`): fields with
//! their widgets in page space, the tab order by page index, and `SetFieldValue` through
//! `apply` with its error's data.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf::{
    Document, FieldKind, FieldValue, Op, OpenOptions, Pt, RectPt, SaveMode, SaveOptions, SetFieldValue, Tabs,
};
use laminapdf_test_fixtures::fixture;

fn open(name: &str) -> Document {
    Document::open_bytes(fixture(&format!("forms/{name}")), OpenOptions::new()).unwrap()
}

#[test]
fn form_gives_each_widget_its_page_index_and_rectangle_in_page_space() {
    let form = open("fields.pdf").form().unwrap();
    let name = form.fields.iter().find(|f| f.name == "name").unwrap();
    // /Rect [50 700 200 720] on a US Letter page: 72 pt from the top, y down.
    let expected = RectPt::new(Pt::new(50.0), Pt::new(72.0), Pt::new(200.0), Pt::new(92.0));
    assert_eq!(
        (name.kind, name.widgets[0].page, name.widgets[0].rect),
        (FieldKind::Text, Some(0), Some(expected))
    );
}

#[test]
fn form_lists_the_reconciliation_report_by_id() {
    let form = open("reconcile.pdf").form().unwrap();
    assert_eq!(
        (form.orphan_widgets, form.widgetless),
        (vec!["a20.0".to_owned()], vec!["f13.0".to_owned()])
    );
}

#[test]
fn tab_order_names_widgets_and_fields_by_id() {
    let order = open("tabs.pdf").tab_order(0).unwrap();
    assert_eq!((order.tabs, order.widgets.len()), (Some(Tabs::Row), 4));
    assert!(
        order
            .widgets
            .iter()
            .all(|s| s.widget.starts_with('a') && s.field.starts_with('f'))
    );
}

#[test]
fn tab_order_refuses_a_page_index_out_of_range() {
    let err = open("tabs.pdf").tab_order(6).unwrap_err();
    assert_eq!(err.code(), "page-index-out-of-range");
}

#[test]
fn a_value_that_doesnt_fit_names_the_rule_in_its_data() {
    let mut doc = open("fields.pdf");
    let id = doc
        .form()
        .unwrap()
        .fields
        .into_iter()
        .find(|f| f.name == "name")
        .unwrap()
        .id;
    let op = SetFieldValue::new(id.parse().unwrap(), FieldValue::Text("x".repeat(41)));
    let err = doc.apply(vec![Op::from(op)]).unwrap_err();
    let json = serde_json::to_value(err.diagnostic()).unwrap();
    assert_eq!(
        (err.code(), json["data"]["reason"].as_str()),
        ("field-value-invalid", Some("max-len"))
    );
}

#[test]
fn a_read_only_field_keeps_its_flags_and_its_new_value_through_a_save() {
    let mut doc = open("fields.pdf");
    let first = doc
        .form()
        .unwrap()
        .fields
        .into_iter()
        .find(|f| f.name == "person.first")
        .unwrap();
    let op = SetFieldValue::new(first.id.parse().unwrap(), FieldValue::Text("Ada".into()));
    doc.apply(vec![Op::from(op)]).unwrap();
    let saved = doc.save(&SaveOptions::new().with_mode(SaveMode::Incremental)).unwrap();
    let again = Document::open_bytes(saved.bytes, OpenOptions::new()).unwrap();
    let form = again.form().unwrap();
    let read = |name: &str| form.fields.iter().find(|f| f.name == name).unwrap();
    assert_eq!(
        (
            read("person.first").read_only,
            read("person.first").value.clone(),
            read("name").required
        ),
        (true, Some(FieldValue::Text("Ada".into())), true)
    );
}

#[test]
fn xfa_names_the_kind_and_the_packets_of_each_xfa_fixture() {
    use laminapdf::XfaKind;
    let summary = |name: &str| {
        open(name)
            .xfa()
            .unwrap()
            .map(|x| (x.kind, x.packets.iter().map(|p| p.name.clone()).collect::<Vec<_>>()))
    };
    assert_eq!(
        (
            summary("xfa-static.pdf"),
            summary("xfa-dynamic.pdf"),
            summary("fields.pdf")
        ),
        (
            Some((XfaKind::Static, vec!["template".into(), "datasets".into()])),
            Some((
                XfaKind::Dynamic,
                vec!["preamble".into(), "config".into(), "template".into(), "datasets".into(), "postamble".into()]
            )),
            None
        )
    );
}

#[test]
fn remove_xfa_through_apply_leaves_the_fields_and_undo_puts_the_part_back() {
    use laminapdf::RemoveXfa;
    let mut doc = open("xfa-static.pdf");
    let before = doc.xfa().unwrap();
    doc.apply(vec![RemoveXfa::new().into()]).unwrap();
    let removed = (
        doc.xfa().unwrap(),
        doc.form().unwrap().has_xfa,
        doc.form().unwrap().fields.len(),
    );
    doc.undo().unwrap();
    assert_eq!((removed, doc.xfa().unwrap()), ((None, false, 1), before));
}
