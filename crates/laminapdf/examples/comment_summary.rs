//! Print a summary of a review: a new PDF that lists every comment, with each page drawn small
//! beside its comments and their numbers marked on it.
//!
//! The summary is tagged and conforms to PDF/UA-1, so a screen reader reads it in order. The
//! reviewed file doesn't change.
//!
//! ```sh
//! cargo run -p laminapdf --example comment_summary -- reviewed.pdf font.ttf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{Document, Font, OpenOptions, SaveOptions, SummaryOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: comment_summary reviewed.pdf font.ttf [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    let font = PathBuf::from(args.next().ok_or(usage)?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let summary = summarise(std::fs::read(input)?, std::fs::read(font)?)?;
    std::fs::write(out.join("comment_summary.pdf"), summary)?;
    Ok(())
}

/// A summary of the comments on `bytes`, set in the font `font`, which the summary embeds.
pub(crate) fn summarise(bytes: Vec<u8>, font: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let mut summary = doc.comment_summary(&SummaryOptions::new(Font::from_bytes(font)?))?;
    // A page whose fonts the file doesn't embed is listed without its drawing.
    let mut warnings = summary.warnings();
    for warning in warnings.next_batch(warnings.remaining()) {
        eprintln!("{}", warning.code());
    }
    Ok(summary.save(&SaveOptions::new())?.bytes)
}
