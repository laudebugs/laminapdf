---
title: "A PDF/A archive"
description: "An order confirmation for long-term archiving, written as PDF/A-2b and as PDF/A-2u that is also PDF/UA-1, with an embedded font and a photo."
version: "0.2.0-alpha.4"
url: "https://docs.laminapdf.com/recipes/pdf-a-archive/"
---

# A PDF/A archive

An order confirmation for long-term archiving, written as PDF/A-2b and as PDF/A-2u that is also PDF/UA-1, with an embedded font and a photo.

You'll write a letter confirming an order as a PDF/A file, which looks the same in any viewer for decades. You need it for documents kept for years, often by law: contracts, records, letters to customers. The recipe writes the letter twice, at two PDF/A levels.

The function takes the level, `2b` or `2u`, and builds the same letter under that level's profiles. In TypeScript it needs the text module, `init({ text: true })`. Both languages write the same bytes:

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

The result is one A4 letter under a translucent blue band, with a photo beside its text: the Result tab shows both versions, PDF/A-2b and PDF/A-2u, with links to the PDFs.

## How it works

- PDF/A-2b makes the letter look the same everywhere. PDF/A-2u also maps every glyph to Unicode, so the text can be searched and copied. That copy is PDF/UA-1 too, so a screen reader can follow it.
- PDF/A needs every font embedded, so the letter uses Noto Sans rather than one of the [standard 14 fonts](../../pages/reference/glossary.md#standard-14-fonts). `finish()` checks rules like this one and fails with a coded error for the first one the letter breaks.
- The builder writes what PDF/A asks for: the PDF/A identification in the file's XMP metadata, and an sRGB output intent that says what its RGB colours mean. See [What PDF/A asks for](../../pages/guides/archival-pdfs.md#what-pdfa-asks-for).
- Both copies draw the same tagged content: an `H1` title, `P` paragraphs and the photo as a figure with alternate text. PDF/A-2b doesn't ask for tags, but PDF/UA-1 does.
- The blue band is drawn at 35% opacity, as a layout artifact that screen readers skip. PDF/A-2 allows transparency.
- `textBox` breaks the letter into lines inside its box, beside the photo. The JPEG goes into the file as it is.

## What to change

- **The letter:** change `LETTER`. `textBox` leaves out lines that don't fit, so make `BLOCK_HEIGHT` larger for a longer letter, or pass `overflow: "error"` to fail with `text-overflow` instead.
- **The photo:** pass any RGB or gray JPEG, or `Image.png(bytes)` for a PNG (`Image::png` in Rust). A CMYK JPEG fails `finish()` with `device-color-without-output-intent`, because the output intent is sRGB.
- **Other profiles:** pass the ones you need, such as `["PDF/A-2u"]` alone or `["PDF/A-2b", "PDF/UA-1"]`. A document can claim PDF/UA-1 and at most one PDF/A profile.
- **An attached file:** PDF/A-2 refuses attachments, so switch to PDF/A-3b, as the [PDF/A-3 invoice](../../pages/recipes/pdf-a3-invoice-with-xml.md) does.

Next: [a PDF/A-3 invoice with XML](../../pages/recipes/pdf-a3-invoice-with-xml.md), with its data attached for software to read.
