//! Choose how a PDF opens: on a given page at the page's width, with its bookmarks shown and its title in the window.
//!
//! The edit changes only what the file asks a reader to do when it opens it. A file whose open
//! action runs a script keeps that action: the edit leaves the opening page alone and changes
//! the rest.
//!
//! ```sh
//! cargo run -p laminapdf --example set_initial_view -- in.pdf <page-number> [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    Document, Op, OpenAt, OpenOptions, OpenTarget, OpenZoom, PageMode, SaveOptions, SetInitialView,
    WindowPreferencesPatch,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: set_initial_view in.pdf <page-number> [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    let number: u32 = args.next().ok_or(usage)?.parse()?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let index = number.checked_sub(1).ok_or("page numbers start at 1")?;
    let edited = open_on(std::fs::read(input)?, index)?;
    std::fs::write(out.join("set_initial_view.pdf"), edited)?;
    Ok(())
}

/// `bytes` set to open on page `index` (0-based) at the page's width, with the bookmarks panel
/// and the document's title in the window, saved incrementally.
pub(crate) fn open_on(bytes: Vec<u8>, index: u32) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let page = doc.page_info(index)?.id;

    let mut view = SetInitialView::default()
        .with_page_mode(Some(PageMode::Outlines))
        .with_preferences(WindowPreferencesPatch::default().with_display_doc_title(true));
    // An open action that runs a script or another action is never replaced, so leave the page out.
    if let OpenTarget::Action(kind) = doc.initial_view()?.open {
        eprintln!("the file runs a {kind} action when it opens; keeping it");
    } else {
        view = view.with_open(Some(OpenAt::new(page, OpenZoom::FitWidth)));
    }
    doc.apply(vec![Op::from(view)])?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
