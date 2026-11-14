//! Make every page Letter, scaling each page's content to fit.
//!
//! A page keeps its proportions: an A4 page is scaled down to Letter's height and centred, with
//! a little room on each side. The bookmarks, links and named places that point into a page move
//! with its content, so they still land on the same words.
//!
//! ```sh
//! cargo run -p laminapdf --example resize_pages -- in.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{Document, OpenOptions, PageSize, ResizePages, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: resize_pages in.pdf [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let resized = resize(std::fs::read(input)?)?;
    std::fs::write(out.join("resize_pages.pdf"), resized)?;
    Ok(())
}

/// The document with every page on Letter paper, its content scaled to fit.
pub(crate) fn resize(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let pages = doc.page_infos()?.into_iter().map(|page| page.id).collect();
    doc.apply(vec![ResizePages::scale(pages, PageSize::LETTER).into()])?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
