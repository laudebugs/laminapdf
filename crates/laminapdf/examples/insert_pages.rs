//! Copy a page from another PDF into this one.
//!
//! The copy keeps its annotations and form fields. A field whose name the document already uses
//! takes the next free `<name>#2`, and a link to a page that didn't come along is removed; the
//! edit lists both.
//!
//! ```sh
//! cargo run -p laminapdf --example insert_pages -- form.pdf other.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{Document, FieldOutcome, InsertPagesFrom, OpenOptions, PagePosition, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: insert_pages form.pdf other.pdf [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    let other = PathBuf::from(args.next().ok_or(usage)?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let combined = insert(std::fs::read(input)?, std::fs::read(other)?)?;
    std::fs::write(out.join("insert_pages.pdf"), combined)?;
    Ok(())
}

/// `bytes` with the third page of `other` added at the end.
pub(crate) fn insert(bytes: Vec<u8>, other: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let source = doc.add_source(other, None)?;
    let third = doc
        .source_pages(&source)?
        .into_iter()
        .nth(2)
        .ok_or("the other file has no third page")?;
    let insert = InsertPagesFrom::new(source, vec![third], PagePosition::End);
    let edited = doc.apply(vec![insert.into()])?;
    for field in &edited.fields {
        if let FieldOutcome::Renamed { name, .. } = &field.outcome {
            eprintln!("a copied field was renamed {name}");
        }
    }
    eprintln!("{} links removed", edited.links_removed.len());
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
