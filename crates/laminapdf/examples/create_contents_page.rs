//! Add a clickable contents page to a PDF, made from its bookmarks.
//!
//! The page lists the bookmarks that go to a page, two levels deep, each with the label its page
//! shows and a link to it, and gets a bookmark of its own. It's one edit, so one undo takes it
//! out again, and it saves incrementally.
//!
//! ```sh
//! cargo run -p laminapdf --example create_contents_page -- in.pdf font.ttf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{ContentsFonts, ContentsOptions, Document, Font, OpenOptions, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: create_contents_page in.pdf font.ttf [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    let font = PathBuf::from(args.next().ok_or(usage)?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let edited = add_contents(std::fs::read(input)?, std::fs::read(font)?)?;
    std::fs::write(out.join("create_contents_page.pdf"), edited)?;
    Ok(())
}

/// `bytes` with a contents page before the first page, set in the font `font` (embedded, so a
/// PDF/A or PDF/UA-1 file keeps its claim), saved incrementally.
pub(crate) fn add_contents(bytes: Vec<u8>, font: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let options = ContentsOptions::new().with_bookmark(true);
    let fonts = ContentsFonts::new().with_font(Font::from_bytes(font)?);
    doc.create_contents_page(&options, &fonts)?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
