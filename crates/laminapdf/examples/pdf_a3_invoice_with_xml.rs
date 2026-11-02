//! A PDF/A-3b invoice that carries the same invoice as XML, attached for software to read.
//!
//! Run it with `cargo run -p laminapdf --example pdf_a3_invoice_with_xml -- [out-dir]`.

use std::error::Error;
use std::fmt::Write as _;
use std::path::PathBuf;

use laminapdf::{
    AfRelationship, Attachment, BuildError, BuildOptions, Builder, CancelToken, Color, FontHandle, LinkTarget,
    Metadata, PageSize, PointPt, Profile, Pt, SaveOptions, StrokeStyle, Surface,
};

const MARGIN: f64 = 56.0;
/// The page's right edge, less the margin: where the rule under the items ends.
const RIGHT: f64 = PageSize::A4.width.get() - MARGIN;
const AMOUNTS: f64 = 460.0;
const ROW: f64 = 22.0;
const TITLE_BASELINE: f64 = 80.0;
const FIRST_ROW: f64 = 130.0;
/// How far above the next row's baseline the rule sits, and how far below it the total and
/// the note are.
const RULE_RISE: f64 = 7.0;
const TOTAL_DROP: f64 = 18.0;
const NOTE_DROP: f64 = 68.0;
/// How far left of the amounts column the total starts, to fit its "Total" label.
const TOTAL_INDENT: f64 = 40.0;
const TITLE: Pt = Pt::new(24.0);
const TOTAL: Pt = Pt::new(14.0);
const BODY: Pt = Pt::new(11.0);
const ITEMS: [(&str, u32, f64); 3] = [("Sprockets", 12, 1.25), ("Gadget", 1, 79.90), ("Delivery", 1, 4.50)];

fn main() -> Result<(), Box<dyn Error>> {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("pdf_a3_invoice_with_xml.pdf"), build()?)?;
    Ok(())
}

/// The invoice's total.
fn total() -> f64 {
    ITEMS.iter().map(|(_, qty, price)| f64::from(*qty) * price).sum()
}

/// The invoice as XML, in a made-up schema (a real e-invoice follows EN 16931, as Factur-X does).
fn invoice_xml() -> Result<String, std::fmt::Error> {
    let mut xml =
        String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<invoice number=\"42\" currency=\"EUR\">\n");
    for (item, qty, price) in ITEMS {
        writeln!(xml, "  <line item=\"{item}\" quantity=\"{qty}\" price=\"{price:.2}\"/>")?;
    }
    writeln!(xml, "  <total>{:.2}</total>\n</invoice>", total())?;
    Ok(xml)
}

/// The invoice's bytes.
pub(crate) fn build() -> Result<Vec<u8>, Box<dyn Error>> {
    let mut b = Builder::new(BuildOptions::default().with_profile(Profile::PdfA3b));
    b.metadata(
        Metadata::default()
            .with_title("Invoice 42")
            .with_author("Acme Widgets Ltd"),
    );
    let font = b.font_bytes(include_bytes!("../../../fixtures/fonts/NotoSans-Regular.ttf").to_vec())?;
    let mut page = b.page(PageSize::A4);
    draw(page.surface(), &font)?;
    page.finish()?;
    let xml = Attachment::new("invoice-42.xml", invoice_xml()?.into_bytes(), "text/xml")
        .with_relationship(AfRelationship::Alternative)
        .with_description("Invoice 42 as XML");
    b.attach(xml);
    let mut doc = b.finish()?;
    Ok(doc.save(&SaveOptions::new(), &CancelToken::new())?.bytes)
}

fn draw(s: &mut Surface, font: &FontHandle) -> Result<(), BuildError> {
    let at = PointPt::from_points;
    s.text("Invoice 42", font, TITLE, at(MARGIN, TITLE_BASELINE))?;
    let mut y = FIRST_ROW;
    for (item, qty, price) in ITEMS {
        s.text(&format!("{qty} × {item}"), font, BODY, at(MARGIN, y))?;
        let amount = f64::from(qty) * price;
        s.text(&format!("€ {amount:.2}"), font, BODY, at(AMOUNTS, y))?;
        y += ROW;
    }
    let rule = y - RULE_RISE;
    s.stroke_color(Color::gray(0.5))
        .path(|p| _ = p.move_to(at(MARGIN, rule)).line_to(at(RIGHT, rule)))
        .stroke(&StrokeStyle::default());
    let total_at = at(AMOUNTS - TOTAL_INDENT, y + TOTAL_DROP);
    s.text(&format!("Total € {:.2}", total()), font, TOTAL, total_at)?;
    let note = "The invoice is attached as invoice-42.xml.";
    let note = s.text(note, font, BODY, at(MARGIN, y + NOTE_DROP))?;
    let online = LinkTarget::Uri("https://example.com/invoices/42".into());
    note.link(online, Some("Invoice 42 online".into()));
    Ok(())
}
