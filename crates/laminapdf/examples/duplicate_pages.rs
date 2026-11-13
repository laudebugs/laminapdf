//! Duplicate a form page, so a second copy can be filled in on its own.
//!
//! A copy shares its page's content but gets new annotations, so filling in a field on one
//! page leaves the other alone: each copied field gets a name of its own, `<name>#2` (or the
//! next free number). The edit lists each one, and a signature's widget, which isn't copied.
//!
//! ```sh
//! cargo run -p laminapdf --example duplicate_pages -- form.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{Document, DuplicatePages, FieldOutcome, OpenOptions, PagePosition, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: duplicate_pages form.pdf [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let duplicated = duplicate(std::fs::read(input)?)?;
    std::fs::write(out.join("duplicate_pages.pdf"), duplicated)?;
    Ok(())
}

/// The document with a copy of its first page added at the end.
pub(crate) fn duplicate(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let first = doc.page_info(0)?.id;
    let edited = doc.apply(vec![DuplicatePages::new(vec![first], PagePosition::End).into()])?;
    for field in &edited.fields {
        match &field.outcome {
            FieldOutcome::Renamed { name, .. } => eprintln!("the copy's field is {name}"),
            _ => eprintln!("field {} was not copied: it is a signature", field.field),
        }
    }
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
