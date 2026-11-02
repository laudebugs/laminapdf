// An invoice that screen readers can follow, with an embedded font and a table of line items.
// The heading, the address, the items and the total are tagged; the rules under the rows are
// decoration.
import { Builder, type BuilderOptions, Font, type Surface } from "@laminapdf/sdk";

const ITEMS: readonly (readonly [item: string, qty: number, price: number])[] = [
  ["Widgets, blue (W-100)", 3, 12.0],
  ["Gadget with ﬁne-tuned dials", 1, 79.9],
  ["Café-grade sprockets", 12, 1.25],
  ["Delivery", 1, 4.5],
];
const ADDRESS = "Acme Widgets Ltd · 1 Example Street · London";
const MARGIN = 56;
const RIGHT_EDGE = 539;
const BODY = 11;
const FIRST_ROW = 160;
const ROW_HEIGHT = 24;
/** The quantity's and the amount's columns (left edge, width), both right-aligned. */
const COLUMNS = [
  [330, 40],
  [439, 100],
] as const;

/**
 * The document's bytes, in `font` (a TrueType or OpenType file). Needs `init({ text: true })`.
 *
 * @param options - can fix the file ID, the producer and the creation date.
 */
export function invoice(fontBytes: Uint8Array, options: BuilderOptions = {}): Uint8Array<ArrayBuffer> {
  using b = new Builder({ ...options, tagged: true });
  const font = Font.fromBytes(fontBytes);
  const bold = Font.standard("Helvetica-Bold");
  // Registered before drawing, so the font objects come in this order.
  b.font(font);
  b.font(bold);
  const page = b.page("A4");
  const s = page.surface();
  s.text("Invoice 42", { font: bold, size: 24, at: [MARGIN, 80], tag: "H1" });
  s.text(ADDRESS, { font, size: BODY, at: [MARGIN, 100], tag: "P" });
  items(s, font);
  const total = ITEMS.reduce((sum, [, qty, price]) => sum + qty * price, 0);
  const box = [339, 262, 200, 20] as const;
  s.textBox(`Total € ${total.toFixed(2)}`, { font, size: 14, box, align: "end", tag: "P" });
  page.finish();
  b.metadata({ title: "Invoice 42", author: "Acme Widgets Ltd" }).lang("en-GB");
  using doc = b.finish();
  return doc.save();
}

/** The table of line items, each row with a light rule under it (decoration). */
function items(s: Surface, font: Font): void {
  s.tag("Table", (s) => {
    ITEMS.forEach(([item, qty, price], row) => {
      const y = FIRST_ROW + ROW_HEIGHT * row;
      const cells = [String(qty), `€ ${(qty * price).toFixed(2)}`];
      s.tag("TR", (s) => {
        s.text(item, { font, size: BODY, at: [MARGIN, y], tag: "TD" });
        COLUMNS.forEach(([left, width], i) => {
          const box = [left, y - 12, width, 16] as const;
          s.textBox(cells[i] ?? "", { font, size: BODY, box, align: "end", tag: "TD" });
        });
        s.artifact("layout", (s) =>
          s
            .strokeColor([0.8])
            .path((p) => p.moveTo(MARGIN, y + 8).lineTo(RIGHT_EDGE, y + 8))
            .stroke({ width: 0.5 }),
        );
      });
    });
  });
}
