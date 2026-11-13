//! Record a review in a document's XMP metadata: properties of your own, beyond the title and
//! authors, which a PDF/A file keeps without losing its claim.
//!
//! Each property is named by a namespace you own and a local name. In a PDF/A file, Lamina PDF
//! also declares each new property in the metadata's extension schemas, as PDF/A requires, and
//! the edit lists what it declared.
//!
//! ```sh
//! cargo run -p laminapdf --example edit_xmp -- in.pdf <save-time> [out-dir]   # the time as a PDF date
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{Document, Op, OpenOptions, PdfDate, SaveMode, SaveOptions, SetXmpProperty, XmpValue};

/// The namespace of the review properties: a URI you own, so they can't clash with anyone else's.
const REVIEW: &str = "https://example.com/ns/review/1.0/";

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: edit_xmp in.pdf <save-time> [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    // The time of the save, which your app takes from its clock.
    let saved_at: PdfDate = args.next().ok_or(usage)?.parse()?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let edited = edit(std::fs::read(input)?, saved_at)?;
    std::fs::write(out.join("edit_xmp.pdf"), edited)?;
    Ok(())
}

/// The document with the review recorded in its XMP metadata, saved incrementally at `saved_at`.
pub(crate) fn edit(bytes: Vec<u8>, saved_at: PdfDate) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;

    let reviewers = XmpValue::Seq(vec![
        XmpValue::Text("Ada Lovelace".into()),
        XmpValue::Text("Charles Babbage".into()),
    ]);
    let edited = doc.apply(vec![
        Op::from(SetXmpProperty::new(
            REVIEW,
            "review",
            "status",
            XmpValue::Text("Approved".into()),
        )),
        Op::from(SetXmpProperty::new(REVIEW, "review", "reviewers", reviewers)),
        Op::from(SetXmpProperty::new(REVIEW, "review", "round", XmpValue::Integer(2))),
    ])?;
    for property in &edited.declared {
        eprintln!(
            "declared {}:{} as {}",
            property.prefix, property.name, property.value_type
        );
    }

    let options = SaveOptions::new()
        .with_mode(SaveMode::Incremental)
        .with_metadata_date(saved_at);
    Ok(doc.save(&options)?.bytes)
}
