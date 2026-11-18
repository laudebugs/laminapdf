//! Fill in a form: list its fields with their kinds and values, give some of them new values,
//! and save the answers after the original bytes.
//!
//! Each field is named by the ID the form gives it, and each value is one op, checked against
//! the field: text no longer than its maximum length, a state its buttons have, an option from
//! its list. The widgets keep the appearance they had until an edit draws them again.
//!
//! ```sh
//! cargo run -p laminapdf --example fill_form -- form.pdf [out-dir]
//! ```

use std::error::Error;
use std::fmt::Write as _;
use std::path::PathBuf;

use laminapdf::{Document, FieldValue, Op, OpenOptions, SaveMode, SaveOptions, SetFieldValue};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: fill_form form.pdf [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let (report, filled) = fill(std::fs::read(input)?)?;
    print!("{report}");
    std::fs::write(out.join("fill_form.pdf"), filled)?;
    Ok(())
}

/// Each answer: the field's name and the value to give it.
const ANSWERS: [(&str, Answer); 3] = [
    ("name", Answer::Text("Grace Hopper")),
    ("agree", Answer::Off),
    ("country", Answer::Choose("FR")),
];

/// A value, written the way a person answers.
#[derive(Clone, Copy)]
enum Answer {
    Text(&'static str),
    Choose(&'static str),
    Off,
}

impl Answer {
    fn value(self) -> FieldValue {
        match self {
            Self::Text(text) => FieldValue::Text(text.to_owned()),
            Self::Choose(option) => FieldValue::Choice(vec![option.to_owned()]),
            Self::Off => FieldValue::Button(None),
        }
    }
}

/// The form's fields after filling in [`ANSWERS`], one line each, and the filled document saved
/// incrementally.
pub(crate) fn fill(bytes: Vec<u8>) -> Result<(String, Vec<u8>), Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let form = doc.form()?;
    let mut ops = Vec::new();
    for (name, answer) in ANSWERS {
        let field = form
            .fields
            .iter()
            .find(|f| f.name == name)
            .ok_or("the form lacks a field")?;
        ops.push(Op::from(SetFieldValue::new(field.id.parse()?, answer.value())));
    }
    doc.apply(ops)?;

    let mut report = String::new();
    for field in doc.form()?.fields {
        writeln!(
            report,
            "{} ({}): {}",
            field.name,
            field.kind.as_str(),
            shown(field.value.as_ref())
        )?;
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
