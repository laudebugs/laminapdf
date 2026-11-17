//! Flatten a marked-up report for printing: every comment and mark becomes part of its page, so
//! the copy prints and shows the same everywhere and the marks can no longer be selected, edited
//! or removed as comments.
//!
//! The annotations themselves go, with their pop-ups and replies. Saving as a full rewrite leaves no earlier
//! revision that still holds them.
//!
//! ```sh
//! cargo run -p laminapdf --features extract --example flatten_comments -- marked.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{Document, FlattenIntent, FlattenScope, OpenOptions, SaveMode, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: flatten_comments marked.pdf [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let flat = flatten(std::fs::read(input)?)?;
    std::fs::write(out.join("flatten_comments.pdf"), flat)?;
    Ok(())
}

/// The document with every comment drawn into its page, as a printer would show it.
pub(crate) fn flatten(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let edited = doc.flatten_annotations(&FlattenScope::All, FlattenIntent::Print)?;
    // Marks with no picture to keep, such as a file attachment, stay as they are.
    for warning in &edited.warnings {
        eprintln!("{}", warning.code());
    }
    // A full rewrite keeps only the flattened pages, not the revision with the comments.
    let options = SaveOptions::new().with_mode(SaveMode::FullRewrite);
    Ok(doc.save(&options)?.bytes)
}
