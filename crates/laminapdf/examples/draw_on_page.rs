//! Draw on a page: a cloud around a row of the table, an arrow pointing at the chart, and a
//! freehand underline below the total.
//!
//! The points and boxes are in page space (points from the page's top-left), as a viewer reports
//! where a pointer went. Each mark records who drew it and when, and describes itself for screen
//! readers.
//!
//! ```sh
//! cargo run -p laminapdf --example draw_on_page -- in.pdf <time> [out-dir]   # the time as a PDF date
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    AnnotColor, AnnotCommon, AnnotSpec, Border, Document, LineEnding, Op, OpenOptions, PdfDate, PointPt, Pt, RectPt,
    SaveMode, SaveOptions, ShapeKind,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: draw_on_page in.pdf <time> [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    // When the marks were drawn, which your app takes from its clock.
    let at: PdfDate = args.next().ok_or(usage)?.parse()?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let drawn = draw_on_page(std::fs::read(input)?, at)?;
    std::fs::write(out.join("draw_on_page.pdf"), drawn)?;
    Ok(())
}

// Where the marks go on the example's input, the quarterly report, in points from the page's
// top-left: for your own file, take them from your viewer's pointer.
/// Around the table's Q2 row: `[x0, y0, x1, y1]`.
const Q2_ROW: [f64; 4] = [48.0, 317.0, 340.0, 338.0];
/// From the empty space right of the chart to its right edge: `[x, y]` each.
const ARROW: [[f64; 2]; 2] = [[420.0, 180.0], [264.0, 200.0]];
/// A wavy underline below "Total sales", as a pen traced it.
const UNDERLINE: [[f64; 2]; 9] = [
    [56.0, 482.0],
    [72.0, 478.0],
    [88.0, 484.0],
    [104.0, 478.0],
    [120.0, 484.0],
    [136.0, 478.0],
    [152.0, 484.0],
    [168.0, 478.0],
    [180.0, 482.0],
];
/// A reviewer's red pen.
const RED: AnnotColor = AnnotColor::Rgb(0.85, 0.1, 0.1);

/// The document with a cloud, an arrow and an underline drawn on its first page, saved
/// incrementally.
pub(crate) fn draw_on_page(bytes: Vec<u8>, at: PdfDate) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let by_me = |description: &str| {
        let mut common = AnnotCommon::default();
        common.author = Some("Ada Lovelace".into());
        common.created = Some(at);
        common.modified = Some(at);
        common.color = Some(RED);
        common.contents = Some(description.into());
        common
    };

    let mut cloud = doc.shape_at(0, ShapeKind::Square, area(Q2_ROW))?;
    if let AnnotSpec::Shape(shape) = &mut cloud.spec {
        shape.border = Border::cloudy(1.5, 0.5);
    }
    cloud.common = by_me("Check the Q2 figures");

    let [from, to] = ARROW.map(point);
    let mut arrow = doc.line_at(0, from, to, [LineEnding::None, LineEnding::ClosedArrow])?;
    if let AnnotSpec::Line(line) = &mut arrow.spec {
        line.border = Border::solid(2.0);
        line.interior = Some(RED);
    }
    arrow.common = by_me("Label the chart's axes");

    let mut underline = doc.ink_at(0, &[UNDERLINE.map(point).to_vec()], 1.5)?;
    underline.common = by_me("Underlined: the total");

    doc.apply(vec![Op::from(cloud), Op::from(arrow), Op::from(underline)])?;
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
