//! List a PDF's blocks with where they sit: each heading, paragraph, list item and table, its page
//! and its box in points from the page's top-left corner.
//!
//! ```sh
//! cargo run -p laminapdf --example extract_blocks -- <file.pdf>
//! ```

use std::error::Error;
use std::fmt::Write as _;

use laminapdf::extract::{Block, BlockKind, ExtractOptions};
use laminapdf::{Document, OpenOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let input = std::env::args().nth(1).ok_or("usage: extract_blocks <file.pdf>")?;
    print!("{}", blocks_of(std::fs::read(input)?)?);
    Ok(())
}

/// One line per block: its page number, its kind, its box (left, top, width, height) and its text.
pub(crate) fn blocks_of(bytes: Vec<u8>) -> Result<String, Box<dyn Error>> {
    let doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let extraction = doc.extract(&ExtractOptions::new())?;
    let mut out = String::new();
    for block in &extraction.blocks {
        let bbox = block.bbox;
        let place = [bbox.x0, bbox.y0, bbox.x1 - bbox.x0, bbox.y1 - bbox.y0].map(|v| v.get().round().to_string());
        let text = block.text.replace(['\t', '\n'], " ");
        writeln!(
            out,
            "page {} {} [{}] {text}",
            block.page + 1,
            kind(block),
            place.join(" ")
        )?;
    }
    Ok(out)
}

/// The block's kind as the JSON names it, with a heading's level.
fn kind(block: &Block) -> String {
    match &block.kind {
        BlockKind::Heading { level } => format!("heading {level}"),
        BlockKind::Paragraph => "paragraph".into(),
        BlockKind::ListItem { .. } => "list item".into(),
        BlockKind::Caption => "caption".into(),
        BlockKind::Footnote => "footnote".into(),
        BlockKind::Table { .. } => "table".into(),
        _ => "other".into(),
    }
}
