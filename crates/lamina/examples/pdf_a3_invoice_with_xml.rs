//! A hybrid invoice (PDF/A-3b, todo B8b): the invoice people read, with the same invoice as
//! XML attached for software to read. `Builder::attach` writes the file into
//! `/EmbeddedFiles` and associates it with the document (`/AF`, `/AFRelationship
//! /Alternative`), which PDF/A-3 requires of every attached file.
//!
//! ```sh
//! cargo run -p lamina --example pdf_a3_invoice_with_xml -- [out-dir]   # writes <out-dir>/pdf_a3_invoice_with_xml.pdf
//! ```

use std::error::Error;
use std::fmt::Write as _;
use std::path::PathBuf;

use lamina::{
    AfRelationship, Attachment, BuildOptions, Builder, CancelToken, Color, LinkTarget, Metadata, PageSize, PointPt,
    Profile, Pt, SaveOptions, StrokeStyle,
};

fn main() -> Result<(), Box<dyn Error>> {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("pdf_a3_invoice_with_xml.pdf"), build()?)?;
    Ok(())
}

const ITEMS: [(&str, u32, f64); 3] = [("Sprockets", 12, 1.25), ("Gadget", 1, 79.90), ("Delivery", 1, 4.50)];

/// The invoice as XML: the attachment, in a made-up schema (a real e-invoice would follow
/// EN 16931, such as Factur-X's Cross Industry Invoice).
fn invoice_xml(total: f64) -> String {
    let mut xml =
        String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<invoice number=\"42\" currency=\"EUR\">\n");
    for (item, qty, price) in ITEMS {
        // Writing into a String can't fail.
        let _ = writeln!(xml, "  <line item=\"{item}\" quantity=\"{qty}\" price=\"{price:.2}\"/>");
    }
    let _ = writeln!(xml, "  <total>{total:.2}</total>\n</invoice>");
    xml
}

/// The document's bytes.
pub(crate) fn build() -> Result<Vec<u8>, Box<dyn Error>> {
    let at = PointPt::from_points;
    let options = BuildOptions::default()
        .with_id(*b"lamina-ex-pdfa3b")
        .with_producer("lamina examples")
        .with_creation_date("D:20261031120000Z".parse()?)
        .with_profile(Profile::PdfA3b);
    let mut b = Builder::new(options);
    b.metadata(
        Metadata::default()
            .with_title("Invoice 42")
            .with_author("Acme Widgets Ltd"),
    );
    let font = b.font_bytes(include_bytes!("../../../fixtures/fonts/NotoSans-Regular.ttf").to_vec())?;
    let mut page = b.page(PageSize::A4);
    let s = page.surface();
    let _ = s.text("Invoice 42", &font, Pt::new(24.0), at(56.0, 80.0))?;
    let mut total = 0.0;
    for (row, (item, qty, price)) in (0_u8..).zip(ITEMS) {
        let y = 130.0 + 22.0 * f64::from(row);
        let amount = f64::from(qty) * price;
        total += amount;
        let _ = s.text(&format!("{qty} × {item}"), &font, Pt::new(11.0), at(56.0, y))?;
        let _ = s.text(&format!("€ {amount:.2}"), &font, Pt::new(11.0), at(460.0, y))?;
    }
    s.stroke_color(Color::gray(0.5))
        .path(|p| {
            p.move_to(at(56.0, 205.0)).line_to(at(539.0, 205.0));
        })
        .stroke(&StrokeStyle::default());
    let _ = s.text(&format!("Total € {total:.2}"), &font, Pt::new(14.0), at(420.0, 230.0))?;
    let _ = s
        .text(
            "The invoice is attached as invoice-42.xml.",
            &font,
            Pt::new(9.0),
            at(56.0, 280.0),
        )?
        .link(
            LinkTarget::Uri("https://example.com/invoices/42".into()),
            Some("Invoice 42 online".into()),
        );
    page.finish()?;
    b.attach(
        Attachment::new("invoice-42.xml", invoice_xml(total).into_bytes(), "text/xml")
            .with_relationship(AfRelationship::Alternative)
            .with_description("Invoice 42 as XML"),
    );
    let mut doc = b.finish()?;
    Ok(doc.save(&SaveOptions::new(), &CancelToken::new())?.bytes)
}
