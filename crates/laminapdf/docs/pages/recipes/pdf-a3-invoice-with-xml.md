---
title: "A PDF/A-3 invoice with XML"
description: "A PDF/A-3b invoice that carries the same invoice as an attached XML file, for people to read and for software to process."
version: "0.4.0"
url: "https://docs.laminapdf.com/recipes/pdf-a3-invoice-with-xml/"
---

# A PDF/A-3 invoice with XML

A PDF/A-3b invoice that carries the same invoice as an attached XML file, for people to read and for software to process.

You'll write a one-page invoice as PDF/A-3b, with the same invoice attached as XML. You need this for e-invoices such as Factur-X and ZUGFeRD, where a person reads the PDF and the buyer's software reads the XML.

The function draws the invoice, writes the same items as XML and attaches that file before it finishes. In TypeScript it needs the text module, `init({ text: true })`. Both languages write the same bytes:

**TypeScript**

`packages/sdk/examples/pdf-a3-invoice-with-xml.ts`:

```ts
// A PDF/A-3b invoice that carries the same invoice as XML, attached for software to read.
import { Builder, Font, PAGE_SIZES, type Surface } from "@laminapdf/sdk";

const MARGIN = 56;
/** The page's right edge, less the margin: where the rule under the items ends. */
const RIGHT = PAGE_SIZES.A4[0] - MARGIN;
const AMOUNTS = 460;
const ROW = 22;
const TITLE_BASELINE = 80;
const FIRST_ROW = 130;
/**
 * How far above the next row's baseline the rule sits, and how far below it the total and
 * the note are.
 */
const RULE_RISE = 7;
const TOTAL_DROP = 18;
const NOTE_DROP = 68;
/** How far left of the amounts column the total starts, to fit its "Total" label. */
const TOTAL_INDENT = 40;
const TITLE = 24;
const TOTAL = 14;
const BODY = 11;
const ITEMS = [
  ["Sprockets", 12, 1.25],
  ["Gadget", 1, 79.9],
  ["Delivery", 1, 4.5],
] as const;

/** The invoice's total. */
function total(): number {
  return ITEMS.reduce((sum, [, qty, price]) => sum + qty * price, 0);
}

/** The invoice as XML, in a made-up schema (a real e-invoice follows EN 16931, as Factur-X does). */
function invoiceXml(): string {
  const lines = ITEMS.map(
    ([item, qty, price]) => `  <line item="${item}" quantity="${qty}" price="${price.toFixed(2)}"/>\n`,
  );
  return `<?xml version="1.0" encoding="UTF-8"?>\n<invoice number="42" currency="EUR">\n${lines.join("")}  <total>${total().toFixed(2)}</total>\n</invoice>\n`;
}

/**
 * The invoice's bytes. Needs `init({ text: true })`.
 *
 * @param notoSans - the bytes of the Noto Sans font file.
 */
export function pdfA3InvoiceWithXml(notoSans: Uint8Array): Uint8Array<ArrayBuffer> {
  using b = new Builder({ profiles: ["PDF/A-3b"] });
  b.metadata({ title: "Invoice 42", author: "Acme Widgets Ltd" });
  const font = Font.fromBytes(notoSans);
  b.font(font);
  const page = b.page("A4");
  draw(page.surface(), font);
  page.finish();
  b.attach("invoice-42.xml", new TextEncoder().encode(invoiceXml()), "text/xml", {
    relationship: "alternative",
    description: "Invoice 42 as XML",
  });
  using doc = b.finish();
  return doc.save();
}

function draw(s: Surface, font: Font): void {
  s.text("Invoice 42", { font, size: TITLE, at: [MARGIN, TITLE_BASELINE] });
  let y = FIRST_ROW;
  for (const [item, qty, price] of ITEMS) {
    s.text(`${qty} × ${item}`, { font, size: BODY, at: [MARGIN, y] });
    s.text(`€ ${(qty * price).toFixed(2)}`, { font, size: BODY, at: [AMOUNTS, y] });
    y += ROW;
  }
  const rule = y - RULE_RISE;
  s.strokeColor([0.5])
    .path((p) => p.moveTo(MARGIN, rule).lineTo(RIGHT, rule))
    .stroke();
  s.text(`Total € ${total().toFixed(2)}`, { font, size: TOTAL, at: [AMOUNTS - TOTAL_INDENT, y + TOTAL_DROP] });
  s.text("The invoice is attached as invoice-42.xml.", {
    font,
    size: BODY,
    at: [MARGIN, y + NOTE_DROP],
    link: { target: { type: "uri", uri: "https://example.com/invoices/42" }, contents: "Invoice 42 online" },
  });
}
```

**Rust**

`crates/laminapdf/examples/pdf_a3_invoice_with_xml.rs`:

```rust
//! A PDF/A-3b invoice that carries the same invoice as XML, attached for software to read.
//!
//! Run it with `cargo run -p laminapdf --example pdf_a3_invoice_with_xml -- <font.ttf> [out-dir]`.

use std::error::Error;
use std::fmt::Write as _;
use std::path::PathBuf;

use laminapdf::{
    AfRelationship, Attachment, BuildError, BuildOptions, Builder, Color, FontHandle, LinkTarget, Metadata, PageSize,
    PointPt, Profile, Pt, SaveOptions, StrokeStyle, Surface,
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
    let mut args = std::env::args().skip(1);
    let font = std::fs::read(args.next().ok_or("arguments: <font.ttf> [out-dir]")?)?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    Ok(std::fs::write(out.join("pdf_a3_invoice_with_xml.pdf"), build(font)?)?)
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

/// The invoice's bytes, in `font` (a TrueType or OpenType file).
pub(crate) fn build(font: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut b = Builder::new(BuildOptions::new().with_profile(Profile::PdfA3b));
    b.metadata(
        Metadata::default()
            .with_title("Invoice 42")
            .with_author("Acme Widgets Ltd"),
    );
    let font = b.font_bytes(font)?;
    let mut page = b.page(PageSize::A4);
    draw(page.surface(), &font)?;
    page.finish()?;
    let xml = Attachment::new("invoice-42.xml", invoice_xml()?.into_bytes(), "text/xml")
        .with_relationship(AfRelationship::Alternative)
        .with_description("Invoice 42 as XML");
    b.attach(xml);
    let mut doc = b.finish()?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
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
```

**Result:** [pdf_a3_invoice_with_xml.pdf](https://docs.laminapdf.com/examples/pdf/pdf_a3_invoice_with_xml.pdf)

The result is one A4 page with three line items and a total of € 99.40, carrying `invoice-42.xml`. The Result tab shows it, with a link to the PDF. Viewers that show attachments list it there.

## How it works

- `profiles: ["PDF/A-3b"]` (`Profile::PdfA3b` in Rust) asks for PDF/A-2b's rules and allows attached files of any kind. `finish()` fails with a coded error for the first rule the invoice breaks; see [Archival PDFs](../../pages/guides/archival-pdfs.md).
- PDF/A needs every font embedded, so the invoice uses Noto Sans rather than one of the [standard 14 fonts](../../pages/reference/glossary.md#standard-14-fonts).
- `invoiceXml` writes the line items and the total from the same `ITEMS` the page draws, so the PDF and the XML always agree. Its schema is made up for the recipe.
- `attach` adds the file with its name, its bytes and its MIME type, `text/xml`. PDF/A needs a MIME type on every attachment, or `finish()` fails with `embedded-file-without-mime-type`.
- `relationship: "alternative"` (`AfRelationship::Alternative`) says the XML is another form of the same content, and the builder associates it with the document. `description` is the text a viewer shows beside the file.

## What to change

- **The items:** change `ITEMS`. The page, the total and the XML all follow.
- **A real e-invoice:** replace `invoiceXml` with XML that follows EN 16931, the European e-invoicing standard, as Factur-X and ZUGFeRD do. Those formats also need XMP entries of their own, which the builder can't add yet.
- **Other attachments:** attach the file the invoice was made from with `relationship: "source"`, or the data behind it with `"data"` (`AfRelationship::Source`, `AfRelationship::Data`). Each needs its own name and MIME type.
- **Accessible too:** add `"PDF/UA-1"` to `profiles`, set `lang` and tag the content, as the [PDF/UA report](../../pages/recipes/pdf-ua-report.md) does.

Next: [a 100-page report](../../pages/recipes/report.md), the case Lamina PDF times in every runtime.
