// A 100-page report with two embedded fonts. Each page has a heading, a paragraph broken into a
// box, a line of Chinese, a table of products with rules between the rows, and a page number.
import { Builder, type BuilderOptions, Font, type Surface } from "@laminapdf/sdk";

/** The number of pages. */
export const REPORT_PAGES = 100;
const PRODUCTS = 20;
const MARGIN = 56;
const RIGHT_EDGE = 539;
const FIRST_ROW = 200;
const ROW_HEIGHT = 24;
const PRICE = 12.5;
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
    s.text(`Region ${region}: quarterly report`, { font, size: 20, at: [MARGIN, 72] });
    s.textBox(PARAGRAPH, { font, size: 11, box: [MARGIN, 88, RIGHT_EDGE - MARGIN, 60] });
    s.text("你好世界，这是中文字体子集测试。", { font: chinese, size: 11, at: [MARGIN, 166] });
    products(s, font, region);
    s.text(`Page ${region} of ${REPORT_PAGES}`, { font, size: 9, at: [MARGIN, 800] });
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
    s.text(`Product ${product + 1}`, { font, size: 10, at: [MARGIN, y] });
    s.textBox(String(units), { font, size: 10, box: [300, y - 11, 60, 14], align: "end" });
    s.textBox(`€ ${(units * PRICE).toFixed(2)}`, { font, size: 10, box: [400, y - 11, 139, 14], align: "end" });
    s.strokeColor([0.8])
      .path((p) => p.moveTo(MARGIN, y + 7).lineTo(RIGHT_EDGE, y + 7))
      .stroke({ width: 0.5 });
  }
}
