---
title: "Hello, world"
description: "One A4 page with a line of text in a standard 14 font, the smallest PDF Lamina PDF writes, in TypeScript and Rust."
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/recipes/hello/"
---

# Hello, world

One A4 page with a line of text in a standard 14 font, the smallest PDF Lamina PDF writes, in TypeScript and Rust.

You'll make the smallest PDF Lamina PDF writes: one A4 page that says "Hello, world!" in Helvetica. Start from it to check that a new setup works, or as the skeleton of a document of your own.

The function builds the page and returns the PDF's bytes. In TypeScript it needs the text module, `init({ text: true })`. Both languages write the same bytes:

**TypeScript**

`packages/sdk/examples/hello.ts`:

```ts
// The smallest document: one A4 page with a line of text in a standard 14 font.
import { Builder, type BuilderOptions, Font } from "@laminapdf/sdk";

/**
 * The document's bytes. Needs `init({ text: true })`.
 *
 * @param options - can fix the file ID, the producer and the creation date, so that the same
 *   input gives the same bytes on every run.
 */
export function hello(options: BuilderOptions = {}): Uint8Array<ArrayBuffer> {
  using b = new Builder(options);
  const page = b.page("A4");
  // Page space: points from the top-left corner, y down; `at[1]` is the text's baseline.
  page.surface().text("Hello, world!", { font: Font.standard("Helvetica"), size: 24, at: [72, 96] });
  page.finish();
  using doc = b.finish();
  return doc.save();
}
```

**Rust**

`crates/laminapdf/examples/hello.rs`:

````rust
//! The smallest document: one A4 page with a line of text in a standard 14 font.
//!
//! ```sh
//! cargo run -p laminapdf --example hello -- [out-dir]   # writes <out-dir>/hello.pdf (default target/examples-out)
//! ```

use std::error::Error;
use std::path::PathBuf;

use laminapdf::{BuildOptions, Builder, PageSize, PointPt, Pt, SaveOptions, Standard14};

fn main() -> Result<(), Box<dyn Error>> {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("hello.pdf"), build(BuildOptions::new())?)?;
    Ok(())
}

/// The document's bytes. `options` can fix the file ID, the producer and the creation date, so
/// that the same input gives the same bytes on every run.
pub(crate) fn build(options: BuildOptions) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut b = Builder::new(options);
    let helvetica = b.standard_font(Standard14::Helvetica)?;
    let mut page = b.page(PageSize::A4);
    // Page space: points from the top-left corner, y down; `y` is the text's baseline.
    page.surface().text(
        "Hello, world!",
        &helvetica,
        Pt::new(24.0),
        PointPt::from_points(72.0, 96.0),
    )?;
    page.finish()?;
    let mut doc = b.finish()?;
    Ok(doc.save(&SaveOptions::new())?.bytes)
}
````

**Result:** [hello.pdf](https://docs.laminapdf.com/examples/pdf/hello.pdf)

The result is one page with the line near its top-left corner. The Result tab shows it, with a link to the PDF.

## How it works

- The builder makes an A4 page, and the page's surface is what you draw on.
- `at` is in points from the top-left corner of the page, with y going down. Its second number is the text's baseline. See [Pages and coordinates](../../pages/concepts/pages-and-coordinates.md).
- Helvetica is one of the [standard 14 fonts](../../pages/reference/glossary.md#standard-14-fonts), which every PDF viewer has, so the PDF embeds no font.
- Finishing the builder gives a document, and saving the document gives its bytes. `using` in TypeScript frees both as soon as the function ends.

## What to change

- **The text and where it goes:** change the string, the size and `at`.
- **The page size:** name another size, such as `"Letter"`, or pass `[width, height]` in points.
- **The font:** for characters outside the standard 14 fonts' Latin set, embed a font. See [Text and fonts](../../pages/guides/fonts-and-text.md).
- **A file ID or a date of your own:** pass them in the builder's options. Without them, the file ID is a hash of the content and the PDF has no creation date, so the same code writes the same bytes on every run.

Next: [an invoice](../../pages/recipes/invoice.md), with an embedded font and a table of line items.
