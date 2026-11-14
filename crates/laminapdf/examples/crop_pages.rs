//! Crop every page by half an inch on each side, whatever its size or rotation.
//!
//! Margins are measured on the page as it shows, so a page turned on its side loses the same
//! strip from its visible top as an upright one. Cropping only hides content: the edit lists the
//! pages whose hidden content stays in the file, where text extraction still finds it.
//!
//! ```sh
//! cargo run -p laminapdf --example crop_pages -- in.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{CropMargins, CropPages, Document, OpenOptions, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: crop_pages in.pdf [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let cropped = crop(std::fs::read(input)?)?;
    std::fs::write(out.join("crop_pages.pdf"), cropped)?;
    Ok(())
}

/// The document with 36 points cropped from each side of every page.
pub(crate) fn crop(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let pages = doc.page_infos()?.into_iter().map(|page| page.id).collect();
    let edited = doc.apply(vec![CropPages::new(pages, CropMargins::all(36.0)).into()])?;
    for page in &edited.cropped_content_remains {
        eprintln!("page {page} hides content that is still in the file");
    }
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
