//! An invoice that screen readers can follow, with an embedded font and a table of line items.
//!
//! The heading, the address, the table of items and the total are tagged, so a screen reader
//! reads them in order. Run it with `cargo run -p laminapdf --example invoice -- <font.ttf> [out-dir]`.

use laminapdf::{
    Alignment, BuildError, BuildOptions, Builder, FontHandle, Metadata, PageSize, PointPt, Pt, RectPt, SaveOptions,
    Standard14, Surface, TagKind, TextBoxOptions,
};

const ITEMS: [(&str, u32, f64); 4] = [
    ("Widgets, blue (W-100)", 3, 12.00),
    ("Gadget with ﬁne-tuned dials", 1, 79.90),
    ("Café-grade sprockets", 12, 1.25),
    ("Delivery", 1, 4.50),
];
const ADDRESS: &str = "Acme Widgets Ltd · 1 Example Street · London";
const PAGE: PageSize = PageSize::A4;
const MARGIN: f64 = 56.0;
const RIGHT_EDGE: f64 = PAGE.width.get() - MARGIN;
const TITLE_AT: PointPt = PointPt::from_points(MARGIN, 80.0);
const ADDRESS_AT: PointPt = PointPt::from_points(MARGIN, 100.0);
const TITLE: Pt = Pt::new(24.0);
const BODY: Pt = Pt::new(11.0);
const FIRST_ROW: f64 = 160.0;
const ROW_HEIGHT: f64 = 24.0;
/// The quantity's and the amount's columns, each a width ending this far left of the right edge.
const COLUMNS: [(f64, f64); 2] = [(40.0, 169.0), (100.0, 0.0)];
/// A cell's box starts this far above its row's baseline and is this tall.
const CELL_RISE: f64 = 12.0;
const CELL_HEIGHT: f64 = 16.0;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let font = std::fs::read(args.next().ok_or("usage: invoice <font.ttf> [out-dir]")?)?;
    let out = std::path::PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let pdf = build(BuildOptions::new(), font)?;
    Ok(std::fs::write(out.join("invoice.pdf"), pdf)?)
}

/// The bytes, in `font` (TrueType or OpenType); `options` can fix the file ID, producer and date.
pub(crate) fn build(options: BuildOptions, font: Vec<u8>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut b = Builder::new(options.with_tagged(true));
    draw(&mut b, font)?;
    Ok(b.finish()?.save(&SaveOptions::new())?.bytes)
}

/// Draws the invoice's one page into `b` and sets its title, author and language.
pub(crate) fn draw(b: &mut Builder, font: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    let (font, bold) = (b.font_bytes(font)?, b.standard_font(Standard14::HelveticaBold)?);
    let mut page = b.page(PAGE);
    let s = page.surface();
    s.text("Invoice 42", &bold, TITLE, TITLE_AT)?.tag(TagKind::H1);
    s.text(ADDRESS, &font, BODY, ADDRESS_AT)?.tag(TagKind::P);
    items(s, &font)?;
    let total: f64 = ITEMS.iter().map(|(_, qty, price)| f64::from(*qty) * price).sum();
    let (total, below_items) = (format!("Total € {total:.2}"), u32::try_from(ITEMS.len())?);
    let area = cell(COLUMNS[1], row_baseline(below_items));
    s.tag(TagKind::P, |s| right_aligned(s, &font, &total, area))?;
    page.finish()?;
    let metadata = Metadata::default().with_title("Invoice 42");
    b.metadata(metadata.with_author("Acme Widgets Ltd")).lang("en-GB");
    Ok(())
}

/// The table of line items: what, how many, and the amount, right-aligned.
fn items(s: &mut Surface, font: &FontHandle) -> Result<(), BuildError> {
    s.tag(TagKind::Table, |s| {
        for (row, (item, qty, price)) in (0_u32..).zip(ITEMS) {
            let y = row_baseline(row);
            s.tag(TagKind::TR, |s| {
                s.text(item, font, BODY, PointPt::from_points(MARGIN, y))?
                    .tag(TagKind::TD);
                let amount = format!("€ {:.2}", f64::from(qty) * price);
                for (text, column) in [qty.to_string(), amount].iter().zip(COLUMNS) {
                    s.tag(TagKind::TD, |s| right_aligned(s, font, text, cell(column, y)))?;
                }
                Ok(())
            })?;
        }
        Ok(())
    })
}

/// The baseline of row `row` of the table.
fn row_baseline(row: u32) -> f64 {
    FIRST_ROW + ROW_HEIGHT * f64::from(row)
}

/// The box of a cell in `column` (width, distance from the right edge) on the row at `baseline`.
fn cell((width, inset): (f64, f64), baseline: f64) -> RectPt {
    RectPt::from_points(RIGHT_EDGE - inset - width, baseline - CELL_RISE, width, CELL_HEIGHT)
}

/// `text` in `area`, in the body size, aligned to its right edge.
fn right_aligned(s: &mut Surface, font: &FontHandle, text: &str, area: RectPt) -> Result<(), BuildError> {
    let right = TextBoxOptions::new().with_align(Alignment::End);
    s.text_box(text, font, BODY, area, &right).map(|_| ())
}
