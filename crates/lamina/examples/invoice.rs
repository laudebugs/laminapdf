//! An invoice: an embedded, subset font (Noto Sans), a bold standard 14 heading, a table of
//! line items with right-aligned amounts, rules and an accent bar, a link and metadata.
//!
//! ```sh
//! cargo run -p lamina --example invoice -- [out-dir]   # writes <out-dir>/invoice.pdf
//! ```

use std::error::Error;
use std::path::PathBuf;

use lamina::{
    Alignment, BuildOptions, Builder, CancelToken, Color, FillRule, LinkTarget, Metadata, PageSize, PointPt, Pt,
    RectPt, SaveOptions, Standard14, StrokeStyle, TextBoxOptions,
};

const ITEMS: [(&str, u32, f64); 4] = [
    ("Widgets, blue (W-100)", 3, 12.00),
    ("Gadget with ﬁne-tuned dials", 1, 79.90),
    ("Café-grade sprockets", 12, 1.25),
    ("Delivery", 1, 4.50),
];

fn main() -> Result<(), Box<dyn Error>> {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("invoice.pdf"), build()?)?;
    Ok(())
}

fn rect(x: f64, y: f64, w: f64, h: f64) -> RectPt {
    RectPt::from_xywh(Pt::new(x), Pt::new(y), Pt::new(w), Pt::new(h))
}

/// The document's bytes.
pub(crate) fn build() -> Result<Vec<u8>, Box<dyn Error>> {
    let at = PointPt::from_points;
    let options = BuildOptions::default()
        .with_id(*b"lamina-ex-invoic")
        .with_producer("lamina examples")
        .with_creation_date("D:20261030120000Z".parse()?);
    let mut b = Builder::new(options);
    let font = b.font_bytes(include_bytes!("../../../fixtures/fonts/NotoSans-Regular.ttf").to_vec())?;
    let bold = b.standard_font(Standard14::HelveticaBold)?;
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    s.fill_color(Color::hex("#1e78c8")?)
        .path(|p| {
            p.rect(rect(0.0, 0.0, 595.28, 14.0));
        })
        .fill(FillRule::NonZero)
        .fill_color(Color::gray(0.0));
    let _ = s.text("Invoice 42", &bold, Pt::new(24.0), at(56.0, 80.0))?;
    let _ = s.text(
        "Acme Widgets Ltd · 1 Example Street · London",
        &font,
        Pt::new(10.0),
        at(56.0, 100.0),
    )?;
    let right = TextBoxOptions::default().with_align(Alignment::End);
    let hairline = StrokeStyle {
        width: Pt::new(0.5),
        ..StrokeStyle::default()
    };
    let mut total = 0.0;
    for (row, (item, qty, price)) in (0_u32..).zip(ITEMS) {
        let y = 160.0 + 24.0 * f64::from(row);
        let amount = f64::from(qty) * price;
        total += amount;
        let _ = s.text(item, &font, Pt::new(11.0), at(56.0, y))?;
        s.text_box(
            &qty.to_string(),
            &font,
            Pt::new(11.0),
            rect(330.0, y - 12.0, 40.0, 16.0),
            &right,
        )?;
        s.text_box(
            &format!("€ {amount:.2}"),
            &font,
            Pt::new(11.0),
            rect(439.0, y - 12.0, 100.0, 16.0),
            &right,
        )?;
        s.stroke_color(Color::gray(0.8))
            .path(|p| {
                p.move_to(at(56.0, y + 8.0)).line_to(at(539.0, y + 8.0));
            })
            .stroke(&hairline);
    }
    s.text_box(
        &format!("Total € {total:.2}"),
        &font,
        Pt::new(14.0),
        rect(339.0, 262.0, 200.0, 20.0),
        &right,
    )?;
    let _ = s
        .text(
            "Pay online at example.com/pay/42",
            &font,
            Pt::new(11.0),
            at(56.0, 320.0),
        )?
        .fill(Color::rgb(0.12, 0.47, 0.78))
        .link(
            LinkTarget::Uri("https://example.com/pay/42".into()),
            Some("Pay invoice 42 online".into()),
        );
    page.finish()?;
    b.metadata(
        Metadata::default()
            .with_title("Invoice 42")
            .with_author("Acme Widgets Ltd"),
    )
    .lang("en-GB");
    let mut doc = b.finish()?;
    Ok(doc.save(&SaveOptions::new(), &CancelToken::new())?.bytes)
}
