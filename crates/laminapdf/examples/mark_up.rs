//! Mark up a page: highlight a line of text, leave a note beside the chart, and propose a better word.
//!
//! The boxes are in page space (points from the page's top-left), as a viewer reports a text
//! selection and a click. Each mark records who made it and when. In a tagged document a
//! highlight's description, which screen readers read, is the text it marks.
//!
//! ```sh
//! cargo run -p laminapdf --features extract --example mark_up -- in.pdf <time> [out-dir]   # the time as a PDF date
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    AnnotColor, AnnotCommon, Document, MarkupKind, NoteIcon, Op, OpenOptions, PdfDate, PointPt, Pt, RectPt, SaveMode,
    SaveOptions,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: mark_up in.pdf <time> [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    // When the marks were made, which your app takes from its clock.
    let at: PdfDate = args.next().ok_or(usage)?.parse()?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let marked = mark_up(std::fs::read(input)?, at)?;
    std::fs::write(out.join("mark_up.pdf"), marked)?;
    Ok(())
}

// Where the marks go on the example's input, the quarterly report, in points from the page's
// top-left: for your own file, take them from your viewer's selection and clicks, or from
// `page_text`'s lines.
/// The line "Widgets sold out in May.", as a selection gives it: `[x0, y0, x1, y1]`.
const WIDGETS_LINE: [f64; 4] = [56.0, 358.241, 196.335, 373.223];
/// The words "sold out" on that line.
const SOLD_OUT: [f64; 4] = [114.902, 358.241, 156.713, 373.223];
/// The note's spot, right of the chart.
const BESIDE_CHART: [f64; 2] = [310.0, 140.0];
/// The colour of a paper sticky note.
const STICKY_YELLOW: AnnotColor = AnnotColor::Rgb(1.0, 0.82, 0.0);

/// The document with a highlight, a note and a proposed word on its first page, saved
/// incrementally.
pub(crate) fn mark_up(bytes: Vec<u8>, at: PdfDate) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let mut by_me = AnnotCommon::default();
    by_me.author = Some("Ada Lovelace".into());
    by_me.created = Some(at.clone());
    by_me.modified = Some(at);

    // In a tagged document the highlight's description is already the marked text; add who
    // made it.
    let mut highlight = doc.mark_text(0, MarkupKind::Highlight, &[area(WIDGETS_LINE)])?;
    sign(&mut highlight.common, &by_me);

    let [x, y] = BESIDE_CHART;
    let spot = PointPt::new(Pt::new(x), Pt::new(y));
    let mut note = doc.note_at(0, spot, "Add June's figures?", NoteIcon::Comment)?;
    sign(&mut note.common, &by_me);
    note.common.color = Some(STICKY_YELLOW);

    doc.apply(vec![Op::from(highlight), Op::from(note)])?;

    // "sold out" reads better as "ran out": a strike-out and a caret, applied as one undo step.
    doc.replace_text(0, &[area(SOLD_OUT)], "ran out", by_me)?;

    let options = SaveOptions::new().with_mode(SaveMode::Incremental);
    Ok(doc.save(&options)?.bytes)
}

/// A box in page space from its corners.
fn area([x0, y0, x1, y1]: [f64; 4]) -> RectPt {
    RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
}

/// Records on a mark who made it and when, from `by`.
fn sign(common: &mut AnnotCommon, by: &AnnotCommon) {
    common.author.clone_from(&by.author);
    common.created.clone_from(&by.created);
    common.modified.clone_from(&by.modified);
}
