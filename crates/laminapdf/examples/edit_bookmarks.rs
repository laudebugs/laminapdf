//! Edit a document's bookmarks and named destinations: add a bookmark, move one, and name a place links can jump to.
//!
//! A new "Summary" bookmark goes first, in bold, showing the top of the last page. The place
//! comes from page space, as a viewer reports it, and becomes the destination the bookmark
//! stores. The same place gets the name `summary`, a destination is renamed with every bookmark
//! that uses it, and the "Website" bookmark moves to the end of the "Chapters" bookmark.
//!
//! ```sh
//! cargo run -p laminapdf --example edit_bookmarks -- in.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    AddOutlineItem, Document, MoveOutlineItem, Op, OpenOptions, OutlineStyle, OutlineTarget, PageTargetView,
    RenameNamedDestination, SaveMode, SaveOptions, SetNamedDestination, SiblingPosition,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: edit_bookmarks in.pdf [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let edited = edit(std::fs::read(input)?)?;
    std::fs::write(out.join("edit_bookmarks.pdf"), edited)?;
    Ok(())
}

/// The document with its bookmarks edited, saved incrementally.
pub(crate) fn edit(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;

    // The top of the last page, in page space (points from the page's top-left).
    let last = doc.page_count()?.saturating_sub(1);
    let top = PageTargetView::At {
        x: Some(0.0),
        y: Some(0.0),
        zoom: None,
    };
    let summary = doc.destination_at(last, top)?;

    let mut add = AddOutlineItem::new(
        None,
        SiblingPosition::First,
        "Summary",
        OutlineTarget::Destination(summary.clone()),
    );
    add.style = OutlineStyle::new(None, true, false);
    doc.apply(vec![
        Op::from(add),
        Op::from(SetNamedDestination::new("summary", summary)),
        Op::from(RenameNamedDestination::new("chapter-2", "methods")),
    ])?;

    let outline = doc.outline()?;
    let find = |title: &str| outline.iter().find(|n| n.title == title).map(|n| n.id.clone());
    if let (Some(website), Some(chapters)) = (find("Website"), find("Chapters")) {
        doc.apply(vec![Op::from(MoveOutlineItem::new(
            website,
            Some(chapters),
            SiblingPosition::Last,
        ))])?;
    }

    let options = SaveOptions::new().with_mode(SaveMode::Incremental);
    Ok(doc.save(&options)?.bytes)
}
