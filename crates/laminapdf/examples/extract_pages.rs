//! Make a new PDF from some pages of another.
//!
//! The new file holds only what its pages use, and its pages keep the labels they had (`iii`,
//! `1`, `2` rather than 1, 2, 3).
//!
//! ```sh
//! cargo run -p laminapdf --example extract_pages -- book.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{Document, OpenOptions, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: extract_pages book.pdf [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let part = extract(std::fs::read(input)?)?;
    std::fs::write(out.join("extract_pages.pdf"), part)?;
    Ok(())
}

/// A new document of the third to the fifth page of `bytes`.
pub(crate) fn extract(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let pages: Vec<_> = doc.page_infos()?.into_iter().skip(2).take(3).map(|p| p.id).collect();
    let mut part = doc.extract_pages(&pages)?;
    Ok(part.save(&SaveOptions::new())?.bytes)
}
