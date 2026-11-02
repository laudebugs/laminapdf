//! An invoice: an embedded, subset font (Noto Sans), a bold standard 14 heading, a table of
//! line items with right-aligned amounts, rules and an accent bar, a link and metadata, tagged
//! (B8a): `H1`, `P`, a `Table` of `TR`/`TD`, the link in a `Link` element, the bar and the
//! rules as artifacts.
//!
//! ```sh
//! cargo run -p laminapdf --example invoice -- [out-dir]   # writes <out-dir>/invoice.pdf
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    Alignment, ArtifactKind, BuildOptions, Builder, Color, FillRule, LinkTarget, Metadata, PageSize, PointPt, Pt,
    RectPt, SaveOptions, Standard14, StrokeStyle, Surface, TagKind, TextBoxOptions,
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

/// The document's bytes.
pub(crate) fn build() -> Result<Vec<u8>, Box<dyn Error>> {
    let mut doc = builder()?.finish()?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}

/// The invoice, drawn and not yet finished (its tests read the tagging diagnostics).
pub(crate) fn builder() -> Result<Builder, Box<dyn Error>> {
    let at = PointPt::from_points;
    let options = BuildOptions::new()
        .with_id(*b"lamina-ex-invoic")
        .with_producer("lamina examples")
        .with_creation_date("D:20261030120000Z".parse()?)
        .with_tagged(true);
    let mut b = Builder::new(options);
    let font = b.font_bytes(include_bytes!("../../../fixtures/fonts/NotoSans-Regular.ttf").to_vec())?;
    let bold = b.standard_font(Standard14::HelveticaBold)?;
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    // The accent bar and the hairlines are decoration (artifacts); the rest is the
    // structure a screen reader follows: a heading, a paragraph, a table, the total, a link.
    s.artifact(ArtifactKind::Layout, |s| {
        s.fill_color(Color::hex("#1e78c8")?)
            .path(|p| {
                p.rect(RectPt::from_points(0.0, 0.0, 595.28, 14.0));
            })
            .fill(FillRule::NonZero)
            .fill_color(Color::gray(0.0));
        Ok(())
    })?;
    s.text("Invoice 42", &bold, Pt::new(24.0), at(56.0, 80.0))?
        .tag(TagKind::H1);
    s.text(
        "Acme Widgets Ltd · 1 Example Street · London",
        &font,
        Pt::new(10.0),
        at(56.0, 100.0),
    )?
    .tag(TagKind::P);
    let right = TextBoxOptions::new().with_align(Alignment::End);
    let hairline = StrokeStyle {
        width: Pt::new(0.5),
        ..StrokeStyle::default()
    };
    let cell = |s: &mut Surface, text: &str, r: RectPt| {
        s.tag(TagKind::TD, |s| {
            s.text_box(text, &font, Pt::new(11.0), r, &right).map(|_| ())
        })
    };
    let mut total = 0.0;
    s.tag(TagKind::Table, |s| {
        for (row, (item, qty, price)) in (0_u32..).zip(ITEMS) {
            let y = 160.0 + 24.0 * f64::from(row);
            let amount = f64::from(qty) * price;
            total += amount;
            s.tag(TagKind::TR, |s| {
                s.text(item, &font, Pt::new(11.0), at(56.0, y))?.tag(TagKind::TD);
                cell(s, &qty.to_string(), RectPt::from_points(330.0, y - 12.0, 40.0, 16.0))?;
                cell(
                    s,
                    &format!("€ {amount:.2}"),
                    RectPt::from_points(439.0, y - 12.0, 100.0, 16.0),
                )?;
                s.artifact(ArtifactKind::Layout, |s| {
                    s.stroke_color(Color::gray(0.8))
                        .path(|p| {
                            p.move_to(at(56.0, y + 8.0)).line_to(at(539.0, y + 8.0));
                        })
                        .stroke(&hairline);
                    Ok(())
                })
            })?;
        }
        Ok(())
    })?;
    s.tag(TagKind::P, |s| {
        s.text_box(
            &format!("Total € {total:.2}"),
            &font,
            Pt::new(14.0),
            RectPt::from_points(339.0, 262.0, 200.0, 20.0),
            &right,
        )
        .map(|_| ())
    })?;
    s.text(
        "Pay online at example.com/pay/42",
        &font,
        Pt::new(11.0),
        at(56.0, 320.0),
    )?
    .fill(Color::rgb(0.12, 0.47, 0.78))
    .link(
        LinkTarget::Uri("https://example.com/pay/42".into()),
        Some("Pay invoice 42 online".into()),
    )
    .tag(TagKind::P);
    page.finish()?;
    b.metadata(
        Metadata::default()
            .with_title("Invoice 42")
            .with_author("Acme Widgets Ltd"),
    )
    .lang("en-GB");
    Ok(b)
}
