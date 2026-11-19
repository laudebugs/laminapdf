//! Flatten a filled invoice form for sending: every answer becomes part of its page, so the copy
//! shows the same everywhere and the answers can no longer be changed.
//!
//! The fields go from the form; a signature field, or one a signature locks, would stay. Saving
//! as a full rewrite leaves no earlier revision that still holds the fields.
//!
//! ```sh
//! cargo run -p laminapdf --example flatten_form -- invoice.pdf invoice.json [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{Document, FillOptions, FlattenIntent, OpenOptions, SaveMode, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: flatten_form form.pdf answers.json [out-dir]";
    let form = PathBuf::from(args.next().ok_or(usage)?);
    let answers = std::fs::read_to_string(args.next().ok_or(usage)?)?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let flat = flatten(std::fs::read(form)?, &answers)?;
    std::fs::write(out.join("flatten_form.pdf"), flat)?;
    Ok(())
}

/// The form filled from `answers`, then every field drawn into its page, as a printer would
/// show it.
pub(crate) fn flatten(bytes: Vec<u8>, answers: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    doc.fill_form(answers, FillOptions::default())?;
    let edited = doc.flatten_form(None, FlattenIntent::Print)?;
    // A field with nothing to draw, such as a push button without an appearance, stays.
    for warning in &edited.warnings {
        eprintln!("{}", warning.code());
    }
    // A full rewrite keeps only the flattened pages, not the revisions with the fields.
    let options = SaveOptions::new().with_mode(SaveMode::FullRewrite);
    Ok(doc.save(&options)?.bytes)
}
