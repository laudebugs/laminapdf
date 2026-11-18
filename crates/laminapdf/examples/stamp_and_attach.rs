//! Approve a report: a stamp with the approver's name at the top of its first page, and the
//! table's figures attached as a CSV file beside the table.
//!
//! The box and the point are in page space (points from the page's top-left), as a viewer
//! reports where a pointer went. The stamp's lines are set in Helvetica, which every reader has,
//! so the WebAssembly extract module can draw it too.
//!
//! ```sh
//! cargo run -p laminapdf --example stamp_and_attach -- in.pdf <time> [out-dir]   # the time as a PDF date
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    AnnotCommon, AnnotSpec, AttachmentIcon, Document, DynamicStamp, Op, OpenOptions, PdfDate, PointPt, Pt, RectPt,
    SaveMode, SaveOptions, StampKind, StampStyle,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: stamp_and_attach in.pdf <time> [out-dir]";
    let input = PathBuf::from(args.next().ok_or(usage)?);
    // When the report was approved, which your app takes from its clock.
    let at: PdfDate = args.next().ok_or(usage)?.parse()?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let approved = stamp_and_attach(std::fs::read(input)?, at)?;
    std::fs::write(out.join("stamp_and_attach.pdf"), approved)?;
    Ok(())
}

// Where the marks go on the example's input, the quarterly report, in points from the page's
// top-left: for your own file, take them from your viewer's pointer.
/// The stamp, right of the title: `[x0, y0, x1, y1]`.
const STAMP: [f64; 4] = [370.0, 56.0, 540.0, 106.0];
/// The attachment's icon, right of the table's last column.
const ICON: [f64; 2] = [370.0, 282.0];
/// The table's figures.
const FIGURES: &str = "Quarter,Units,Sales\nQ1,120,1200\nQ2,310,3100\n";

/// The document approved: a stamp and an attached file on its first page, saved incrementally.
pub(crate) fn stamp_and_attach(bytes: Vec<u8>, at: PdfDate) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = Document::open_bytes(bytes, OpenOptions::new())?;
    let by_me = || {
        let mut common = AnnotCommon::default();
        common.author = Some("Ada Lovelace".into());
        common.created = Some(at);
        common.modified = Some(at);
        common
    };

    // The stamp's lines are written into it now and never change, as on a rubber stamp.
    let lines = vec!["APPROVED".to_owned(), "by Ada Lovelace".to_owned()];
    let approved = StampKind::Dynamic(DynamicStamp::new(lines, StampStyle::Green));
    let mut stamp = doc.stamp_at(0, area(STAMP), approved)?;
    stamp.common = by_me();

    let mut attach = doc.attach_file_at(0, point(ICON), "figures.csv", FIGURES.as_bytes().to_vec(), "text/csv")?;
    if let AnnotSpec::FileAttachment(spec) = &mut attach.spec {
        spec.icon = AttachmentIcon::Graph;
        spec.file.description = Some("The table's figures".into());
        spec.file.modified = Some(at);
    }
    attach.common = by_me();

    doc.apply(vec![Op::from(stamp), Op::from(attach)])?;
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
