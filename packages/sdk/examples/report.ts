// A 100-page report with two embedded fonts. Each page has a heading, a paragraph broken into a
// box, a line of Chinese, a table of products with rules between the rows, and a page number.
import { Builder, type BuilderOptions, Font, type Rect, type Surface } from "@laminapdf/sdk";

/** The number of pages. */
export const REPORT_PAGES = 100;
const PRODUCTS = 20;
const PRICE = 12.5;
const MARGIN = 56;
const RIGHT_EDGE = 539;
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
