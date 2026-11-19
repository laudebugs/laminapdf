//! The form cases of the round-trip suite (todo E8c, E.06): `fixtures/forms/fields.pdf` with five
//! fields filled (text, a check box turned off, a radio group, a combo box, and a read-only text
//! field), saved incrementally and as a full rewrite. Lamina reopens each with those values and
//! the fields' required and read-only flags; the tools read the same from the widgets
//! (`fields`: `[name, value, read-only, required]`, sorted by name).
//!
//! Then `fixtures/forms/scripts.pdf`, whose fields format, validate and calculate (todo E8f,
//! E.05, E.07, E.08, E.10): a quantity changed, which recalculates two totals, a date filled and
//! a field hidden. Lamina reopens each save with those values, the totals, the Hidden flag and
//! every field's helper calls still read; the tools read the values.
//!
//! Then form data (todo E8g, E.11, E.12, E.13): `fields.pdf` filled from
//! `fixtures/forms/data/unknown-names.xfdf`, whose unknown names are left out, and `country`
//! reset, each one transaction. Lamina reopens each save with those values; the tools read them.
//!
//! Then a flattened form (todo E8p, E.14): `fields.pdf` with the five fields filled, then
//! flattened for printing. Lamina reopens each save without them, the fields left as they were
//! and the page drawing each flattened widget; the tools find none of the five (`flattened`) and
//! read the fields left.
#![allow(
    unreachable_pub,
    reason = "shared by the tests and the `roundtrip` example, as `tests/common` helpers are"
)]
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test support: a panic is a test failure"
)]

use std::path::Path;

use laminapdf_cos::OpenOptions;
use laminapdf_model::FieldValue;
use laminapdf_ops::{Document, FlattenForm, FlattenIntent, Op, Origin, SetFieldValue};
use laminapdf_save::{Save as _, SaveMode, SaveOptions};
use serde_json::{Value, json};

use super::roundtrip::{Suite, facts};

/// The fields the cases fill, and the value each is given.
fn answers() -> [(&'static str, FieldValue); 5] {
    [
        ("name", FieldValue::Text("Grace Hopper".into())),
        ("agree", FieldValue::Button(None)),
        ("colour", FieldValue::Button(Some("green".into()))),
        ("country", FieldValue::Choice(vec!["FR".into()])),
        ("person.first", FieldValue::Text("Ada".into())),
    ]
}

/// The filled fields as the tools read them, sorted by name: `[name, value, read-only,
/// required]`, a button's value its state (`Off` when off) and a choice's its one option.
pub fn field_facts(doc: &laminapdf_model::Document) -> Vec<Value> {
    facts_of(doc, &answers().map(|(name, _)| name))
}

/// [`field_facts`] for the fields `names`.
fn facts_of(doc: &laminapdf_model::Document, names: &[&str]) -> Vec<Value> {
    let form = doc.form().unwrap();
    let mut out: Vec<Value> = names
        .iter()
        .filter_map(|name| form.field_named(name))
        .map(|f| {
            let value = match &f.value {
                Some(FieldValue::Text(t)) => t.clone(),
                Some(FieldValue::Button(state)) => state.clone().unwrap_or_else(|| "Off".to_owned()),
                Some(FieldValue::Choice(c)) => c.join(", "),
                _ => String::new(),
            };
            json!([f.name, value, f.read_only(), f.required()])
        })
        .collect();
    out.sort_by_key(|v| v[0].to_string());
    out
}

pub fn filled(root: &Path, suite: &mut Suite) {
    let source = "forms/fields.pdf";
    let bytes = std::fs::read(root.join("fixtures").join(source)).unwrap();
    for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
        let id = format!("forms-fields-{mode:?}").to_lowercase();
        let mut doc = Document::open(bytes.clone(), OpenOptions::default())
            .unwrap()
            .with_appearances(laminapdf_build::AnnotationAppearances);
        let form = doc.form().unwrap();
        let ops: Vec<Op> = answers()
            .into_iter()
            .map(|(name, value)| SetFieldValue::new(form.field_named(name).unwrap().id.clone(), value).into())
            .collect();
        doc.apply_all(Origin::Local, ops).unwrap();
        let (pages, rotations, info, labels) = facts(&doc);
        let fields = field_facts(&doc);
        let mut output = Vec::new();
        let report = doc.save(&mut output, &SaveOptions::new().with_mode(mode)).unwrap();
        suite.saved += 1;
        let reopened = laminapdf_model::Document::open(output.clone(), OpenOptions::default()).unwrap();
        if field_facts(&reopened) != fields
            || facts(&reopened) != (pages, rotations.clone(), info.clone(), labels)
            || reopened.was_repaired()
        {
            suite.failures.push(format!(
                "{id}: lamina reopened fields {:?} (repaired {}), expected {fields:?}",
                field_facts(&reopened),
                reopened.was_repaired()
            ));
        }
        let file = format!("{id}.pdf");
        suite.manifest.push(json!({
            "file": &file,
            "source": source,
            "mode": format!("{:?}", report.mode()),
            "pages": pages,
            "rotations": rotations,
            "title": info[0],
            "info": null,
            "encrypted": false,
            "direct_info": false,
            "invalidated": 0,
            "fields": fields,
        }));
        suite.outputs.push((file, output));
    }
}

/// The scripted form's fields the case changes or calculates.
const SCRIPTED: [&str; 4] = ["due", "qty", "sum", "total"];

/// What the scripted case must find after a save: the values, the Hidden flag of `price`'s
/// widget, and each field's helper calls read again.
fn scripted_facts(doc: &laminapdf_model::Document) -> (Vec<Value>, bool, Vec<bool>) {
    let form = doc.form().unwrap();
    let hidden = form.field_named("price").unwrap().widgets[0].flags.0 & 2 != 0;
    let read = ["due", "price", "total"]
        .iter()
        .map(|name| {
            let b = doc.field_behaviour(form.field_named(name).unwrap()).unwrap();
            b.format.is_some() || b.validate.is_some() || b.calculate.is_some()
        })
        .collect();
    (facts_of(doc, &SCRIPTED), hidden, read)
}

pub fn scripted(root: &Path, suite: &mut Suite) {
    let source = "forms/scripts.pdf";
    let bytes = std::fs::read(root.join("fixtures").join(source)).unwrap();
    for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
        let id = format!("forms-scripts-{mode:?}").to_lowercase();
        let mut doc = Document::open(bytes.clone(), OpenOptions::default())
            .unwrap()
            .with_appearances(laminapdf_build::AnnotationAppearances);
        let form = doc.form().unwrap();
        let id_of = |name: &str| form.field_named(name).unwrap().id.clone();
        let ops: Vec<Op> = vec![
            SetFieldValue::new(id_of("qty"), FieldValue::Text("4".into())).into(),
            SetFieldValue::new(id_of("due"), FieldValue::Text("1/2/2027".into())).into(),
            laminapdf_ops::SetFieldHidden::new(id_of("price"), true).into(),
        ];
        doc.apply_all(Origin::Local, ops).unwrap();
        let (pages, rotations, info, _) = facts(&doc);
        let expected = scripted_facts(&doc);
        let mut output = Vec::new();
        let report = doc.save(&mut output, &SaveOptions::new().with_mode(mode)).unwrap();
        suite.saved += 1;
        let reopened = laminapdf_model::Document::open(output.clone(), OpenOptions::default()).unwrap();
        let found = scripted_facts(&reopened);
        if found != expected || !expected.1 || reopened.was_repaired() {
            suite.failures.push(format!(
                "{id}: lamina reopened {found:?} (repaired {}), expected {expected:?}",
                reopened.was_repaired()
            ));
        }
        let file = format!("{id}.pdf");
        suite.manifest.push(json!({
            "file": &file,
            "source": source,
            "mode": format!("{:?}", report.mode()),
            "pages": pages,
            "rotations": rotations,
            "title": info[0],
            "info": null,
            "encrypted": false,
            "direct_info": false,
            "invalidated": 0,
            "fields": expected.0,
        }));
        suite.outputs.push((file, output));
    }
}

/// The fields the form data case fills or resets.
const FROM_DATA: [&str; 2] = ["country", "name"];

pub fn from_data(root: &Path, suite: &mut Suite) {
    let source = "forms/fields.pdf";
    let bytes = std::fs::read(root.join("fixtures").join(source)).unwrap();
    let xfdf = std::fs::read(root.join("fixtures/forms/data/unknown-names.xfdf")).unwrap();
    let data = laminapdf_interchange::read_form_data(&xfdf, &laminapdf_syntax::Limits::default()).unwrap();
    for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
        let id = format!("forms-data-{mode:?}").to_lowercase();
        let mut doc = Document::open(bytes.clone(), OpenOptions::default())
            .unwrap()
            .with_appearances(laminapdf_build::AnnotationAppearances);
        let form = doc.form().unwrap();
        let plan = laminapdf_interchange::plan_form_data(&form, &data);
        doc.fill(plan.values, true).unwrap();
        let country = form.field_named("country").unwrap().id.clone();
        let reset = laminapdf_interchange::reset_values(&doc.form().unwrap(), Some(&[country])).unwrap();
        doc.fill(reset, true).unwrap();
        let (pages, rotations, info, _) = facts(&doc);
        let expected = facts_of(&doc, &FROM_DATA);
        let mut output = Vec::new();
        let report = doc.save(&mut output, &SaveOptions::new().with_mode(mode)).unwrap();
        suite.saved += 1;
        let reopened = laminapdf_model::Document::open(output.clone(), OpenOptions::default()).unwrap();
        let found = facts_of(&reopened, &FROM_DATA);
        // The XFDF's name, and `country` emptied: it has no default.
        let filled = expected.first().is_some_and(|c| c[1] == "")
            && expected.get(1).is_some_and(|n| n[1] == "Katherine Johnson");
        if found != expected || !filled || plan.unknown.len() != 2 || reopened.was_repaired() {
            suite.failures.push(format!(
                "{id}: lamina reopened {found:?} (repaired {}), expected {expected:?}; unknown {:?}",
                reopened.was_repaired(),
                plan.unknown
            ));
        }
        let file = format!("{id}.pdf");
        suite.manifest.push(json!({
            "file": &file,
            "source": source,
            "mode": format!("{:?}", report.mode()),
            "pages": pages,
            "rotations": rotations,
            "title": info[0],
            "info": null,
            "encrypted": false,
            "direct_info": false,
            "invalidated": 0,
            "fields": expected,
        }));
        suite.outputs.push((file, output));
    }
}

/// Fields the flatten case leaves, which keep their values: `person.last` is a kid whose sibling
/// goes. Text fields only, which every tool reads alike (PDFium reads a list's first choice, and
/// pdf.js a radio group's export value).
const KEPT: [&str; 2] = ["address", "person.last"];

pub fn flattened(root: &Path, suite: &mut Suite) {
    let source = "forms/fields.pdf";
    let bytes = std::fs::read(root.join("fixtures").join(source)).unwrap();
    let names = answers().map(|(name, _)| name);
    for mode in [SaveMode::Incremental, SaveMode::FullRewrite] {
        let id = format!("forms-flattened-{mode:?}").to_lowercase();
        let mut doc = Document::open(bytes.clone(), OpenOptions::default())
            .unwrap()
            .with_appearances(laminapdf_build::AnnotationAppearances);
        let form = doc.form().unwrap();
        let ids: Vec<_> = names.iter().map(|n| form.field_named(n).unwrap().id.clone()).collect();
        let widgets: usize = names.iter().map(|n| form.field_named(n).unwrap().widgets.len()).sum();
        let fills: Vec<Op> = answers()
            .into_iter()
            .map(|(name, value)| SetFieldValue::new(form.field_named(name).unwrap().id.clone(), value).into())
            .collect();
        doc.apply_all(Origin::Local, fills).unwrap();
        doc.apply_all(Origin::Local, [FlattenForm::new(ids, FlattenIntent::Print).into()])
            .unwrap();
        let (pages, rotations, info, labels) = facts(&doc);
        let kept = facts_of(&doc, &KEPT);
        let mut output = Vec::new();
        let report = doc.save(&mut output, &SaveOptions::new().with_mode(mode)).unwrap();
        suite.saved += 1;
        let reopened = laminapdf_model::Document::open(output.clone(), OpenOptions::default()).unwrap();
        let draws: usize = reopened
            .pages()
            .unwrap()
            .iter()
            .map(|p| {
                let content = reopened.cos().page_content(p.cos()).unwrap();
                String::from_utf8_lossy(&content).matches(" Do").count()
            })
            .sum();
        if !facts_of(&reopened, &names).is_empty()
            || facts_of(&reopened, &KEPT) != kept
            || draws < widgets
            || facts(&reopened) != (pages, rotations.clone(), info.clone(), labels)
            || reopened.was_repaired()
        {
            suite.failures.push(format!(
                "{id}: lamina reopened flattened fields {:?}, kept {:?} with {draws} drawings (repaired {}), expected none, {kept:?} and {widgets}",
                facts_of(&reopened, &names),
                facts_of(&reopened, &KEPT),
                reopened.was_repaired()
            ));
        }
        let file = format!("{id}.pdf");
        suite.manifest.push(json!({
            "file": &file,
            "source": source,
            "mode": format!("{:?}", report.mode()),
            "pages": pages,
            "rotations": rotations,
            "title": info[0],
            "info": null,
            "encrypted": false,
            "direct_info": false,
            "invalidated": 0,
            "fields": kept,
            "flattened": names,
        }));
        suite.outputs.push((file, output));
    }
}
