---
title: "Element flow"
description: "A report whose title, paragraphs and photo each start below wherever the one before ended, with a body that continues on a new page."
version: "0.3.0"
url: "https://docs.laminapdf.com/recipes/element-flow/"
---

# Element flow

A report whose title, paragraphs and photo each start below wherever the one before ended, with a body that continues on a new page.

You'll lay out a report where each part starts below wherever the part before it ended: a title, a summary, optional notes, a photo and a body that runs onto a second page. Use it when the length of your text changes from one document to the next.

The function takes a JPEG's bytes and the report's text, and returns the PDF's bytes. In TypeScript it draws text, so it needs `init({ text: true })`. Both languages write the same bytes:

**TypeScript**

`packages/sdk/examples/element-flow.ts`:

```ts
// A report whose title, paragraphs and photo each start below wherever the one before ended. Each
// text box reports its `bottom`, and the next element starts a gap below it, so a summary that
// wraps moves everything after it and empty notes take no space. The body continues on new pages
// from the top margin. Needs `init({ text: true })`.
import { Builder, type BuilderOptions, Font, Image, PAGE_SIZES, type Surface, type TextLayout } from "@laminapdf/sdk";

const [PAGE_WIDTH, PAGE_HEIGHT] = PAGE_SIZES.A5;
const MARGIN = 48;
const WIDTH = PAGE_WIDTH - 2 * MARGIN;
const BOTTOM = PAGE_HEIGHT - MARGIN;
const TITLE = 18;
const BODY = 11;
const GAP = 10;
/** The photo's height, for a 3:2 photo across the text column. */
const PHOTO_HEIGHT = WIDTH / 1.5;

/** What the report says. Empty `notes` take no space; blank lines separate the body's paragraphs. */
export interface Report {
  readonly title: string;
  readonly summary: string;
  readonly notes: string;
  readonly body: string;
}

/**
 * The report's bytes, with `photo` (a JPEG) below its notes.
 *
 * @param options - can fix the file ID, the producer and the creation date.
 */
export function elementFlow(photo: Uint8Array, report: Report, options: BuilderOptions = {}): Uint8Array<ArrayBuffer> {
  using b = new Builder(options);
  const fonts = { bold: Font.standard("Helvetica-Bold"), regular: Font.standard("Helvetica") };
  const jpeg = Image.jpeg(photo);
  let page = b.page("A5");
  let y = header(page.surface(), fonts, jpeg, report);
  let [rest, newPage] = [report.body.trimEnd(), false];
  for (;;) {
    const layout = page.surface().textBox(rest, { font: fonts.regular, size: BODY, box: area(y) });
    // A clipped box still drew its first lines: the rest goes on a new page, from its top.
    const cut = layout.overflow;
    if (cut === undefined) break;
    if (cut === 0 && newPage) throw new Error("a line of the body is taller than the page");
    [rest, y, newPage] = [rest.slice(cut).trimStart(), MARGIN, true];
    page.finish();
    page = b.page("A5");
  }
  page.finish();
  using doc = b.finish();
  return doc.save();
}

/**
 * The title with a rule under it, the summary, the notes and the photo, each below the one before;
 * returns where the body starts. Text that doesn't fit throws `text-overflow`.
 */
function header(s: Surface, fonts: { bold: Font; regular: Font }, photo: Image, report: Report): number {
  const fit = (text: string, font: Font, size: number, top: number) =>
    below(s.textBox(text, { font, size, box: area(top), overflow: "error" }), top);
  let y = fit(report.title, fonts.bold, TITLE, MARGIN);
  // A decoration half a gap above the summary. It's stroked: a fill colour would colour the text too.
  s.strokeColor([0.6])
    .path((p) => p.rect([MARGIN, y - GAP / 2, WIDTH, 0]))
    .stroke();
  for (const text of [report.summary, report.notes]) y = fit(text, fonts.regular, BODY, y);
  if (y + PHOTO_HEIGHT > BOTTOM) throw new Error("the photo doesn't fit below the notes");
  s.image(photo, [MARGIN, y, WIDTH, PHOTO_HEIGHT]);
  return y + PHOTO_HEIGHT + GAP;
}

/** Where the next element starts: a gap below `layout`'s last drawn line, or `y` if it drew none. */
function below(layout: TextLayout, y: number): number {
  return layout.bottom === undefined ? y : layout.bottom + GAP;
}

/** The rest of the page's text column, from `y` down to the bottom margin. */
function area(y: number): [number, number, number, number] {
  return [MARGIN, y, WIDTH, Math.max(BOTTOM - y, 0)];
}
```

**Rust**

`crates/laminapdf/examples/element_flow.rs`:

```rust
//! A report whose title, paragraphs and photo each start below wherever the one before ended.
//!
//! Each text box reports its `bottom`, and the next element starts a gap below it, so a summary
//! that wraps moves everything after it and empty notes take no space. The body continues on new
//! pages from the top margin. Run it with `cargo run -p laminapdf --example element_flow -- photo.jpg`.

use std::error::Error;

use laminapdf::{
    BuildOptions, Builder, Color, FontHandle, Image, Overflow, PageSize, Pt, RectPt, SaveOptions,
    Standard14::{Helvetica, HelveticaBold},
    StrokeStyle, Surface, TextBoxOptions, TextLayout,
};

const PAGE: PageSize = PageSize::A5;
const MARGIN: f64 = 48.0;
const WIDTH: f64 = PAGE.width.get() - 2.0 * MARGIN;
const BOTTOM: f64 = PAGE.height.get() - MARGIN;
const TITLE: Pt = Pt::new(18.0);
const BODY: Pt = Pt::new(11.0);
const GAP: f64 = 10.0;
const PHOTO_HEIGHT: f64 = WIDTH / 1.5; // a 3:2 photo across the text column

/// What the report says. Empty `notes` take no space; blank lines separate the body's paragraphs.
pub(crate) struct Report<'a> {
    pub(crate) title: &'a str,
    pub(crate) summary: &'a str,
    pub(crate) notes: &'a str,
    pub(crate) body: &'a str,
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = std::env::args().skip(1);
    let photo = std::fs::read(args.next().ok_or("usage: element_flow <photo.jpg> [out-dir]")?)?;
    let out = std::path::PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    let report = Report {
        title: "Third-quarter report",
        summary: "Revenue rose twelve per cent over the quarter, led by the new reader and its offline mode.",
        notes: "Figures are unaudited.",
        body: "Pages now appear as soon as they are read, and support requests about slow files fell by a third.",
    };
    let pdf = build(BuildOptions::new(), photo, &report)?;
    std::fs::create_dir_all(&out)?;
    Ok(std::fs::write(out.join("element_flow.pdf"), pdf)?)
}

/// The report's bytes, with `photo` (a JPEG) below its notes. `options` can fix the file ID, the
/// producer and the creation date.
pub(crate) fn build(options: BuildOptions, photo: Vec<u8>, report: &Report) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut b = Builder::new(options);
    let fonts = [b.standard_font(HelveticaBold)?, b.standard_font(Helvetica)?];
    let photo = Image::jpeg(photo)?;
    let mut page = b.page(PAGE);
    let mut y = header(page.surface(), &fonts, &photo, report)?;
    let (mut rest, mut new_page) = (report.body.trim_end(), false);
    loop {
        let s = page.surface();
        let layout = s.text_box(rest, &fonts[1], BODY, area(y), &TextBoxOptions::new())?;
        // A clipped box still drew its first lines: the rest goes on a new page, from its top.
        let Some(cut) = layout.overflow else { break };
        if cut == 0 && new_page {
            return Err("a line of the body is taller than the page".into());
        }
        (rest, y, new_page) = (rest[cut..].trim_start(), MARGIN, true);
        page.finish()?;
        page = b.page(PAGE);
    }
    page.finish()?;
    Ok(b.finish()?.save(&SaveOptions::new())?.bytes)
}

/// The title with a rule under it, the summary, the notes and the photo, each below the one
/// before; returns where the body starts. Text that doesn't fit fails `text-overflow`.
fn header(s: &mut Surface, fonts: &[FontHandle; 2], photo: &Image, report: &Report) -> Result<f64, Box<dyn Error>> {
    let [bold, regular] = fonts;
    let must_fit = TextBoxOptions::new().with_overflow(Overflow::Error);
    let mut y = below(&s.text_box(report.title, bold, TITLE, area(MARGIN), &must_fit)?, MARGIN);
    // A decoration half a gap above the summary. It's stroked: a fill colour would colour the text too.
    let (rule, gray) = (RectPt::from_points(MARGIN, y - GAP / 2.0, WIDTH, 0.0), Color::gray(0.6));
    let line = StrokeStyle::default();
    s.stroke_color(gray).path(|p| _ = p.rect(rule)).stroke(&line);
    for text in [report.summary, report.notes] {
        y = below(&s.text_box(text, regular, BODY, area(y), &must_fit)?, y);
    }
    if y + PHOTO_HEIGHT > BOTTOM {
        return Err("the photo doesn't fit below the notes".into());
    }
    s.image(photo, RectPt::from_points(MARGIN, y, WIDTH, PHOTO_HEIGHT));
    Ok(y + PHOTO_HEIGHT + GAP)
}

/// Where the next element starts: a gap below `layout`'s last drawn line, or `y` if it drew none.
fn below(layout: &TextLayout, y: f64) -> f64 {
    layout.bottom().map_or(y, |bottom| bottom.get() + GAP)
}

/// The rest of the page's text column, from `y` down to the bottom margin.
fn area(y: f64) -> RectPt {
    RectPt::from_points(MARGIN, y, WIDTH, (BOTTOM - y).max(0.0))
}
```

**Result:** [element_flow.pdf](https://docs.laminapdf.com/examples/pdf/element_flow.pdf)

The result is two A5 pages. The body fills the first page below the photo and finishes at the top of the second. The Result tab shows it, with a link to the PDF.

## How it works

Every text box tells you where its text ended, so you never type a position for anything after the first element.

- **`bottom` is where the next element can start.** `textBox` returns it with the lines it drew: the bottom of the last line that draws something. Rust has `TextLayout::bottom()`, and `TextRun::bottom()` for a single run of text.
- **Empty text has no `bottom`.** The notes here are optional. When they're empty, `bottom` is `undefined` (`None` in Rust), `below` keeps `y` where it was, and the photo moves up with no gap left behind.
- **A longer summary moves everything after it.** When the summary wraps onto one more line, the notes, the photo and the body each move down by one line height, and none of the code changes.
- **The photo is placed by your own rectangle.** An image reports nothing back, so the code adds the photo's height and the gap to `y` itself.
- **The rule under the title is a decoration.** It's drawn half a gap above the summary and leaves `y` alone. It's stroked rather than filled: a fill colour stays set and would colour the text drawn after it.

## Text that doesn't fit

The report treats overflow two ways, and you choose which for each element.

The title, summary and notes must fit on the first page. They're drawn with `overflow: "error"` (`Overflow::Error` in Rust), so text that doesn't fit throws `text-overflow` instead of disappearing. The photo must fit too: when it would cross the bottom margin, the code stops with an error of its own.

The body may run on. Its box reaches the bottom margin and clips what doesn't fit. Then `overflow` holds the offset of the first character it didn't draw: a UTF-16 offset in TypeScript, a byte offset in Rust. The loop starts a new page and draws the rest from the top margin. Test `overflow` with `!== undefined`, because `0` means not even one line fitted. If no line fits even on a fresh page, a line is taller than the page, so the loop stops with an error instead of adding pages for ever.

## What `bottom` measures

`bottom` is a typographic measurement in points: the descent of the last drawn line's font, below its baseline. A glyph that reaches lower, or a thick stroke around the text, can paint past it.

Like every position, `bottom` is in [page space](../../pages/reference/glossary.md#page-space), measured from the top of the page. It's measured before the surface's transform, so inside `transform` it stays in the coordinates you drew in.

In TypeScript you pass a box as a tuple, `[x, y, width, height]`. The lines a text box returns describe their `rect` as an object with `x`, `y`, `width` and `height`.

## What stays your job

Page breaks stay your decision: the code here makes each one from `overflow`. The builder's surface doesn't paginate, float an image beside text, or lay out tables. [Pages that flow](../../pages/guides/flowing-pages.md) paginates for you, in both languages: headings kept with their text, running headers and "Page X of Y" footers.

## What to change

- **Paragraphs a gap apart:** draw each paragraph in its own text box and start the next one at `bottom` plus the gap, as the summary and notes do.
- **A line height for the body:** pass `lineHeight` (`with_line_height` in Rust); `bottom` follows it.
- **Two columns:** keep a `y` for each column, and when the left column fills, carry on at the top margin of the right one.

Next: [the PDF/UA report](../../pages/recipes/pdf-ua-report.md), a report that screen readers can follow.
