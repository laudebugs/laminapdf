//! Extract a PDF's text for an LLM: Markdown in reading order, JSON with a box for every
//! block, and chunks sized for retrieval.
//!
//! Tables drawn with lines come out as Markdown tables, and text a reader can't see (white,
//! invisible or off the page) is left out of all three.
//!
//! ```sh
//! cargo run -p laminapdf --example extract_markdown -- <file.pdf> [out-dir]
//! # writes <out-dir>/<file>.md, <file>.json and <file>.chunks.jsonl
//! ```

use std::error::Error;
use std::path::{Path, PathBuf};

use laminapdf::extract::{ChunkOptions, ExtractOptions, MarkdownOptions, chunks_jsonl};
use laminapdf::{Document, OpenOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: extract_markdown <file.pdf> [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("document");
    write_outputs(&std::fs::read(&input)?, &out, stem)
}

/// Writes the Markdown, the JSON and the chunks of `bytes` into `out`.
pub(crate) fn write_outputs(bytes: &[u8], out: &Path, stem: &str) -> Result<(), Box<dyn Error>> {
    let doc = Document::open_bytes(bytes.to_vec(), OpenOptions::new())?;
    let extraction = doc.extract(&ExtractOptions::new())?;

    // Page markers let a reader cite the page a passage came from.
    let markdown = extraction.to_markdown(&MarkdownOptions::new().with_page_markers(true));
    std::fs::write(out.join(format!("{stem}.md")), &markdown.text)?;

    // About 400 tokens a chunk, starting a new one at each heading.
    let chunks = extraction.chunks(&markdown, &ChunkOptions::new().with_max_tokens(400));
    std::fs::write(out.join(format!("{stem}.chunks.jsonl")), chunks_jsonl(&chunks))?;

    let json = doc.extract_json(&extraction, &markdown, Some(chunks));
    std::fs::write(out.join(format!("{stem}.json")), json)?;
    Ok(())
}
