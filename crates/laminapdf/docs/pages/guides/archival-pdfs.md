---
title: "Archival PDFs"
description: "Build PDF/A-2b, PDF/A-2u and PDF/A-3b documents that look the same in years to come, and attach files such as an e-invoice's XML."
version: "0.3.0"
url: "https://docs.laminapdf.com/guides/archival-pdfs/"
---

# Archival PDFs

Build PDF/A-2b, PDF/A-2u and PDF/A-3b documents that look the same in years to come, and attach files such as an e-invoice's XML.

This guide builds PDF/A documents, which you need when a file must stay readable and look the same for decades: contracts, records kept by law, e-invoices. Lamina PDF writes PDF/A as it generates a document; it doesn't convert existing files to PDF/A.

## What PDF/A asks for

PDF/A is the archival subset of PDF. A PDF/A file carries everything it needs to be displayed: its fonts are embedded, its colours are defined, and nothing depends on the viewer, the network or a password. Lamina PDF writes three PDF/A profiles:

- **PDF/A-2b** ("basic"): the document looks the same everywhere.
- **PDF/A-2u** ("Unicode"): PDF/A-2b, and every glyph maps to Unicode, so the text can be searched and copied.
- **PDF/A-3b:** PDF/A-2b that may carry attached files of any kind.

Choose one with the `profiles` option (`BuildOptions::with_profile` in Rust). The builder then writes two things the standard asks for. It writes the PDF/A identification in the document's XMP metadata, the XML packet in the file that describes it. That packet always agrees with the Info dictionary, the older list of the same facts (title, author, dates) that PDF readers also show.

It also writes an output intent: a declaration of what device colours mean. The intent embeds an ICC profile, the standard file format that describes a colour space. Lamina PDF's output intent is sRGB, so RGB and gray colour are defined. `DeviceCMYK` is CMYK colour given as plain numbers, with no profile to say which inks they mean (`Color::cmyk` in Rust, four numbers in TypeScript), so the sRGB intent leaves it undefined.

`finish()` fails with a coded error for each rule the document breaks:

| Rule | Error |
| --- | --- |
| Every font is embedded (no standard 14 font) | [`font-not-embedded`](../../errors/font-not-embedded.md) |
| No `DeviceCMYK` colour or CMYK image (the output intent is sRGB) | [`device-color-without-output-intent`](../../errors/device-color-without-output-intent.md) |
| PDF/A-2u: every glyph maps to Unicode | [`glyph-without-unicode`](../../errors/glyph-without-unicode.md) |
| PDF/A-2b, PDF/A-2u: no attached files | [`embedded-file-forbidden`](../../errors/embedded-file-forbidden.md) |
| Every attached file has a MIME type | [`embedded-file-without-mime-type`](../../errors/embedded-file-without-mime-type.md) |
| The PDF version is 1.7 or lower | [`profile-version-conflict`](../../errors/profile-version-conflict.md) |

For CMYK colour under PDF/A, give the colour an ICC profile instead (`Color::icc` in Rust): an ICC-based colour defines itself. Transparency is allowed in PDF/A-2 and PDF/A-3.

## An archived letter

The letter below is written twice, as PDF/A-2b and as PDF/A-2u. It embeds its font, places a photo and draws a translucent band across the top. The PDF/A-2u copy is also PDF/UA-1, the accessibility standard (see [Accessible PDFs](../../pages/guides/accessible-pdfs.md)): a document can be both archival and accessible.

**TypeScript**

`packages/sdk/examples/pdf-a-archive.ts`:

```ts
// An archival letter, written as PDF/A-2b and as PDF/A-2u that is also PDF/UA-1.
import { Builder, Font, Image, PAGE_SIZES, type Profile, type Surface } from "@laminapdf/sdk";

const MARGIN = 56;
const BAND_HEIGHT = 70;
const TITLE_BASELINE = 110;
/** The top of the letter's text and of the photo beside it, and the height of both. */
const BLOCK_TOP = 130;
const BLOCK_HEIGHT = 120;
const LETTER_WIDTH = 300;
const PHOTO_LEFT = 380;
const PHOTO_WIDTH = 160;
const FOOTER_BASELINE = 300;
const TITLE = 22;
const BODY = 11;
const BAND_OPACITY = 0.35;
const PHOTO_ALT = "A photograph of the sprockets, packed in a box";
const LETTER =
  "Thank you for your order of 12 sprockets. This letter confirms the delivery date and the price we agreed. " +
  "Keep it: PDF/A files look the same in any viewer, now and in the future.";

/** PDF/A-2b, or PDF/A-2u (whose text maps to Unicode) with PDF/UA-1. */
const PROFILES: Record<"2b" | "2u", Profile[]> = {
  "2b": ["PDF/A-2b"],
  "2u": ["PDF/A-2u", "PDF/UA-1"],
};

/**
 * The letter's bytes. Needs `init({ text: true })`.
 *
 * @param level - `2b` or `2u`.
 * @param notoSans - the bytes of the Noto Sans font file.
 * @param photo - the bytes of a JPEG photo.
 */
export function pdfAArchive(level: "2b" | "2u", notoSans: Uint8Array, photo: Uint8Array): Uint8Array<ArrayBuffer> {
  using b = new Builder({ profiles: PROFILES[level], tagged: true });
  b.metadata({ title: "Order confirmation 42", author: "Acme Widgets Ltd" });
  b.lang("en-GB");
  const font = Font.fromBytes(notoSans);
  b.font(font);
  const page = b.page("A4");
  draw(page.surface(), font, Image.jpeg(photo));
  page.finish();
  using doc = b.finish();
  return doc.save();
}

function draw(s: Surface, font: Font, photo: Image): void {
  // A translucent band across the top: decoration, and transparency, which PDF/A-2 allows.
  const [width] = PAGE_SIZES.A4;
  s.artifact("layout", (s) => {
    s.save().opacity(BAND_OPACITY, 1).fillColor([0.12, 0.47, 0.78]);
    s.path((p) => p.rect([0, 0, width, BAND_HEIGHT])).fill();
    s.restore();
  });
  s.tag("H1", (s) => s.text("Order confirmation 42", { font, size: TITLE, at: [MARGIN, TITLE_BASELINE] }));
  s.tag("P", (s) => s.textBox(LETTER, { font, size: BODY, box: [MARGIN, BLOCK_TOP, LETTER_WIDTH, BLOCK_HEIGHT] }));
  s.image(photo, [PHOTO_LEFT, BLOCK_TOP, PHOTO_WIDTH, BLOCK_HEIGHT], PHOTO_ALT);
  s.tag("P", (s) =>
    s.text("Acme Widgets Ltd, 1 Example Street, London", { font, size: BODY, at: [MARGIN, FOOTER_BASELINE] }),
  );
}
```

**Rust**

`crates/laminapdf/examples/pdf_a_archive.rs`:

```rust
//! An archival letter, written as PDF/A-2b and as PDF/A-2u that is also PDF/UA-1.
//!
//! Run it with `cargo run -p laminapdf --example pdf_a_archive -- <font.ttf> <photo.jpg> [out-dir]`: it writes
//! `pdf_a_archive-2b.pdf` and `pdf_a_archive-2u.pdf`.

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{
    ArtifactKind, BuildError, BuildOptions, Builder, Color, FillRule, FontHandle, Image, Metadata, PageSize, PointPt,
    Profile, Pt, RectPt, SaveOptions, Surface, TagKind, TextBoxOptions,
};

const MARGIN: f64 = 56.0;
const BAND_HEIGHT: f64 = 70.0;
const TITLE_AT: PointPt = PointPt::from_points(MARGIN, 110.0);
/// The top of the letter's text and of the photo beside it, and the height of both.
const BLOCK_TOP: f64 = 130.0;
const BLOCK_HEIGHT: f64 = 120.0;
const LETTER_WIDTH: f64 = 300.0;
const PHOTO_LEFT: f64 = 380.0;
const PHOTO_WIDTH: f64 = 160.0;
const FOOTER_AT: PointPt = PointPt::from_points(MARGIN, 300.0);
const TITLE: Pt = Pt::new(22.0);
const BODY: Pt = Pt::new(11.0);
const BAND_OPACITY: f64 = 0.35;
const PHOTO_ALT: &str = "A photograph of the sprockets, packed in a box";
const LETTER: &str = "Thank you for your order of 12 sprockets. This letter confirms the delivery date and the \
                      price we agreed. Keep it: PDF/A files look the same in any viewer, now and in the future.";

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "arguments: <font.ttf> <photo.jpg> [out-dir]";
    let font = std::fs::read(args.next().ok_or(usage)?)?;
    let photo = std::fs::read(args.next().ok_or(usage)?)?;
    let out = PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let a2b = build(Level::A2b, font.clone(), photo.clone())?;
    std::fs::write(out.join("pdf_a_archive-2b.pdf"), a2b)?;
    let a2u = build(Level::A2u, font, photo)?;
    Ok(std::fs::write(out.join("pdf_a_archive-2u.pdf"), a2u)?)
}

/// Which archive to write.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Level {
    /// PDF/A-2b: the letter looks the same everywhere.
    A2b,
    /// PDF/A-2u, whose text maps to Unicode, and PDF/UA-1.
    A2u,
}

/// The letter's bytes, in `font` (a TrueType or OpenType file) with `photo` (a JPEG).
pub(crate) fn build(level: Level, font: Vec<u8>, photo: Vec<u8>) -> Result<Vec<u8>, Box<dyn Error>> {
    let options = match level {
        Level::A2b => BuildOptions::new().with_profile(Profile::PdfA2b),
        Level::A2u => BuildOptions::new()
            .with_profile(Profile::PdfA2u)
            .with_profile(Profile::PdfUa1),
    };
    let mut b = Builder::new(options.with_tagged(true));
    b.metadata(
        Metadata::default()
            .with_title("Order confirmation 42")
            .with_author("Acme Widgets Ltd"),
    );
    b.lang("en-GB");
    let font = b.font_bytes(font)?;
    let photo = Image::jpeg(photo)?;
    let mut page = b.page(PageSize::A4);
    draw(page.surface(), &font, &photo)?;
    page.finish()?;
    let mut doc = b.finish()?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}

fn draw(s: &mut Surface, font: &FontHandle, photo: &Image) -> Result<(), BuildError> {
    // A translucent band across the top: decoration, and transparency, which PDF/A-2 allows.
    let width = PageSize::A4.width.get();
    s.artifact(ArtifactKind::Layout, |s| {
        s.save()
            .opacity(BAND_OPACITY, 1.0)
            .fill_color(Color::rgb(0.12, 0.47, 0.78));
        s.path(|p| _ = p.rect(RectPt::from_points(0.0, 0.0, width, BAND_HEIGHT)))
            .fill(FillRule::NonZero);
        s.restore();
        Ok(())
    })?;
    s.tag(TagKind::H1, |s| {
        s.text("Order confirmation 42", font, TITLE, TITLE_AT).map(drop)
    })?;
    let body = RectPt::from_points(MARGIN, BLOCK_TOP, LETTER_WIDTH, BLOCK_HEIGHT);
    s.tag(TagKind::P, |s| {
        s.text_box(LETTER, font, BODY, body, &TextBoxOptions::new()).map(drop)
    })?;
    let beside = RectPt::from_points(PHOTO_LEFT, BLOCK_TOP, PHOTO_WIDTH, BLOCK_HEIGHT);
    s.figure(photo, beside, Some(PHOTO_ALT));
    let footer = "Acme Widgets Ltd, 1 Example Street, London";
    s.tag(TagKind::P, |s| s.text(footer, font, BODY, FOOTER_AT).map(drop))
}
```

**Result:** [pdf_a_archive-2b.pdf](https://docs.laminapdf.com/examples/pdf/pdf_a_archive-2b.pdf), [pdf_a_archive-2u.pdf](https://docs.laminapdf.com/examples/pdf/pdf_a_archive-2u.pdf)

[See the output](../../pages/recipes/pdf-a-archive.md).

## Attaching files: a hybrid invoice

PDF/A-3 lets a document carry other files, provided each one is associated with the document and says how it relates to it: its source, the data it was made from, or another representation of the same content. E-invoicing formats such as Factur-X and ZUGFeRD use this to put the machine-readable invoice (XML) inside the human-readable one (PDF).

`attach(name, bytes, mime, { relationship })` adds a file, its MIME type and its relationship, and lists it in the document's attachments. The example uses a made-up XML schema; a real e-invoice follows the EN 16931 standard and adds its own XMP metadata.

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

[See the output](../../pages/recipes/pdf-a3-invoice-with-xml.md).

## How Lamina PDF checks its own output

Every example on this site that claims a PDF/A profile is validated each night with veraPDF, the open-source PDF/A and PDF/UA validator the PDF Association backs.

> **Details:** PDF/A is ISO 19005: PDF/A-2 is part 2 and PDF/A-3 part 3. PDF/A-2, PDF/A-3 and PDF/UA-1 all build on PDF 1.7, which is why a PDF 2.0 document can't claim any of them.

Next: open a PDF and change it, in [Edit a PDF](../../pages/guides/edit-a-pdf.md).
