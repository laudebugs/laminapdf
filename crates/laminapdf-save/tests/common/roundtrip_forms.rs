//! The form cases of the round-trip suite (todo E8c, E.06): `fixtures/forms/fields.pdf` with five
//! fields filled (text, a check box turned off, a radio group, a combo box, and a read-only text
//! field), saved incrementally and as a full rewrite. Lamina reopens each with those values and
//! the fields' required and read-only flags; the tools read the same from the widgets
//! (`fields`: `[name, value, read-only, required]`, sorted by name).
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
use laminapdf_ops::{Document, Op, Origin, SetFieldValue};
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
    let form = doc.form().unwrap();
    let mut out: Vec<Value> = answers()
        .iter()
        .filter_map(|(name, _)| form.field_named(name))
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
