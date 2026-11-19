//! `Document::form` when `/AcroForm /Fields` lists a widget instead of its field (bug
//! `docs/bugs/forms-widget-in-fields`, pdf.js's `opt_demo.pdf`): the widget stands for the field
//! its `/Parent` chain reaches, read once, and whatever is still left out is reported.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf_model::{Document, FieldKind, FieldValue, Form, OpenOptions};
use laminapdf_test_fixtures::{classic_with_root, fixture};

/// The form of a one-page file whose `/Fields` is `fields`, whose page lists `annots` and whose
/// objects from 5 on are `objects`.
fn form_of(fields: &str, annots: &str, objects: &[&str]) -> Form {
    let page = format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [{annots}] >>");
    let acro = format!("<< /Fields [{fields}] >>");
    let mut all = vec![
        "<< /Type /Catalog /Pages 2 0 R /AcroForm 4 0 R >>",
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        page.as_str(),
        acro.as_str(),
    ];
    all.extend_from_slice(objects);
    Document::open(classic_with_root(&all, ""), OpenOptions::default())
        .unwrap()
        .form()
        .unwrap()
}

/// A widget on page 3 whose `/Parent` is object `parent`.
fn widget(parent: u32) -> String {
    format!("<< /Type /Annot /Subtype /Widget /Rect [0 0 10 10] /P 3 0 R /Parent {parent} 0 R >>")
}

/// A field's name, kind, value and widgets.
type FieldSummary = (String, FieldKind, Option<FieldValue>, Vec<String>);

/// What the reconciliation reports: orphan widgets (with their fields), untyped and widgetless
/// fields.
type Reported = (Vec<(String, Option<String>)>, Vec<String>, Vec<String>);

/// Each field's name, kind, value and widgets.
fn fields(form: &Form) -> Vec<FieldSummary> {
    form.fields
        .iter()
        .map(|f| {
            (
                f.name.clone(),
                f.kind,
                f.value.clone(),
                f.widgets.iter().map(|w| w.id.to_string()).collect(),
            )
        })
        .collect()
}

/// The reconciliation's reports.
fn reported(form: &Form) -> Reported {
    (
        form.orphan_widgets
            .iter()
            .map(|o| (o.widget.to_string(), o.field.as_ref().map(ToString::to_string)))
            .collect(),
        form.untyped.iter().map(ToString::to_string).collect(),
        form.widgetless.iter().map(ToString::to_string).collect(),
    )
}

/// `veg`, a list box set to Potato, with `widgets`.
fn potato(widgets: &[&str]) -> Vec<FieldSummary> {
    vec![(
        "veg".to_owned(),
        FieldKind::ListBox,
        Some(FieldValue::Choice(vec!["Potato".to_owned()])),
        widgets.iter().map(|w| (*w).to_owned()).collect(),
    )]
}

const VEG: &str = "<< /FT /Ch /T (veg) /V (Potato) /Opt [(Carrot) (Potato)] >>";

#[test]
fn a_widget_listed_in_fields_should_be_read_as_its_parent_field() {
    let form = form_of("6 0 R", "6 0 R", &[VEG, &widget(5)]);
    assert_eq!(fields(&form), potato(&["a6.0"]));
}

#[test]
fn a_widget_listed_in_fields_should_leave_nothing_to_report() {
    let form = form_of("6 0 R", "6 0 R", &[VEG, &widget(5)]);
    assert_eq!(reported(&form), (vec![], vec![], vec![]));
}

#[test]
fn two_widgets_listed_in_fields_should_be_one_field_with_both() {
    let form = form_of("6 0 R 7 0 R", "6 0 R 7 0 R", &[VEG, &widget(5), &widget(5)]);
    assert_eq!(fields(&form), potato(&["a6.0", "a7.0"]));
}

#[test]
fn a_field_listed_with_its_widget_should_be_read_once() {
    let kids = "<< /FT /Ch /T (veg) /V (Potato) /Kids [6 0 R] >>";
    let listed = [
        form_of("5 0 R 6 0 R", "6 0 R", &[kids, &widget(5)]),
        form_of("6 0 R 5 0 R", "6 0 R", &[kids, &widget(5)]),
        form_of("6 0 R 5 0 R", "6 0 R", &[VEG, &widget(5)]),
    ];
    assert_eq!(listed.map(|f| fields(&f)), [(); 3].map(|()| potato(&["a6.0"])));
}

#[test]
fn a_widget_two_levels_down_should_inherit_and_be_named_from_the_root() {
    let root = "<< /FT /Ch /T (order) >>";
    let veg = "<< /T (veg) /Parent 5 0 R /V (Potato) >>";
    let form = form_of("6 0 R", "6 0 R", &[root, &widget(7), veg]);
    assert_eq!(
        fields(&form),
        [(
            "order.veg".to_owned(),
            FieldKind::ListBox,
            Some(FieldValue::Choice(vec!["Potato".to_owned()])),
            vec!["a6.0".to_owned()]
        )]
    );
}

#[test]
fn a_widget_whose_field_the_tree_also_reaches_should_be_read_once() {
    let root = "<< /FT /Ch /T (order) /Kids [7 0 R] >>";
    let veg = "<< /T (veg) /Parent 5 0 R /V (Potato) >>";
    let orders = [
        form_of("5 0 R 6 0 R", "6 0 R", &[root, &widget(7), veg]),
        form_of("6 0 R 5 0 R", "6 0 R", &[root, &widget(7), veg]),
    ];
    let names = orders.map(|f| {
        f.fields
            .iter()
            .map(|f| (f.name.clone(), f.widgets.len()))
            .collect::<Vec<_>>()
    });
    assert_eq!(names, [(); 2].map(|()| vec![("order.veg".to_owned(), 1)]));
}

#[test]
fn a_parent_loop_should_end_the_climb_and_keep_the_field() {
    let back_to_widget = "<< /FT /Ch /T (veg) /V (Potato) /Parent 6 0 R >>";
    let field_loop = "<< /FT /Ch /T (veg) /V (Potato) /Parent 7 0 R >>";
    let order = "<< /T (order) /Parent 5 0 R >>";
    let names = [
        form_of("6 0 R", "6 0 R", &[back_to_widget, &widget(5)]),
        form_of("6 0 R", "6 0 R", &[field_loop, &widget(5), order]),
        form_of("6 0 R", "6 0 R", &["<< /FT /Ch /T (veg) >>", &widget(6)]),
    ]
    .map(|f| f.fields.iter().map(|f| f.name.clone()).collect::<Vec<_>>());
    // A widget that is its own parent stands for no field: it is the untyped node it was.
    assert_eq!(names, [vec!["veg".to_owned()], vec!["order.veg".to_owned()], vec![]]);
}

#[test]
fn a_widget_whose_chain_has_no_type_should_report_its_field_untyped() {
    let form = form_of("6 0 R", "6 0 R", &["<< /T (veg) >>", &widget(5)]);
    assert_eq!(
        (form.fields.len(), reported(&form)),
        (0, (vec![], vec!["f5.0".to_owned()], vec![]))
    );
}

#[test]
fn a_widget_without_a_parent_should_stay_an_untyped_node() {
    let lone = "<< /Type /Annot /Subtype /Widget /Rect [0 0 10 10] /P 3 0 R >>";
    let form = form_of("5 0 R", "5 0 R", &[lone]);
    assert_eq!(
        (form.fields.len(), reported(&form)),
        (0, (vec![], vec!["f5.0".to_owned()], vec![]))
    );
}

#[test]
fn a_widget_whose_parent_has_field_kids_should_be_reported_an_orphan() {
    let parent = "<< /FT /Tx /T (order) /Kids [7 0 R] >>";
    let kid = "<< /T (note) /Parent 5 0 R /V (x) /Type /Annot /Subtype /Widget /Rect [0 0 9 9] /P 3 0 R >>";
    let form = form_of("6 0 R", "6 0 R 7 0 R", &[parent, &widget(5), kid]);
    assert_eq!(
        (
            form.fields.iter().map(|f| f.name.clone()).collect::<Vec<_>>(),
            reported(&form)
        ),
        (
            vec!["order.note".to_owned()],
            (vec![("a6.0".to_owned(), Some("f5.0".to_owned()))], vec![], vec![])
        )
    );
}

#[test]
fn a_widget_whose_parent_is_no_field_should_stay_an_untyped_node() {
    // The page tree (object 2) is no field: its kids are pages, not fields to walk.
    let form = form_of("5 0 R", "5 0 R", &[&widget(2)]);
    assert_eq!(
        (form.fields.len(), reported(&form)),
        (0, (vec![], vec!["f5.0".to_owned()], vec![]))
    );
}

#[test]
fn a_listed_widget_with_a_type_of_its_own_should_stay_that_field() {
    let typed = "<< /FT /Tx /V (hi) /Type /Annot /Subtype /Widget /Rect [0 0 10 10] /P 3 0 R /Parent 5 0 R >>";
    let form = form_of("6 0 R", "6 0 R", &[VEG, typed]);
    assert_eq!(
        fields(&form),
        [(
            String::new(),
            FieldKind::Text,
            Some(FieldValue::Text("hi".to_owned())),
            vec!["a6.0".to_owned()]
        )]
    );
}

#[test]
fn a_listed_widget_with_a_name_of_its_own_should_stay_that_field() {
    let named =
        "<< /T (veg) /FT /Ch /V (Potato) /Type /Annot /Subtype /Widget /Rect [0 0 10 10] /P 3 0 R /Parent 5 0 R >>";
    let form = form_of("6 0 R", "6 0 R", &["<< /T (order) /FT /Ch >>", named]);
    assert_eq!(fields(&form), potato(&["a6.0"]));
}

#[test]
fn a_listed_widget_should_join_the_widgets_its_field_lists() {
    let veg = "<< /FT /Ch /T (veg) /V (Potato) /Kids [7 0 R] >>";
    let form = form_of("6 0 R", "6 0 R 7 0 R", &[veg, &widget(5), &widget(5)]);
    assert_eq!(fields(&form), potato(&["a7.0", "a6.0"]));
}

#[test]
fn the_fixture_should_read_veg_as_opt_demo_does() {
    let doc = Document::open(fixture("forms/widget-entries.pdf"), OpenOptions::default()).unwrap();
    let form = doc.form().unwrap();
    let veg = form.field_named("veg").unwrap();
    assert_eq!(
        (
            veg.kind,
            veg.value.clone(),
            veg.export_of_state("1"),
            veg.widgets.len(),
            reported(&form)
        ),
        (
            FieldKind::RadioGroup,
            Some(FieldValue::Button(Some("1".to_owned()))),
            Some("Potato"),
            2,
            (vec![], vec![], vec![])
        )
    );
}
