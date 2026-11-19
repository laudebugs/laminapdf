//! Fill an invoice form from JSON: the answers come from an order system, go into the form as one
//! change, and the form's own calculations work out each line's amount and the total.
//!
//! The answers are `{ "fields": { "<field name>": value } }`. Fields are matched by their full
//! names (`customer.name` is `name` inside `customer`). A check box takes `true` or `false`, a
//! list an array of its options. A name the form doesn't have, or a value its field refuses, is
//! listed and left out, and the rest is filled.
//!
//! ```sh
//! cargo run -p laminapdf --example fill_form_from_json -- invoice.pdf invoice.json [out-dir]
//! ```

use std::error::Error;
use std::fmt::Write as _;
use std::path::PathBuf;

use laminapdf::{Document, FieldValue, FillOptions, OpenOptions, SaveMode, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: fill_form_from_json form.pdf answers.json [out-dir]";
    let form = PathBuf::from(args.next().ok_or(usage)?);
    let answers = std::fs::read_to_string(args.next().ok_or(usage)?)?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let (report, filled) = fill(std::fs::read(form)?, &answers)?;
    print!("{report}");
    std::fs::write(out.join("fill_form_from_json.pdf"), filled)?;
    Ok(())
}

/// The form filled from `answers`: what the fill said, each field's value after it, and the
/// filled document saved after the original bytes.
pub(crate) fn fill(bytes: Vec<u8>, answers: &str) -> Result<(String, Vec<u8>), Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let filled = doc.fill_form(answers, FillOptions::default())?;

    let mut report = String::new();
    for name in &filled.unknown {
        writeln!(report, "no field named {name}")?;
    }
    for refused in &filled.refused {
        writeln!(report, "{} refused its value ({})", refused.name, refused.code)?;
    }
    for field in doc.form()?.fields {
        writeln!(report, "{}: {}", field.name, shown(field.value.as_ref()))?;
    }
    let saved = doc.save(&SaveOptions::new().with_mode(SaveMode::Incremental))?;
    Ok((report, saved.bytes))
}

fn shown(value: Option<&FieldValue>) -> String {
    match value {
        Some(FieldValue::Text(text)) => format!("{text:?}"),
        Some(FieldValue::Button(Some(state))) => format!("on ({state})"),
        Some(FieldValue::Button(None)) => "off".to_owned(),
        Some(FieldValue::Choice(options)) => options.join(", "),
        _ => "no value".to_owned(),
    }
}
