//! Split a PDF into one file per chapter.
//!
//! The file is cut where each top-level bookmark's page begins, and each part keeps the
//! bookmarks of its chapter.
//!
//! ```sh
//! cargo run -p laminapdf --example split_document -- book.pdf [out-dir]
//! ```

use std::error::Error;
use std::num::NonZeroUsize;
use std::path::PathBuf;

use laminapdf::{Document, OpenOptions, SaveOptions, SplitBy};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: split_document book.pdf [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    for (i, part) in split(std::fs::read(input)?)?.into_iter().enumerate() {
        std::fs::write(out.join(format!("split_document-{}.pdf", i + 1)), part)?;
    }
    Ok(())
}

/// One file per chapter of `bytes`: a part from each top-level bookmark's page to the next.
pub(crate) fn split(bytes: Vec<u8>) -> Result<Vec<Vec<u8>>, Box<dyn Error>> {
    let doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let chapters = doc.split(&SplitBy::Bookmarks {
        level: NonZeroUsize::MIN,
    })?;
    let mut files = Vec::with_capacity(chapters.len());
    for mut chapter in chapters {
        files.push(chapter.save(&SaveOptions::new())?.bytes);
    }
    Ok(files)
}
