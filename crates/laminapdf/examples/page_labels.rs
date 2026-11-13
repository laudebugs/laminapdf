//! Give a document page labels, the names viewers show for its pages: "Cover" for the first,
//! lowercase roman numerals for the front matter, and numbers from 1 where the body starts.
//!
//! A range names its first page by ID and labels the pages up to the next range. The edit is
//! one op, saved incrementally after the original bytes.
//!
//! ```sh
//! cargo run -p laminapdf --example page_labels -- in.pdf [out-dir]   # a file of three pages or more
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{Document, LabelStyle, Op, OpenOptions, PageLabelRange, SaveMode, SaveOptions, SetPageLabels};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: page_labels in.pdf [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let labelled = label(std::fs::read(input)?)?;
    std::fs::write(out.join("page_labels.pdf"), labelled)?;
    Ok(())
}

/// The document with a cover, roman front matter from its second page and numbers from its
/// third, saved incrementally.
pub(crate) fn label(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    if doc.page_count()? < 3 {
        return Err("the example needs three pages or more".into());
    }
    let [cover, contents, body] = [0, 1, 2].map(|index| doc.page_info(index).map(|page| page.id));

    doc.apply(vec![Op::from(SetPageLabels::new(vec![
        PageLabelRange::new(cover?, None).with_prefix("Cover"),
        PageLabelRange::new(contents?, Some(LabelStyle::LowerRoman)),
        PageLabelRange::new(body?, Some(LabelStyle::Decimal)),
    ]))])?;

    // `doc.page_info(i)?.label` now reads "Cover", "i", "1", "2", …
    Ok(doc.save(&SaveOptions::new().with_mode(SaveMode::Incremental))?.bytes)
}
