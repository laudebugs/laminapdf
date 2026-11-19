//! Form data (todo E8g) over `fixtures/forms/` (`tools/fixtures/forms.py`): each format exported,
//! imported into an emptied copy and exported again gives the same file; a fill is one undo step
//! whose undo saves the same bytes as before; a lock, an unknown name and a refused value are
//! listed while the rest fills; a reset restores defaults; and an XFDF file with a DTD is refused.
//! `fixtures/forms/data/` holds `fields.pdf`'s export in each format, which `UPDATE_GOLDEN=1`
//! rewrites, and the files a fill must refuse or partly refuse.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf::{
    Document, FieldValue, FillOptions, FormDataFormat, FormDataReport, OpenOptions, SaveMode, SaveOptions,
};
use laminapdf_test_fixtures::{fixture, fixtures_dir};

fn open(path: &str) -> Document {
    Document::open_bytes(fixture(path), OpenOptions::new()).unwrap()
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

fn fill(doc: &mut Document, json: &str) -> FormDataReport {
    doc.fill_form(json, FillOptions::default()).unwrap()
}

fn incremental_copy(doc: &mut Document) -> Vec<u8> {
    doc.save_copy(&SaveOptions::new().with_mode(SaveMode::Incremental))
        .unwrap()
        .bytes
}

#[test]
fn export_then_import_into_an_emptied_copy_should_export_the_same_file_in_each_format() {
    for format in [FormDataFormat::Json, FormDataFormat::Xfdf, FormDataFormat::Fdf] {
        let original = open("forms/fields.pdf");
        let exported = original.export_form_data(format, Some("fields.pdf")).unwrap();
        let mut copy = open("forms/fields.pdf");
        copy.reset_fields(None).unwrap();
        assert_ne!(
            copy.export_form_data(format, Some("fields.pdf")).unwrap(),
            exported,
            "{format:?}: the reset changed nothing"
        );
        let report = copy.import_form_data(&exported, FillOptions::default()).unwrap();
        assert_eq!(
            (report.unknown.len(), report.refused.len(), report.locked.len()),
            (0, 0, 0),
            "{format:?}: {report:?}"
        );
        let again = copy.export_form_data(format, Some("fields.pdf")).unwrap();
        assert_eq!(
            String::from_utf8_lossy(&again),
            String::from_utf8_lossy(&exported),
            "{format:?}"
        );
    }
}

#[test]
fn fill_form_should_be_one_undo_step_whose_undo_empties_the_overlay_and_saves_the_bytes_saved_before() {
    let mut doc = open("forms/fields.pdf");
    let before = incremental_copy(&mut doc);
    fill(
        &mut doc,
        r#"{ "fields": { "name": "Grace Hopper", "agree": false, "country": "UK" } }"#,
    );
    let filled = incremental_copy(&mut doc);
    doc.undo().unwrap();
    // Undone, the overlay is empty: nothing is left to save.
    let empty = !doc.has_unsaved_changes();
    let undone = incremental_copy(&mut doc);
    doc.redo().unwrap();
    let redone = incremental_copy(&mut doc);
    assert_eq!([empty, undone == before, redone == filled, filled != before], [true; 4]);
}

#[test]
fn fill_form_should_list_unknown_names_and_refused_values_and_fill_the_rest() {
    let mut doc = open("forms/fields.pdf");
    let report = fill(
        &mut doc,
        r#"{ "fields": { "nobody": "x", "name": ["a", "b"], "signature": "x", "code": "toolong", "address": "1 Hill Road" } }"#,
    );
    let refused: Vec<(&str, &str)> = report.refused.iter().map(|r| (r.name.as_str(), r.code)).collect();
    assert_eq!(
        (report.unknown.clone(), refused, value(&doc, "address")),
        (
            vec!["nobody".to_owned()],
            vec![
                ("name", "field-value-invalid"),
                ("signature", "field-is-signature"),
                ("code", "field-value-invalid"),
            ],
            Some(FieldValue::Text("1 Hill Road".into())),
        )
    );
}

#[test]
fn fill_form_should_skip_and_list_a_field_a_signature_locks() {
    let mut doc = open("signed/form-locked-include.pdf");
    let report = fill(
        &mut doc,
        r#"{ "fields": { "name": "Grace", "address": "1 Hill Road" } }"#,
    );
    assert_eq!(
        (report.locked, value(&doc, "address")),
        (vec!["name".to_owned()], Some(FieldValue::Text("1 Hill Road".into())))
    );
}

#[test]
fn fill_form_should_calculate_once_unless_turned_off() {
    let total = |calculate: bool| {
        let mut doc = open("forms/scripts.pdf");
        let before = value(&doc, "sum");
        doc.fill_form(
            r#"{ "fields": { "qty": "3" } }"#,
            FillOptions::default().with_calculate(calculate),
        )
        .unwrap();
        value(&doc, "sum") != before
    };
    assert_eq!([total(true), total(false)], [true, false]);
}

#[test]
fn reset_fields_should_give_each_field_its_default_or_empty_it() {
    let mut doc = open("forms/fields.pdf");
    doc.reset_fields(None).unwrap();
    let json = String::from_utf8(doc.export_form_data(FormDataFormat::Json, None).unwrap()).unwrap();
    assert!(
        json.contains(r#""name": null"#) && json.contains(r#""agree": false"#),
        "{json}"
    );
}

#[test]
fn import_form_data_should_refuse_an_xfdf_file_with_a_dtd() {
    let mut doc = open("forms/fields.pdf");
    let error = doc
        .import_form_data(&fixture("forms/data/dtd.xfdf"), FillOptions::default())
        .unwrap_err();
    assert_eq!(error.code(), "interchange-dtd");
}

/// Compares `bytes` with `fixtures/forms/data/<name>`; `UPDATE_GOLDEN=1` rewrites it.
fn check_golden(name: &str, bytes: &[u8]) {
    let golden = fixtures_dir().join("forms/data").join(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(golden.parent().unwrap()).unwrap();
        std::fs::write(&golden, bytes).unwrap();
    }
    assert!(
        std::fs::read(&golden).unwrap() == bytes,
        "{} is stale: run with UPDATE_GOLDEN=1",
        golden.display()
    );
}

#[test]
fn export_form_data_should_write_the_goldens_of_fields_pdf() {
    let doc = open("forms/fields.pdf");
    for (format, name) in [
        (FormDataFormat::Json, "fields.json"),
        (FormDataFormat::Xfdf, "fields.xfdf"),
        (FormDataFormat::Fdf, "fields.fdf"),
    ] {
        check_golden(name, &doc.export_form_data(format, Some("fields.pdf")).unwrap());
    }
}

#[test]
fn import_form_data_should_fill_what_it_knows_of_a_file_with_unknown_names() {
    let mut doc = open("forms/fields.pdf");
    let report = doc
        .import_form_data(&fixture("forms/data/unknown-names.xfdf"), FillOptions::default())
        .unwrap();
    assert_eq!(
        (report.unknown, value(&doc, "name")),
        (
            vec!["phone".to_owned(), "address.city".to_owned()],
            Some(FieldValue::Text("Katherine Johnson".into()))
        )
    );
}

#[test]
fn import_form_data_should_name_the_line_of_a_malformed_file_in_each_format() {
    let messages: Vec<String> = ["malformed.json", "malformed.xfdf", "malformed.fdf"]
        .into_iter()
        .map(|name| {
            let mut doc = open("forms/fields.pdf");
            let error = doc
                .import_form_data(&fixture(&format!("forms/data/{name}")), FillOptions::default())
                .unwrap_err();
            format!("{name} {}: {error}", error.code())
        })
        .collect();
    let found: Vec<bool> = messages
        .iter()
        .zip(["line 4", "line 5", "line 5"])
        .map(|(message, line)| message.contains("interchange-malformed") && message.contains(line))
        .collect();
    assert_eq!(found, [true; 3], "{messages:?}");
}

#[test]
fn fill_form_should_report_that_an_xfa_forms_answers_were_not_changed() {
    let mut doc = open("forms/xfa-static.pdf");
    let filled = fill(&mut doc, r#"{ "fields": { "name": "Grace" } }"#);
    let mut unchanged = open("forms/xfa-static.pdf");
    let same = fill(&mut unchanged, r#"{ "fields": { "name": "Ada" } }"#);
    assert_eq!([filled.xfa_not_updated, same.xfa_not_updated], [true, false]);
}

#[test]
fn reset_fields_should_refuse_a_push_button_which_has_no_value() {
    let mut doc = open("forms/fields.pdf");
    let submit = doc
        .form()
        .unwrap()
        .fields
        .into_iter()
        .find(|f| f.name == "submit")
        .unwrap();
    let error = doc.reset_fields(Some(&[submit.id.parse().unwrap()])).unwrap_err();
    assert_eq!(error.code(), "field-value-invalid");
}
