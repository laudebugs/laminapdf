//! Organise a document's pages: delete one, move one to the front, add a blank page at the end
//! and turn two pages a quarter clockwise, in one edit that one undo reverts.
//!
//! Pages are named by their IDs, which stay the same while other pages move. Deleting a page
//! points the bookmarks, links and named destinations that went to it at the nearest page left,
//! and the edit lists each one, so an app can say what moved.
//!
//! ```sh
//! cargo run -p laminapdf --example organise_pages -- in.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    DeletePages, Document, InsertBlankPages, MovePages, Op, OpenOptions, PagePosition, PageSize, ReferenceOutcome,
    RotatePages, Rotation, SaveOptions,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: organise_pages in.pdf [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let organised = organise(std::fs::read(input)?)?;
    std::fs::write(out.join("organise_pages.pdf"), organised)?;
    Ok(())
}

/// The document with its third page deleted, its last page moved to the front, a blank Letter
/// page added at the end and its first two pages turned a quarter clockwise.
pub(crate) fn organise(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let ids: Vec<_> = doc.page_infos()?.into_iter().map(|page| page.id).collect();
    let (Some(third), Some(last)) = (ids.get(2), ids.last()) else {
        return Err("the document needs at least three pages".into());
    };
    let edited = doc.apply(vec![
        Op::from(DeletePages::new(vec![third.clone()])),
        Op::from(MovePages::new(vec![last.clone()], PagePosition::Start)),
        Op::from(InsertBlankPages::new(PagePosition::End, 1, PageSize::LETTER)),
        Op::from(RotatePages::turn(
            ids.iter().take(2).cloned().collect(),
            Rotation::Clockwise90,
        )),
    ])?;
    for reference in &edited.references {
        if let ReferenceOutcome::Retargeted { to } = &reference.outcome {
            eprintln!("{:?} now points to page {to}", reference.kind);
        }
    }
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
