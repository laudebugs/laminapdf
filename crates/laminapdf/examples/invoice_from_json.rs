//! An invoice from a JSON template and its data: the template lays out the heading, the text and
//! a table, and the data fills in the number, the line items and the total.
//!
//! The table runs onto a second page with its header row repeated, and the total stays with the
//! last rows. The template names its two fonts, `regular` and `bold`; their bytes come beside it.
//! The output is checked against PDF/UA-1, which the template asks for. Run it with `cargo run -p
//! laminapdf --example invoice_from_json -- invoice.json invoice-data.json regular.ttf bold.ttc`.

use std::error::Error;

use laminapdf::flow::document::{Resources, fill};
use laminapdf::{BuildOptions, Builder, Font, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    let [template, data, regular, bold] = paths.as_slice() else {
        return Err("arguments: <invoice.json> <invoice-data.json> <regular.ttf> <bold.ttc>".into());
    };
    let read = |path: &String| std::fs::read(path);
    let pdf = build(
        BuildOptions::new(),
        &read(template)?,
        &read(data)?,
        read(regular)?,
        read(bold)?,
    )?;
    std::fs::create_dir_all("target/examples-out")?;
    Ok(std::fs::write("target/examples-out/invoice_from_json.pdf", pdf)?)
}

/// The invoice's bytes. `options` can fix the file ID, the producer and the creation date.
pub(crate) fn build(
    options: BuildOptions,
    template: &[u8],
    data: &[u8],
    regular: Vec<u8>,
    bold: Vec<u8>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let document = fill(&serde_json::from_slice(template)?, &serde_json::from_slice(data)?)?;
    let options = document
        .profiles()
        .into_iter()
        .fold(options, BuildOptions::with_profile);
    let mut b = Builder::new(options);
    let resources = Resources::new()
        .with_font("regular", &b.font(Font::from_bytes(regular)?)?)
        .with_font("bold", &b.font(Font::from_collection(bold, 1)?)?);
    b.document(&document, &resources)?;
    Ok(b.finish()?.save(&SaveOptions::new())?.bytes)
}
