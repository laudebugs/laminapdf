//! Type on a page: a callout pointing at the chart, and a filled note beside the total with one
//! word in bold.
//!
//! The boxes and points are in page space (points from the page's top-left), as a viewer reports
//! where a pointer went. The boxes are in Helvetica, which every reader has, so the WebAssembly
//! extract module can draw them too.
//!
//! ```sh
//! cargo run -p laminapdf --example add_text_box -- in.pdf <time> [out-dir]   # the time as a PDF date
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    AnnotColor, AnnotCommon, AnnotSpec, Document, Op, OpenOptions, PdfDate, PointPt, Pt, RectPt, RichText, SaveMode,
    SaveOptions, TextSpan,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: add_text_box in.pdf <time> [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    // When the boxes were typed, which your app takes from its clock.
    let at: PdfDate = args.next().ok_or(usage)?.parse()?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let typed = add_text_box(std::fs::read(input)?, at)?;
    std::fs::write(out.join("add_text_box.pdf"), typed)?;
    Ok(())
}

// Where the boxes go on the example's input, the quarterly report, in points from the page's
// top-left: for your own file, take them from your viewer's pointer.
/// The callout's box, in the space right of the chart: `[x0, y0, x1, y1]`.
const CALLOUT: [f64; 4] = [330.0, 168.0, 500.0, 190.0];
/// The callout's line: from the chart's right edge, through a knee, to the box.
const LEADER: [[f64; 2]; 3] = [[257.0, 200.0], [300.0, 179.0], [330.0, 179.0]];
/// The note beside the total.
const NOTE: [f64; 4] = [300.0, 452.0, 520.0, 486.0];
/// A pale yellow, like a sticky note.
const YELLOW: AnnotColor = AnnotColor::Rgb(1.0, 0.96, 0.7);

/// The document with a callout and a note typed on its first page, saved incrementally.
pub(crate) fn add_text_box(bytes: Vec<u8>, at: PdfDate) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let by_me = || {
        let mut common = AnnotCommon::default();
        common.author = Some("Ada Lovelace".into());
        common.created = Some(at);
        common.modified = Some(at);
        common
    };

    let leader: Vec<PointPt> = LEADER.iter().map(|&p| point(p)).collect();
    let mut callout = doc.callout_at(0, area(CALLOUT), &leader, RichText::plain("Label the chart's axes"))?;
    callout.common = by_me();

    let text = RichText {
        paragraphs: vec![vec![
            TextSpan::plain("Check: "),
            TextSpan::plain("€ 4,300").with_bold(true),
            TextSpan::plain(" should match the table's total."),
        ]],
    };
    let mut note = doc.text_box_at(0, area(NOTE), text)?;
    if let AnnotSpec::FreeText(spec) = &mut note.spec {
        spec.fill = Some(YELLOW);
    }
    note.common = by_me();

    doc.apply(vec![Op::from(callout), Op::from(note)])?;
    let options = SaveOptions::new().with_mode(SaveMode::Incremental);
    Ok(doc.save(&options)?.bytes)
}

/// A point in page space.
fn point([x, y]: [f64; 2]) -> PointPt {
    PointPt::new(Pt::new(x), Pt::new(y))
}

/// A box in page space from its corners.
fn area([x0, y0, x1, y1]: [f64; 4]) -> RectPt {
    RectPt::new(Pt::new(x0), Pt::new(y0), Pt::new(x1), Pt::new(y1))
}
