//! Read a PDF's text in reading order: paragraphs, headings and table rows, a blank line between
//! blocks, and nothing a reader can't see.
//!
//! ```sh
//! cargo run -p laminapdf --example extract_text -- <file.pdf>
//! ```

use std::error::Error;

use laminapdf::extract::ExtractOptions;
use laminapdf::{Document, OpenOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let input = std::env::args().nth(1).ok_or("usage: extract_text <file.pdf>")?;
    print!("{}", text_of(std::fs::read(input)?)?);
    Ok(())
}

/// The text of `bytes`, in reading order.
pub(crate) fn text_of(bytes: Vec<u8>) -> Result<String, Box<dyn Error>> {
    let doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let extraction = doc.extract(&ExtractOptions::new())?;
    Ok(extraction.to_text().text)
}
