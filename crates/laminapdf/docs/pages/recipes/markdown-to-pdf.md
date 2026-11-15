---
title: "Turn Markdown into a PDF"
description: "A report written in Markdown, as tagged pages that flow, from the command line, TypeScript or Rust, with tables, footnotes, images and a contents page."
version: "0.4.0"
url: "https://docs.laminapdf.com/recipes/markdown-to-pdf/"
---

# Turn Markdown into a PDF

A report written in Markdown, as tagged pages that flow, from the command line, TypeScript or Rust, with tables, footnotes, images and a contents page.

You'll turn a report written in Markdown into an accessible PDF: headings, lists, a table that runs across pages, a photo, footnotes, a contents page, and paragraphs in Arabic and Hindi. Use it for anything you already write in Markdown, such as notes, documentation or a report an LLM drafted.

From the command line, one command builds it:

```sh
laminapdf build report.md -o report.pdf \
  --font regular=NotoSans-Regular.ttf --font bold=NotoSans-Bold.ttf \
  --font fallback=NotoSansArabic-Regular.ttf --font fallback=NotoSansDevanagari-Regular.ttf \
  --profile pdf-ua-1
```

The command reads the images the Markdown names from the folder the Markdown is in. Add `--json` for a report of the pages, the size and anything the PDF left out.

In code, the function passes the Markdown, the fonts and the photo to `builder.markdown` (`Builder::markdown` in Rust) and returns the PDF's bytes. In TypeScript it needs the text module, `init({ text: true })`. Both languages write the same bytes:

**TypeScript**

`packages/sdk/examples/markdown-to-pdf.ts`:

```ts
// A report written in Markdown, turned into tagged pages that flow: headings, paragraphs with
// emphasis, code and links, a photo, lists and a task list, a block quote, a table that runs
// across pages, a code block, footnotes, and paragraphs in Arabic and Hindi.
//
// The front matter gives the title and language, a contents page lists the headings, and the
// output meets PDF/UA-1, so every font is embedded. The builder reads no files: the fonts and the
// one photo the Markdown names are passed in, and nothing is fetched.
import { Builder, type BuilderOptions, Font, Image } from "@laminapdf/sdk";

/** The fonts the report is set in: Latin, a bold face, Arabic and Devanagari. */
export interface ReportFonts {
  readonly regular: Uint8Array;
  /** A collection whose second face (index 1) is the bold one. */
  readonly bold: Uint8Array;
  readonly arabic: Uint8Array;
  readonly devanagari: Uint8Array;
}

/**
 * The report's bytes. Needs `init({ text: true })`.
 *
 * @param markdown - the report's Markdown.
 * @param fonts - the fonts to set it in.
 * @param photo - the JPEG the Markdown shows as `photo.jpg`.
 * @param options - can fix the file ID, the producer and the creation date.
 */
export function markdownToPdf(
  markdown: string,
  fonts: ReportFonts,
  photo: Uint8Array,
  options: BuilderOptions = {},
): Uint8Array<ArrayBuffer> {
  using b = new Builder({ ...options, profiles: ["PDF/UA-1"] });
  const report = b.markdown(markdown, {
    fonts: {
      regular: Font.fromBytes(fonts.regular),
      bold: Font.fromCollection(fonts.bold, 1),
      fallbacks: [Font.fromBytes(fonts.arabic), Font.fromBytes(fonts.devanagari)],
    },
    images: { "photo.jpg": Image.jpeg(photo) },
    // A5 pages, so the table runs across two of them, with the page number in the footer.
    page: { size: "A5", footer: { center: "Page {page} of {pages}" }, furnitureSize: 8 },
    contents: 2,
  });
  using doc = b.finish();
  const [warning] = doc.warnings;
  if (warning !== undefined) throw new Error(`the Markdown has something the PDF leaves out: ${warning.code}`);
  if (report.pages.length === 0) throw new Error("the report has no pages");
  return doc.save();
}
```

**Rust**

`crates/laminapdf/examples/markdown_to_pdf.rs`:

```rust
//! A report written in Markdown, turned into tagged pages that flow: headings, paragraphs with
//! emphasis, code and links, a photo, lists and a task list, a block quote, a table that runs
//! across pages, a code block, footnotes, and paragraphs in Arabic and Hindi.
//!
//! The front matter gives the title and language, a contents page lists the headings, and the
//! output is checked against PDF/UA-1, so every font is embedded. Run it with `cargo run -p
//! laminapdf --example markdown_to_pdf -- report.md regular.ttf bold.ttc arabic.ttf
//! devanagari.ttf photo.jpg`.

use std::error::Error;

use laminapdf::flow::{PageRegion, PageTemplate, TextStyle};
use laminapdf::markdown::{MarkdownBuildOptions, MarkdownFonts};
use laminapdf::{BuildOptions, Builder, Font, Image, PageSize, Profile, SaveOptions};

fn main() -> Result<(), Box<dyn Error>> {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    let [markdown, regular, bold, arabic, hindi, photo] = paths.as_slice() else {
        return Err("arguments: <report.md> <regular.ttf> <bold.ttc> <arabic.ttf> <devanagari.ttf> <photo.jpg>".into());
    };
    let read = |path: &String| std::fs::read(path);
    let fonts = [read(regular)?, read(bold)?, read(arabic)?, read(hindi)?];
    let markdown = std::fs::read_to_string(markdown)?;
    let pdf = build(BuildOptions::new(), &markdown, fonts, read(photo)?)?;
    std::fs::create_dir_all("target/examples-out")?;
    Ok(std::fs::write("target/examples-out/markdown_to_pdf.pdf", pdf)?)
}

/// The report's bytes. `options` can fix the file ID, the producer and the creation date.
pub(crate) fn build(
    options: BuildOptions,
    markdown: &str,
    fonts: [Vec<u8>; 4],
    photo: Vec<u8>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let [regular, bold, arabic, hindi] = fonts;
    let mut b = Builder::new(options.with_profile(Profile::PdfUa1));
    let regular = b.font(Font::from_bytes(regular)?)?;
    let fonts = MarkdownFonts::new(&regular)
        .with_bold(&b.font(Font::from_collection(bold, 1)?)?)
        .with_fallback(&b.font(Font::from_bytes(arabic)?)?)
        .with_fallback(&b.font(Font::from_bytes(hindi)?)?);
    // A5 pages, so the table runs across two of them, with the page number in the footer.
    let footer = PageRegion::new().with_center("Page {page} of {pages}");
    let pages = PageTemplate::new(PageSize::A5).with_furniture(None, Some(footer), &TextStyle::new(&regular, 8.0));
    let options = MarkdownBuildOptions::new()
        .with_fonts(fonts)
        .with_template(pages)
        .with_contents(2)
        .with_image("photo.jpg", Image::jpeg(photo)?);
    let report = b.markdown(markdown, &options)?;
    if let Some(warning) = report.warnings.first() {
        return Err(format!("the Markdown has something the PDF leaves out: {}", warning.code()).into());
    }
    Ok(b.finish()?.save(&SaveOptions::new())?.bytes)
}
```

**Result:** [markdown_to_pdf.pdf](https://docs.laminapdf.com/examples/pdf/markdown_to_pdf.pdf)

The result is four A5 pages that meet PDF/UA-1. The Result tab shows them, with a link to the PDF.

## How it works

- Each Markdown construct becomes the block you would lay out by hand: a `#` heading becomes a heading, a list a list, and a table a table whose header row repeats on each page. The [pages that flow](../../pages/guides/flowing-pages.md) guide shows those blocks.
- The front matter, plain `key: value` lines between two `---` lines, sets the title, language, author and subject. A screen reader uses the language to choose its voice.
- The fonts come by role: body text, bold (headings too), italic, code, and fallbacks for characters the others lack. Here the Arabic and Devanagari fonts are fallbacks, so each paragraph in those scripts finds its letters.
- Nothing is fetched. An image you don't pass, or a remote one, is left out with the warning `markdown-image-missing`, and raw HTML with `markdown-html-ignored`. Both name a position in the Markdown, never its text.
- The PDF is tagged: headings, lists, tables, figures with the image's text as their alternate text, and footnotes, in reading order.

## What to change

- **Latin text only:** leave out the fonts, and the text is set in Helvetica and Courier, which every PDF reader has. They aren't embedded, so PDF/UA-1 refuses them with `font-not-embedded`; drop the profile, or pass your own fonts.
- **The page size:** `--page-size letter` on the command line, `page: { size: "Letter" }` in TypeScript, or a `PageTemplate` in Rust. The template also sets the margins, a header and the footer.
- **No contents page:** remove `contents` (`with_contents` in Rust). The command line never adds one.
- **An AI agent:** `laminapdf mcp --root DIR --allow-write` serves the `laminapdf_build` tool to agents that speak MCP. It takes the Markdown inline or as a file, reads fonts and images only inside `DIR`, and writes the PDF there as a new file. Inline Markdown reads no images; pass a file for those.

Next: [a PDF/UA report](../../pages/recipes/pdf-ua-report.md), drawn block by block instead of written in Markdown.
