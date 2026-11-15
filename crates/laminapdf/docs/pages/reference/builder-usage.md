---
title: "Using Builder"
description: "The tasks a Builder is for, from a first page of text to accessible and archival PDFs, each with a short example, and the mistakes to avoid."
version: "0.4.0"
url: "https://docs.laminapdf.com/reference/builder-usage/"
---

# Using Builder

The tasks a Builder is for, from a first page of text to accessible and archival PDFs, each with a short example, and the mistakes to avoid.

A `Builder` makes a new PDF: you add pages, draw on them, and finish into a `Document` you save. Each task below has a short example and a link to the guide that goes further. The [`Builder` reference](https://docs.laminapdf.com/reference/sdk/classes/builder/) lists every method and option.

## Draw a page

`b.page("A4")` adds a page, `page.surface()` is what you draw on, and `page.finish()` writes the page into the document. `b.finish()` then returns the new document, and `doc.save()` its bytes:

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

Positions are in points (1/72 inch) from the page's top-left corner, with y going down. [Pages and coordinates](../../pages/concepts/pages-and-coordinates.md) has the details, rotated pages included.

## Make several pages

Call `b.page()` once per page, and finish each page before the builder. This example gives each page a heading and a "Page 2 of 3" footer:

`packages/sdk/examples/page-numbers.ts`:

```ts
// A document of several pages, each with a heading and a footer that reads "Page 2 of 3". A page
// takes no more drawing once it is finished, so the page count is known before the first page is
// drawn.
import { Builder, type BuilderOptions, Font, PAGE_SIZES } from "@laminapdf/sdk";

const MARGIN = 72;
const HEADING_SIZE = 20;
const FOOTER_SIZE = 10;
const PAGE_SIZE = "A4";
const [, PAGE_HEIGHT] = PAGE_SIZES[PAGE_SIZE];

/**
 * One page per heading in `headings`. Needs `init({ text: true })`.
 *
 * @param options - can fix the file ID, the producer and the creation date.
 */
export function pageNumbers(headings: readonly string[], options: BuilderOptions = {}): Uint8Array<ArrayBuffer> {
  using b = new Builder(options);
  const font = Font.standard("Helvetica");
  headings.forEach((heading, index) => {
    const page = b.page(PAGE_SIZE);
    const s = page.surface();
    // `at` is the baseline's start, in points from the page's top-left corner.
    s.text(heading, { font, size: HEADING_SIZE, at: [MARGIN, MARGIN + HEADING_SIZE] });
    const footer = `Page ${index + 1} of ${headings.length}`;
    s.text(footer, { font, size: FOOTER_SIZE, at: [MARGIN, PAGE_HEIGHT - MARGIN / 2] });
    // A page joins the document only once it is finished.
    page.finish();
  });
  b.metadata({ title: headings[0] ?? "Untitled" });
  using doc = b.finish();
  return doc.save();
}
```

When you don't know how many pages the content needs, let the builder decide: [pages that flow](../../pages/guides/flowing-pages.md) breaks headings, paragraphs and tables across pages, with headers and footers, and `b.markdown(text)` does the same for a Markdown file.

## Draw text, shapes and images

- `surface.text(str, { font, size, at })` draws text as it is, starting a new line only at each line break in it, and `surface.textBox(str, { font, size, box })` wraps a paragraph to fit a box. [Text and fonts](../../pages/guides/fonts-and-text.md) covers fonts, scripts and text that doesn't fit.
- `surface.path(p => …)` draws a shape you then fill, stroke or clip to, and `surface.image(image, rect)` places a JPEG, a PNG or raw pixels. [Shapes, colour and images](../../pages/guides/images-and-vectors.md) covers colour, opacity and transforms.

## Add links, bookmarks and metadata

`surface.link(rect, target)` makes a region of a page link to another page or a web address. On the builder, `b.outline(entries)` adds bookmarks, `b.pageLabels(ranges)` numbers pages `i, ii, 1, 2`, and `b.metadata({ title })` and `b.lang("en")` describe the document. [Links, outlines and metadata](../../pages/guides/links-outlines-and-metadata.md) has the example.

## Build to a standard

`new Builder({ profiles: ["PDF/UA-1"] })` makes a document for screen readers, and `"PDF/A-2b"` one for archives. Under a profile, `b.finish()` checks the rules a program can check and fails with the code of the first one the document breaks. [Accessible PDFs](../../pages/guides/accessible-pdfs.md) and [archival PDFs](../../pages/guides/archival-pdfs.md) list the rules.

## Mistakes to avoid

- **Text needs the text module.** Load it with `init({ text: true })` before you create the builder. Without it, a builder draws shapes and images only, and `text` fails with `text-module-not-loaded`.
- **`at` is the baseline.** `at: [x, y]` places the start of the line the letters sit on, so the letters rise above `y`. Text at `y: 0` is drawn above the page's top edge, out of sight.
- **An unfinished page is left out.** A page joins the document only when you call `page.finish()`. Pages still open when the builder finishes are dropped.
- **A finished page takes no more drawing.** Draw everything a page needs, such as a footer with the page count, before you finish it. Drawing after that fails with `builder-finished`.
- **Some mistakes show up late.** A shape with invalid values fails at `page.finish()` with `invalid-geometry`. A link to a page or destination that doesn't exist fails at `b.finish()`, as does a profile's broken rule.
- **The [standard 14 fonts](../../pages/reference/glossary.md#standard-14-fonts) are Latin only.** `Font.standard("Helvetica")` needs no font file, but it covers Western European text and isn't embedded, so PDF/UA and PDF/A refuse it with `font-not-embedded`. Load your own font with `Font.fromBytes(bytes)` for other scripts and for those profiles.
- **Dates come from you.** Lamina PDF never reads the clock, so a document has a creation date only when you pass `creationDate`. The same content and options give the same bytes on every run; [test the PDFs you generate](../../pages/guides/test-generated-pdfs.md) builds on that.
- **Free the builder and the document.** `b.finish()` frees the builder, but a builder that throws before it finishes, such as on a missing glyph, holds its memory until the garbage collector reaches it. Declare it with `using b = new Builder()`, as the examples do, so it is freed either way. The document `b.finish()` returns holds its own memory, so write `using doc = b.finish()` too.

Next: [the `Builder` reference](https://docs.laminapdf.com/reference/sdk/classes/builder/).
