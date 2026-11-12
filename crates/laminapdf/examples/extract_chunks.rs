//! Split a PDF into chunks for retrieval: whole paragraphs and tables up to a token budget, a new
//! chunk at each heading, each with the headings above it and the pages it came from.
//!
//! ```sh
//! cargo run -p laminapdf --example extract_chunks -- <file.pdf>
//! ```

use std::error::Error;
use std::fmt::Write as _;

use laminapdf::extract::{ChunkOptions, ExtractOptions, MarkdownOptions};
use laminapdf::{Document, OpenOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let input = std::env::args().nth(1).ok_or("usage: extract_chunks <file.pdf>")?;
    print!("{}", chunks_of(std::fs::read(input)?)?);
    Ok(())
}

/// Each chunk of `bytes`: its headings, its pages, its size and its text.
pub(crate) fn chunks_of(bytes: Vec<u8>) -> Result<String, Box<dyn Error>> {
    let doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let extraction = doc.extract(&ExtractOptions::new())?;
    // Chunks are cut from the Markdown, so each one keeps its headings and tables.
    let markdown = extraction.to_markdown(&MarkdownOptions::new());
    // A small budget, so this short file gives several chunks; 400 to 800 suits most indexes.
    let chunks = extraction.chunks(&markdown, &ChunkOptions::new().with_max_tokens(60));
    let mut out = String::new();
    for chunk in &chunks {
        let pages: Vec<String> = chunk.pages.iter().map(|p| (p + 1).to_string()).collect();
        writeln!(out, "{} under \"{}\"", chunk.id, chunk.heading_path.join(" > "))?;
        writeln!(out, "pages {}, about {} tokens", pages.join(", "), chunk.tokens_approx)?;
        writeln!(out, "{}\n", chunk.text.trim_end())?;
    }
    Ok(out)
}
