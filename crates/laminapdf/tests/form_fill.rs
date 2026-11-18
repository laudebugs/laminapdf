//! Filling form fields draws their widgets again (todo E8d) over `fixtures/forms/`
//! (`tools/fixtures/forms.py`): every layout rule of `text-layout.pdf` filled into a byte golden,
//! `/NeedAppearances` left alone, nothing written but the changed field's widgets and the fonts
//! it adds, and on a PDF/A file the value drawn in the embedded font given.
#![expect(clippy::unwrap_used, reason = "tests: a panic is a test failure")]

use laminapdf::{BlobId, Document, FieldValue, Op, OpenOptions, SaveMode, SaveOptions, SetFieldValue, Warning};
use laminapdf_cos::{ObjRef, Object};
use laminapdf_test_fixtures::{fixture, fixtures_dir};

fn open(name: &str) -> Document {
    Document::open_bytes(fixture(&format!("forms/{name}")), OpenOptions::new()).unwrap()
}

fn id(doc: &Document, name: &str) -> String {
    doc.form()
        .unwrap()
        .fields
        .into_iter()
        .find(|f| f.name == name)
        .unwrap()
        .id
}

fn fill(doc: &mut Document, name: &str, value: FieldValue, fonts: &[BlobId]) -> laminapdf::Result<()> {
    let op = SetFieldValue::new(id(doc, name).parse().unwrap(), value).with_fonts(fonts.to_vec());
    doc.apply(vec![Op::from(op)]).map(drop)
}

fn text(s: &str) -> FieldValue {
    FieldValue::Text(s.to_owned())
}

/// The incremental save's bytes, leaving the document's changes pending.
fn incremental_copy(doc: &mut Document) -> Vec<u8> {
    doc.save_copy(&SaveOptions::new().with_mode(SaveMode::Incremental))
        .unwrap()
        .bytes
}

fn incremental(doc: &mut Document) -> Vec<u8> {
    doc.save(&SaveOptions::new().with_mode(SaveMode::Incremental))
        .unwrap()
        .bytes
}

/// `text-layout.pdf` with every field filled: what the golden holds.
fn filled_layout() -> Vec<u8> {
    let mut doc = open("text-layout.pdf");
    let noto = doc.add_blob(fixture("fonts/NotoSans-Regular.ttf"));
    let values = [
        ("auto", "Auto-sized to the box"),
        (
            "multi",
            "A longer answer that wraps over several lines and shrinks until every line fits the box.",
        ),
        ("comb", "AB12CD34"),
        ("left", "Left"),
        ("center", "Centred"),
        ("right", "Right"),
        ("r90", "Turned 90"),
        ("r180", "Turned 180"),
        ("r270", "Turned 270"),
        ("secret", "hunter2"),
        ("nohelv", "Helvetica, which /DR lacks"),
    ];
    for (name, value) in values {
        fill(&mut doc, name, text(value), &[]).unwrap();
    }
    fill(&mut doc, "latin", text("Łódź, Gdańsk"), &[noto]).unwrap();
    fill(&mut doc, "pick", FieldValue::Choice(vec!["Banana".into()]), &[]).unwrap();
    fill(&mut doc, "tick", FieldValue::Button(Some("Yes".into())), &[]).unwrap();
    fill(
        &mut doc,
        "rows",
        FieldValue::Choice(vec!["Two".into(), "Four".into()]),
        &[],
    )
    .unwrap();
    incremental(&mut doc)
}

#[test]
fn every_layout_rule_fills_into_the_golden() {
    check_golden("text-layout-filled.pdf", &filled_layout());
}

#[test]
fn the_same_fill_writes_the_same_bytes() {
    assert_eq!(filled_layout(), filled_layout());
}

/// The object `num` of `bytes`, resolved.
fn object(bytes: &[u8], num: u32) -> Object {
    let doc = laminapdf_model::Document::open(bytes.to_vec(), OpenOptions::new()).unwrap();
    let reference = Object::Ref(ObjRef::new(num, 0));
    doc.deref(&reference).unwrap().into_owned()
}

#[test]
fn a_filled_value_is_drawn_in_the_widgets_appearance() {
    let mut doc = open("text-layout.pdf");
    fill(&mut doc, "auto", text("Visible"), &[]).unwrap();
    let saved = incremental(&mut doc);
    let reopened = laminapdf_model::Document::open(saved, OpenOptions::new()).unwrap();
    // `auto` is object 10, a field merged with its widget.
    let reference = Object::Ref(ObjRef::new(10, 0));
    let widget = reopened.deref(&reference).unwrap();
    let normal = widget
        .as_dict()
        .and_then(|d| d.get("AP"))
        .and_then(Object::as_dict)
        .and_then(|ap| ap.get("N"))
        .cloned()
        .unwrap();
    let Object::Stream(form) = &*reopened.deref(&normal).unwrap() else {
        panic!("no appearance stream");
    };
    let content = String::from_utf8_lossy(&reopened.decode_stream(form).unwrap()).into_owned();
    assert!(
        content.starts_with("/Tx BMC") && content.contains("(Visible) Tj"),
        "{content}"
    );
}

#[test]
fn a_fallback_font_is_embedded_and_added_to_the_forms_resources() {
    let mut doc = open("text-layout.pdf");
    let noto = doc.add_blob(fixture("fonts/NotoSans-Regular.ttf"));
    fill(&mut doc, "latin", text("Łódź"), &[noto]).unwrap();
    let saved = incremental(&mut doc);
    let text = String::from_utf8_lossy(&saved);
    assert!(
        text.contains("/FontFile2") && text.contains("/E0"),
        "the subset and its /DR name"
    );
}

#[test]
fn opening_a_form_that_needs_appearances_and_saving_writes_nothing_new() {
    let base = fixture("forms/need-appearances.pdf");
    let mut doc = Document::open_bytes(base.clone(), OpenOptions::new()).unwrap();
    assert_eq!(incremental(&mut doc), base);
}

#[test]
fn a_fill_draws_only_its_field_and_leaves_need_appearances_set() {
    let mut doc = open("need-appearances.pdf");
    fill(&mut doc, "first", text("Grace"), &[]).unwrap();
    let saved = incremental(&mut doc);
    let base = fixture("forms/need-appearances.pdf");
    // `first` (object 10) changes; `second` (11) and the form (5, which holds the flag) don't.
    assert_eq!(
        [10, 11, 5].map(|num| object(&base, num) == object(&saved, num)),
        [false, true, true]
    );
}

/// Compares `bytes` with the golden `name` in `fixtures/golden/forms/`; `UPDATE_GOLDEN=1` rewrites
/// it.
fn check_golden(name: &str, bytes: &[u8]) {
    let golden = fixtures_dir().join("golden/forms").join(name);
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

/// The PDF/A form filled in Noto Sans: a golden that veraPDF checks (`cargo xtask verapdf --dir
/// fixtures/golden/forms`).
#[test]
fn a_pdfa_fill_with_a_font_draws_it_embedded_and_warns_of_nothing() {
    let mut doc = open("pdfa-form.pdf");
    let noto = doc.add_blob(fixture("fonts/NotoSans-Regular.ttf"));
    fill(&mut doc, "name", text("Ada Lovelace"), &[noto]).unwrap();
    check_golden("pdfa-form-filled.pdf", &incremental_copy(&mut doc));
    let plan = doc.plan(&SaveOptions::new()).unwrap();
    assert!(
        !plan
            .conformance_warnings
            .iter()
            .any(|w: &Warning| w.code() == "pdfa-font-not-embedded"),
        "{:?}",
        plan.conformance_warnings
    );
}

#[test]
fn a_pdfa_fill_without_a_font_warns_that_its_font_is_not_embedded() {
    let mut doc = open("pdfa-form.pdf");
    fill(&mut doc, "name", text("Ada Lovelace"), &[]).unwrap();
    let plan = doc.plan(&SaveOptions::new()).unwrap();
    assert!(
        plan.conformance_warnings
            .iter()
            .any(|w| w.code() == "pdfa-font-not-embedded")
    );
}
