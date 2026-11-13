//! Add links to a page: one on a box drawn over a chart, one on a line of text, then point the first somewhere else.
//!
//! The boxes are in page space (points from the page's top-left), as a viewer reports a drawn
//! box and a text selection. Each link gets a description, which screen readers read. In a
//! tagged document the link on text joins the structure tree around the words it covers.
//!
//! ```sh
//! cargo run -p laminapdf --features extract --example add_links -- in.pdf [out-dir]
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{Document, Op, OpenOptions, OutlineTarget, Pt, RectPt, SaveMode, SaveOptions, UpdateLink};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(args.next().ok_or("usage: add_links in.pdf [out-dir]")?);
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let edited = add(std::fs::read(input)?)?;
    std::fs::write(out.join("add_links.pdf"), edited)?;
    Ok(())
}

/// The document with two links added to its first page, saved incrementally.
pub(crate) fn add(bytes: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    // These boxes fit the example's input, the quarterly report; for your own file, take them
    // from your viewer's drawn box and selection, or from `page_text`'s lines.
    let points = |x0, y0, x1, y1| RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1));

    // A box drawn over the chart.
    let chart = points(56.0, 140.0, 300.0, 240.0);
    let mut on_chart = doc.link_on_area(0, chart, OutlineTarget::Uri("https://example.com/sales".into()))?;
    on_chart.contents = Some("Sales figures".into());

    // The line "Widgets sold out in May.", as a selection gives it.
    let line = points(56.0, 358.241, 196.335, 373.223);
    let mut on_text = doc.link_on_text(0, &[line], OutlineTarget::Uri("https://example.com/widgets".into()))?;
    on_text.contents = Some("Widget stock".into());

    let added = doc.apply(vec![Op::from(on_chart), Op::from(on_text)])?;

    // The new links' IDs come back with the edit: change the first one's address.
    if let Some(chart_link) = added.created_annotations.first() {
        let mut update = UpdateLink::new(chart_link.clone());
        update.target = Some(OutlineTarget::Uri("https://example.com/sales/2026".into()));
        doc.apply(vec![Op::from(update)])?;
    }

    let options = SaveOptions::new().with_mode(SaveMode::Incremental);
    Ok(doc.save(&options)?.bytes)
}
