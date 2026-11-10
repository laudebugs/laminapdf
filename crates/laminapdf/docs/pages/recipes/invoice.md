---
title: "An invoice"
description: "A tagged one-page invoice with an embedded font, a table of line items with right-aligned amounts and a total, in TypeScript and Rust."
version: "0.2.0-alpha.5"
url: "https://docs.laminapdf.com/recipes/invoice/"
---

# An invoice

A tagged one-page invoice with an embedded font, a table of line items with right-aligned amounts and a total, in TypeScript and Rust.

You'll make a one-page invoice: a heading, an address, a table of line items with right-aligned amounts, and a total. It's tagged, so a screen reader reads it in order. Start from it for any document built around a table, such as a quote or a receipt.

The function takes a font file, draws the invoice and returns the PDF's bytes. In TypeScript it needs the text module, `init({ text: true })`. Both languages write the same bytes:

**TypeScript**

`packages/sdk/examples/invoice.ts`:

```ts
// An invoice that screen readers can follow, with an embedded font and a table of line items.
// The heading, the address, the table of items and the total are tagged, so a screen reader
// reads them in order.
import { Builder, type BuilderOptions, Font, PAGE_SIZES, type Rect, type Surface } from "@laminapdf/sdk";

const ITEMS: readonly (readonly [item: string, qty: number, price: number])[] = [
  ["Widgets, blue (W-100)", 3, 12.0],
  ["Gadget with ﬁne-tuned dials", 1, 79.9],
  ["Café-grade sprockets", 12, 1.25],
  ["Delivery", 1, 4.5],
];
const ADDRESS = "Acme Widgets Ltd · 1 Example Street · London";
const PAGE = "A4";
const [PAGE_WIDTH] = PAGE_SIZES[PAGE];
const MARGIN = 56;
const RIGHT_EDGE = PAGE_WIDTH - MARGIN;
const TITLE_AT = [MARGIN, 80] as const;
const ADDRESS_AT = [MARGIN, 100] as const;
const TITLE = 24;
const BODY = 11;
const FIRST_ROW = 160;
const ROW_HEIGHT = 24;
/** The quantity's and the amount's columns, each a width ending this far left of the right edge. */
const COLUMNS = [
  [40, 169],
  [100, 0],
] as const;
/** A cell's box starts this far above its row's baseline and is this tall. */
const CELL_RISE = 12;
const CELL_HEIGHT = 16;

/**
 * The document's bytes, in `fontBytes` (a TrueType or OpenType file). Needs `init({ text: true })`.
 *
 * @param options - can fix the file ID, the producer and the creation date.
 */
export function invoice(fontBytes: Uint8Array, options: BuilderOptions = {}): Uint8Array<ArrayBuffer> {
  using b = new Builder({ ...options, tagged: true });
  const font = Font.fromBytes(fontBytes);
  const bold = Font.standard("Helvetica-Bold");
  // Registered up front, in this order, so the embedded font is the document's first.
  b.font(font);
  b.font(bold);
  const page = b.page(PAGE);
  const s = page.surface();
  s.text("Invoice 42", { font: bold, size: TITLE, at: TITLE_AT, tag: "H1" });
  s.text(ADDRESS, { font, size: BODY, at: ADDRESS_AT, tag: "P" });
  items(s, font);
  const total = ITEMS.reduce((sum, [, qty, price]) => sum + qty * price, 0);
  const box = cell(COLUMNS[1], rowBaseline(ITEMS.length));
  s.textBox(`Total € ${total.toFixed(2)}`, { font, size: BODY, box, align: "end", tag: "P" });
  page.finish();
  b.metadata({ title: "Invoice 42", author: "Acme Widgets Ltd" }).lang("en-GB");
  using doc = b.finish();
  return doc.save();
}

/** The table of line items: what, how many, and the amount, right-aligned. */
function items(s: Surface, font: Font): void {
  s.tag("Table", (s) => {
    ITEMS.forEach(([item, qty, price], row) => {
      const y = rowBaseline(row);
      const cells = [
        [String(qty), COLUMNS[0]],
        [`€ ${(qty * price).toFixed(2)}`, COLUMNS[1]],
      ] as const;
      s.tag("TR", (s) => {
        s.text(item, { font, size: BODY, at: [MARGIN, y], tag: "TD" });
        for (const [text, column] of cells) {
          s.textBox(text, { font, size: BODY, box: cell(column, y), align: "end", tag: "TD" });
        }
      });
    });
  });
}

/** The baseline of row `row` of the table. */
function rowBaseline(row: number): number {
  return FIRST_ROW + ROW_HEIGHT * row;
}

/** The box of a cell in `column` (width, distance from the right edge) on the row at `baseline`. */
function cell([width, inset]: readonly [number, number], baseline: number): Rect {
  return [RIGHT_EDGE - inset - width, baseline - CELL_RISE, width, CELL_HEIGHT];
}
```

**Rust**

`crates/laminapdf/examples/invoice.rs`:

```rust
//! An invoice that screen readers can follow, with an embedded font and a table of line items.
//!
//! The heading, the address, the table of items and the total are tagged, so a screen reader
//! reads them in order. Run it with `cargo run -p laminapdf --example invoice -- <font.ttf> [out-dir]`.

use laminapdf::{
    Alignment, BuildError, BuildOptions, Builder, FontHandle, Metadata, PageSize, PointPt, Pt, RectPt, SaveOptions,
    Standard14, Surface, TagKind, TextBoxOptions,
};

const ITEMS: [(&str, u32, f64); 4] = [
    ("Widgets, blue (W-100)", 3, 12.00),
    ("Gadget with ﬁne-tuned dials", 1, 79.90),
    ("Café-grade sprockets", 12, 1.25),
    ("Delivery", 1, 4.50),
];
const ADDRESS: &str = "Acme Widgets Ltd · 1 Example Street · London";
const PAGE: PageSize = PageSize::A4;
const MARGIN: f64 = 56.0;
const RIGHT_EDGE: f64 = PAGE.width.get() - MARGIN;
const TITLE_AT: PointPt = PointPt::from_points(MARGIN, 80.0);
const ADDRESS_AT: PointPt = PointPt::from_points(MARGIN, 100.0);
const TITLE: Pt = Pt::new(24.0);
const BODY: Pt = Pt::new(11.0);
const FIRST_ROW: f64 = 160.0;
const ROW_HEIGHT: f64 = 24.0;
/// The quantity's and the amount's columns, each a width ending this far left of the right edge.
const COLUMNS: [(f64, f64); 2] = [(40.0, 169.0), (100.0, 0.0)];
/// A cell's box starts this far above its row's baseline and is this tall.
const CELL_RISE: f64 = 12.0;
const CELL_HEIGHT: f64 = 16.0;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let font = std::fs::read(args.next().ok_or("usage: invoice <font.ttf> [out-dir]")?)?;
    let out = std::path::PathBuf::from(args.next().unwrap_or_else(|| "target/examples-out".into()));
    std::fs::create_dir_all(&out)?;
    let pdf = build(BuildOptions::new(), font)?;
    Ok(std::fs::write(out.join("invoice.pdf"), pdf)?)
}

/// The bytes, in `font` (TrueType or OpenType); `options` can fix the file ID, producer and date.
pub(crate) fn build(options: BuildOptions, font: Vec<u8>) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut b = Builder::new(options.with_tagged(true));
    draw(&mut b, font)?;
    Ok(b.finish()?.save(&SaveOptions::new())?.bytes)
}

/// Draws the invoice's one page into `b` and sets its title, author and language.
pub(crate) fn draw(b: &mut Builder, font: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    let (font, bold) = (b.font_bytes(font)?, b.standard_font(Standard14::HelveticaBold)?);
    let mut page = b.page(PAGE);
    let s = page.surface();
    s.text("Invoice 42", &bold, TITLE, TITLE_AT)?.tag(TagKind::H1);
    s.text(ADDRESS, &font, BODY, ADDRESS_AT)?.tag(TagKind::P);
    items(s, &font)?;
    let total: f64 = ITEMS.iter().map(|(_, qty, price)| f64::from(*qty) * price).sum();
    let (total, below_items) = (format!("Total € {total:.2}"), u32::try_from(ITEMS.len())?);
    let area = cell(COLUMNS[1], row_baseline(below_items));
    s.tag(TagKind::P, |s| right_aligned(s, &font, &total, area))?;
    page.finish()?;
    let metadata = Metadata::default().with_title("Invoice 42");
    b.metadata(metadata.with_author("Acme Widgets Ltd")).lang("en-GB");
    Ok(())
}

/// The table of line items: what, how many, and the amount, right-aligned.
fn items(s: &mut Surface, font: &FontHandle) -> Result<(), BuildError> {
    s.tag(TagKind::Table, |s| {
        for (row, (item, qty, price)) in (0_u32..).zip(ITEMS) {
            let y = row_baseline(row);
            s.tag(TagKind::TR, |s| {
                s.text(item, font, BODY, PointPt::from_points(MARGIN, y))?
                    .tag(TagKind::TD);
                let amount = format!("€ {:.2}", f64::from(qty) * price);
                for (text, column) in [qty.to_string(), amount].iter().zip(COLUMNS) {
                    s.tag(TagKind::TD, |s| right_aligned(s, font, text, cell(column, y)))?;
                }
                Ok(())
            })?;
        }
        Ok(())
    })
}

/// The baseline of row `row` of the table.
fn row_baseline(row: u32) -> f64 {
    FIRST_ROW + ROW_HEIGHT * f64::from(row)
}

/// The box of a cell in `column` (width, distance from the right edge) on the row at `baseline`.
fn cell((width, inset): (f64, f64), baseline: f64) -> RectPt {
    RectPt::from_points(RIGHT_EDGE - inset - width, baseline - CELL_RISE, width, CELL_HEIGHT)
}

/// `text` in `area`, in the body size, aligned to its right edge.
fn right_aligned(s: &mut Surface, font: &FontHandle, text: &str, area: RectPt) -> Result<(), BuildError> {
    let right = TextBoxOptions::new().with_align(Alignment::End);
    s.text_box(text, font, BODY, area, &right).map(|_| ())
}
```

**Result:** [invoice.pdf](https://docs.laminapdf.com/examples/pdf/invoice.pdf)

The result is one A4 page with its body set in Noto Sans. The Result tab shows it, with a link to the PDF.

## How it works

- The body uses your font file, and Lamina PDF embeds only the glyphs the invoice draws. The heading uses Helvetica Bold, one of the [standard 14 fonts](../../pages/reference/glossary.md#standard-14-fonts), so it embeds nothing.
- Each quantity and amount is a text box aligned to its `end`, so the numbers line up on the right. `cell` works out each box from its column's width and its distance from the right margin.
- `tagged: true` (`with_tagged(true)` in Rust) gives the document a structure for screen readers. The `tag` option and `s.tag(…)` make the heading an `H1`, the address and the total each a paragraph (`P`), and the items a `Table` of rows (`TR`) and cells (`TD`).
- `metadata` sets the title and author, and `lang("en-GB")` tells a screen reader which language to speak.

## What to change

- **The line items:** edit `ITEMS`. Each row sits `ROW_HEIGHT` points below the one before, and the total follows the last row.
- **The columns:** each pair in `COLUMNS` is a column's width and its distance from the right margin, in points.
- **The currency:** change the `€` in the amount and total strings. Your font needs a glyph for the new symbol, or drawing fails with `missing-glyph`. See [Text and fonts](../../pages/guides/fonts-and-text.md).
- **A PDF/UA-1 invoice:** add `profiles: ["PDF/UA-1"]` (`with_profile(Profile::PdfUa1)` in Rust) to check the rules for accessible PDFs. PDF/UA-1 needs every font embedded, so load a bold font file for the heading. See [Accessible PDFs](../../pages/guides/accessible-pdfs.md).

Next: [multilingual text](../../pages/recipes/multilingual.md), with five scripts, each in its own embedded font.
