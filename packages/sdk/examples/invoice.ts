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
const [PAGE_WIDTH] = PAGE_SIZES.A4;
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
  const page = b.page("A4");
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
      const cells = [String(qty), `€ ${(qty * price).toFixed(2)}`];
      s.tag("TR", (s) => {
        s.text(item, { font, size: BODY, at: [MARGIN, y], tag: "TD" });
        COLUMNS.forEach((column, i) => {
          s.textBox(cells[i] ?? "", { font, size: BODY, box: cell(column, y), align: "end", tag: "TD" });
        });
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
