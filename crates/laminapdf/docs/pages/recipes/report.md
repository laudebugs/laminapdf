---
title: "A 100-page report"
description: "A 100-page A4 report with two embedded fonts and a table on every page, the document Lamina PDF times in each JavaScript runtime."
version: "0.2.0-alpha.6"
url: "https://docs.laminapdf.com/recipes/report/"
---

# A 100-page report

A 100-page A4 report with two embedded fonts and a table on every page, the document Lamina PDF times in each JavaScript runtime.

You'll make a 100-page report with two embedded fonts: on every page, a heading, a paragraph, a line of Chinese and a table of 20 products. Start from it for any long document you generate from data, such as monthly statements or a catalogue.

The function draws one region's page at a time in a loop, then returns the PDF's bytes:

`packages/sdk/examples/report.ts`:

```ts
// A 100-page report with two embedded fonts. Each page has a heading, a paragraph broken into a
// box, a line of Chinese, a table of products with rules between the rows, and a page number.
import { Builder, type BuilderOptions, Font, PAGE_SIZES, type Rect, type Surface } from "@laminapdf/sdk";

/** The number of pages. */
const REPORT_PAGES = 100;
const PRODUCTS = 20;
const PRICE = 12.5;
const [PAGE_WIDTH] = PAGE_SIZES.A4;
const MARGIN = 56;
const RIGHT_EDGE = PAGE_WIDTH - MARGIN;
const HEADING_AT = [MARGIN, 72] as const;
const PARAGRAPH_BOX = [MARGIN, 88, RIGHT_EDGE - MARGIN, 60] as const;
const CHINESE_AT = [MARGIN, 166] as const;
const FOOTER_AT = [MARGIN, 800] as const;
const FIRST_ROW = 200;
const ROW_HEIGHT = 24;
/** The units' and the revenue's columns (left edge, width), both right-aligned. */
const UNITS_COLUMN = [300, 60] as const;
const REVENUE_COLUMN = [400, RIGHT_EDGE - 400] as const;
/** A cell's box starts this far above its row's baseline and is this tall; the rule is this far below. */
const CELL_RISE = 11;
const CELL_HEIGHT = 14;
const RULE_DROP = 7;
/** Font sizes: the heading, the paragraph and the Chinese line, the table, the page number. */
const SIZES = { heading: 20, body: 11, table: 10, footer: 9 } as const;
const PARAGRAPH =
  "Quarterly figures for every region follow. Each table lists the units shipped, the revenue " +
  "and the change against the previous quarter; totals are at the end of the report.";

/**
 * The document's bytes. Needs `init({ text: true })`.
 *
 * @param latinFont - a font for the text, such as Noto Sans.
 * @param chineseFont - a font with Chinese glyphs, such as Noto Sans SC.
 * @param options - can fix the file ID, the producer and the creation date.
 */
export function report(
  latinFont: Uint8Array,
  chineseFont: Uint8Array,
  options: BuilderOptions = {},
): Uint8Array<ArrayBuffer> {
  using b = new Builder(options);
  const font = Font.fromBytes(latinFont);
  const chinese = Font.fromBytes(chineseFont);
  b.font(font);
  b.font(chinese);
  for (let region = 1; region <= REPORT_PAGES; region++) {
    const page = b.page("A4");
    const s = page.surface();
    s.text(`Region ${region}: quarterly report`, { font, size: SIZES.heading, at: HEADING_AT });
    s.textBox(PARAGRAPH, { font, size: SIZES.body, box: PARAGRAPH_BOX });
    s.text("你好世界，这是中文字体子集测试。", { font: chinese, size: SIZES.body, at: CHINESE_AT });
    products(s, font, region);
    s.text(`Page ${region} of ${REPORT_PAGES}`, { font, size: SIZES.footer, at: FOOTER_AT });
    page.finish();
  }
  b.metadata({ title: "Quarterly report", author: "lamina examples" }).lang("en-GB");
  using doc = b.finish();
  return doc.save();
}

/** Made-up sales figures: the units of `product` that `region` shipped, from 0 to 499. */
function unitsShipped(region: number, product: number): number {
  return (region * 37 + product * 11) % 500;
}

/** A region's table: one row per product, with its units and revenue right-aligned. */
function products(s: Surface, font: Font, region: number): void {
  for (let product = 0; product < PRODUCTS; product++) {
    const y = FIRST_ROW + ROW_HEIGHT * product;
    const units = unitsShipped(region, product);
    const size = SIZES.table;
    s.text(`Product ${product + 1}`, { font, size, at: [MARGIN, y] });
    s.textBox(String(units), { font, size, box: cell(UNITS_COLUMN, y), align: "end" });
    s.textBox(`€ ${(units * PRICE).toFixed(2)}`, { font, size, box: cell(REVENUE_COLUMN, y), align: "end" });
    s.strokeColor([0.8])
      .path((p) => p.moveTo(MARGIN, y + RULE_DROP).lineTo(RIGHT_EDGE, y + RULE_DROP))
      .stroke({ width: 0.5 });
  }
}

/** The box of a cell in `column` (left edge, width) on the row at `baseline`. */
function cell([left, width]: readonly [number, number], baseline: number): Rect {
  return [left, baseline - CELL_RISE, width, CELL_HEIGHT];
}
```

The result is 100 A4 pages, one per region. It has no reference PDF to open. Instead, Lamina PDF's tests build and time the report in every JavaScript runtime and check its SHA-256 there, as does the Rust benchmark that builds the same bytes. [Benchmarks](../../pages/reference/benchmarks.md) has the timings.

## How it works

- It needs the text module, `init({ text: true })`, because it draws with embedded fonts.
- `Font.fromBytes` reads each font file, and `b.font` registers both before the first page. Each font is embedded once for the whole report, with only the glyphs its 100 pages draw.
- `s.text` draws one line, with `at[1]` as its baseline, and `s.textBox` breaks the paragraph into lines that fit `PARAGRAPH_BOX`.
- In the table, the units and the revenue are text boxes as wide as their columns, with `align: "end"` to right-align them. `cell` works out each box from its column and the row's baseline.
- `strokeColor([0.8])` sets a light gray, and each rule is a path from the left margin to the right edge, stroked 0.5 points wide.
- `metadata` and `lang("en-GB")` set the title, author and language once for the document, and `using` frees the builder and the document when the function returns.

## What to change

- **Your data:** replace `unitsShipped` with your own figures, and set `REPORT_PAGES` and `PRODUCTS` to your counts.
- **A longer table:** 20 rows end at 656 points, above the page number at 800. For more rows, lower `ROW_HEIGHT` or carry the rest over to the next page.
- **Another script:** load a font that covers it and draw its text in that font, as the Chinese line does. See [Text and fonts](../../pages/guides/fonts-and-text.md).
- **The file's details:** pass `producer` and `creationDate` in `options`, such as `creationDate: "D:20261104120000Z"`. The builder never reads the clock, so a report without a date has none.

Next: [open someone else's file](../../pages/recipes/open-untrusted.md), with a password and tighter limits.
