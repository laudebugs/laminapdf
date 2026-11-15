---
title: "Turn photos into a PDF"
description: "Photos into a PDF, one page each, fitted on A4 with margins, in TypeScript or Rust, with each JPEG embedded as it is."
version: "0.4.0"
url: "https://docs.laminapdf.com/recipes/images-to-pdf/"
---

# Turn photos into a PDF

Photos into a PDF, one page each, fitted on A4 with margins, in TypeScript or Rust, with each JPEG embedded as it is.

You'll put photos into one PDF, one A4 page each, every photo fitted inside a 15 mm margin. Use it for scans from a phone, receipts for an expense claim, or pictures to print as one file.

The function passes the photos to `imagesToPdf` (`images_to_pdf` in Rust) and returns the PDF's bytes. In TypeScript it needs the extract or the text module, `init({ extract: true })`. Both languages write the same bytes:

**TypeScript**

`packages/sdk/examples/images-to-pdf.ts`:

```ts
// Photos into a PDF, one A4 page each, fitted inside the margins.
//
// Each JPEG is embedded as it is, without decoding or compressing it again. A page turns to
// landscape for a wide photo, and a photo that its camera marked as taken sideways is drawn
// upright.
import { Image, type ImagesToPdfOptions, imagesToPdf } from "@laminapdf/sdk";

/** The space around each photo, in millimetres. */
const MARGIN_MM = 15;
/** Points in an inch, and millimetres in an inch. */
const POINTS_PER_INCH = 72;
const MM_PER_INCH = 25.4;

/**
 * The document's bytes, one page per photo.
 *
 * @param photos - the JPEGs, one per page, in order.
 * @param options - can fix the file ID, the producer and the creation date.
 */
export function imagesToPdfBytes(
  photos: readonly Uint8Array[],
  options: Omit<ImagesToPdfOptions, "fit" | "margin"> = {},
): Uint8Array<ArrayBuffer> {
  using doc = imagesToPdf(
    photos.map((photo) => Image.jpeg(photo)),
    { ...options, fit: "A4", margin: (MARGIN_MM * POINTS_PER_INCH) / MM_PER_INCH },
  );
  return doc.save();
}
```

**Rust**

`crates/laminapdf/examples/images_to_pdf.rs`:

````rust
//! Photos into a PDF, one A4 page each, fitted inside the margins.
//!
//! Each JPEG is embedded as it is, without decoding or compressing it again. A page turns to
//! landscape for a wide photo, and a photo that its camera marked as taken sideways is drawn
//! upright.
//!
//! ```sh
//! cargo run -p laminapdf --example images_to_pdf -- out-dir photo.jpg [more.jpg …]   # writes out-dir/images_to_pdf.pdf
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{BuildOptions, Image, ImagePages, ImagesOptions, PageSize, Pt, SaveOptions, images_to_pdf};

/// The space around each photo, in millimetres.
const MARGIN_MM: f64 = 15.0;
const USAGE: &str = "usage: images_to_pdf <out-dir> <photo.jpg> [more.jpg …]";

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().ok_or(USAGE)?);
    let photos = args.map(std::fs::read).collect::<Result<Vec<_>, _>>()?;
    if photos.is_empty() {
        return Err(USAGE.into());
    }
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("images_to_pdf.pdf"), build(BuildOptions::new(), photos)?)?;
    Ok(())
}

/// The document's bytes, one page per photo (JPEGs). `options` can fix the file ID, the producer
/// and the creation date.
pub(crate) fn build(options: BuildOptions, photos: Vec<Vec<u8>>) -> Result<Vec<u8>, Box<dyn Error>> {
    let images = photos.into_iter().map(Image::jpeg).collect::<Result<Vec<_>, _>>()?;
    let options = ImagesOptions::new()
        .with_pages(ImagePages::Fit(PageSize::A4))
        .with_margin(Pt::from_mm(MARGIN_MM))
        .with_build(options);
    let mut doc = images_to_pdf(&images, &options)?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
````

**Result:** [images_to_pdf.pdf](https://docs.laminapdf.com/examples/pdf/images_to_pdf.pdf)

The result has two pages: a colour photo and a greyscale one. The Result tab shows them, with a link to the PDF.

## How it works

- Each JPEG goes into the file as it is, without decoding or compressing it again, so the page keeps every pixel the camera wrote and the PDF is barely larger than the photos.
- A PNG is decoded and kept with its transparency.
- A page turns to landscape for a photo wider than it is tall, and the photo is scaled to fit inside the margins and centred.
- A phone often stores a photo sideways and records which way is up in its EXIF data. The page shows it the way up the EXIF data says.

## What to change

- **Another paper size:** pass `fit: "Letter"`, or `[width, height]` in points (`ImagePages::Fit(PageSize::LETTER)` in Rust).
- **Pages the photos' own size:** leave out `fit` (`ImagePages::ImageSize` in Rust). Each page is then its photo's size at the resolution its file records, or at `defaultDpi` (72 by default) when the file records none.
- **No margin:** leave out `margin`, and each photo fills its page edge to edge in one direction.
- **PNG images:** `Image.png(bytes)` in place of `Image.jpeg(bytes)` (`Image::png` in Rust).

Next: [images and vectors](../../pages/recipes/images-and-vectors.md), which draws images, paths and transparency on one page.
