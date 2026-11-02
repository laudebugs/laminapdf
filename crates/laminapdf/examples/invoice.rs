//! An invoice that screen readers can follow, with an embedded font and a table of line items.
//!
//! The heading, the address, the items and the total are tagged; the rules under the rows are
//! decoration. Run it with `cargo run -p laminapdf --example invoice -- <font.ttf> [out-dir]`.

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    Alignment, ArtifactKind, BuildError, BuildOptions, Builder, Color, FontHandle, Metadata, PageSize, PointPt, Pt,
    RectPt, SaveOptions, Standard14, StrokeStyle, Surface, TagKind, TextBoxOptions,
};

const ITEMS: [(&str, u32, f64); 4] = [
    ("Widgets, blue (W-100)", 3, 12.00),
    ("Gadget with ﬁne-tuned dials", 1, 79.90),
    ("Café-grade sprockets", 12, 1.25),
    ("Delivery", 1, 4.50),
];
const MARGIN: f64 = 56.0;
const RIGHT_EDGE: f64 = 539.0;
const ADDRESS: &str = "Acme Widgets Ltd · 1 Example Street · London";
const TITLE: Pt = Pt::new(24.0);
const TOTAL: Pt = Pt::new(14.0);
const BODY: Pt = Pt::new(11.0);
const FIRST_ROW: f64 = 160.0;
const ROW_HEIGHT: f64 = 24.0;
/// The quantity's and the amount's columns (left edge, width), both right-aligned.
const COLUMNS: [(f64, f64); 2] = [(330.0, 40.0), (439.0, 100.0)];

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let font = std::fs::read(args.next().ok_or("usage: invoice <font.ttf> [out-dir]")?)?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let pdf = build(BuildOptions::new(), font)?;
    Ok(std::fs::write(out.join("invoice.pdf"), pdf)?)
}

/// The bytes, in `font` (TrueType or OpenType); `options` can fix the file ID, producer and date.
pub(crate) fn build(options: BuildOptions, font: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut b = Builder::new(options.with_tagged(true));
    draw(&mut b, font)?;
    Ok(b.finish()?.save(&SaveOptions::new())?.bytes)
}

/// Draws the invoice's one page into `b` and sets its title, author and language.
pub(crate) fn draw(b: &mut Builder, font: Vec<u8>) -> Result<(), Box<dyn Error>> {
    let (font, bold) = (b.font_bytes(font)?, b.standard_font(Standard14::HelveticaBold)?);
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    let at = PointPt::from_points;
    s.text("Invoice 42", &bold, TITLE, at(MARGIN, 80.0))?.tag(TagKind::H1);
    s.text(ADDRESS, &font, BODY, at(MARGIN, 100.0))?.tag(TagKind::P);
    items(s, &font)?;
    let total: f64 = ITEMS.iter().map(|(_, qty, price)| f64::from(*qty) * price).sum();
    let total = format!("Total € {total:.2}");
    let total_box = RectPt::from_points(339.0, 262.0, 200.0, 20.0);
    s.tag(TagKind::P, |s| right_aligned(s, &font, &total, TOTAL, total_box))?;
    page.finish()?;
    let metadata = Metadata::default().with_title("Invoice 42");
    b.metadata(metadata.with_author("Acme Widgets Ltd")).lang("en-GB");
    Ok(())
}

/// The table of line items, each row with a light rule under it (decoration).
fn items(s: &mut Surface, font: &FontHandle) -> Result<(), BuildError> {
    let at = PointPt::from_points;
    let rule = StrokeStyle {
        width: Pt::new(0.5),
        ..StrokeStyle::default()
    };
    s.tag(TagKind::Table, |s| {
        for (row, (item, qty, price)) in (0_u32..).zip(ITEMS) {
            let y = FIRST_ROW + ROW_HEIGHT * f64::from(row);
            let amount = f64::from(qty) * price;
            s.tag(TagKind::TR, |s| {
                s.text(item, font, BODY, at(MARGIN, y))?.tag(TagKind::TD);
                for (text, (left, width)) in [qty.to_string(), format!("€ {amount:.2}")].iter().zip(COLUMNS) {
                    let area = RectPt::from_points(left, y - 12.0, width, 16.0);
                    s.tag(TagKind::TD, |s| right_aligned(s, font, text, BODY, area))?;
                }
                let (from, to) = (at(MARGIN, y + 8.0), at(RIGHT_EDGE, y + 8.0));
                s.artifact(ArtifactKind::Layout, |s| {
                    s.stroke_color(Color::gray(0.8))
                        .path(|p| _ = p.move_to(from).line_to(to))
                        .stroke(&rule);
                    Ok(())
                })
            })?;
        }
        Ok(())
    })
}

/// `text` in `area`, aligned to its right edge.
fn right_aligned(s: &mut Surface, font: &FontHandle, text: &str, size: Pt, area: RectPt) -> Result<(), BuildError> {
    let right = TextBoxOptions::new().with_align(Alignment::End);
    s.text_box(text, font, size, area, &right).map(|_| ())
}
