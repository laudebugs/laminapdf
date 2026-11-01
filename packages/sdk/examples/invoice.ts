// An invoice: an embedded, subset font and a standard 14 one, a coloured band, right-aligned
// text boxes, hairlines and a link, tagged (headings, a table, a link element; the bar and the
// rules as artifacts). The TS twin of crates/laminapdf/examples/invoice.rs.
import { Builder, Font, type Rect } from "@laminapdf/sdk";

const ITEMS: readonly (readonly [item: string, qty: number, price: number])[] = [
  ["Widgets, blue (W-100)", 3, 12.0],
  ["Gadget with ﬁne-tuned dials", 1, 79.9],
  ["Café-grade sprockets", 12, 1.25],
  ["Delivery", 1, 4.5],
];

/**
 * The document's bytes. Needs `init({ text: true })`.
 *
 * @param notoSans - `fixtures/fonts/NotoSans-Regular.ttf` (the caller reads files; the package doesn't).
 */
export function invoice(notoSans: Uint8Array): Uint8Array {
  const b = new Builder({
    id: new TextEncoder().encode("lamina-ex-invoic"),
    producer: "lamina examples",
    creationDate: "D:20261030120000Z",
    tagged: true,
  });
  const font = Font.fromBytes(notoSans);
  const bold = Font.standard("Helvetica-Bold");
  // Registered in the Rust example's order, so the objects are numbered the same.
  b.font(font);
  b.font(bold);
  const page = b.page("A4");
  const s = page.surface();
  // The accent bar and the hairlines are decoration (artifacts); the rest is the structure a
  // screen reader follows: a heading, a paragraph, a table, the total, a link.
  s.artifact("layout", (s) =>
    s
      .fillColor("#1e78c8")
      .path((p) => p.rect([0, 0, 595.28, 14]))
      .fill()
      .fillColor([0]),
  );
  s.text("Invoice 42", { font: bold, size: 24, at: [56, 80], tag: "H1" });
  s.text("Acme Widgets Ltd · 1 Example Street · London", { font, size: 10, at: [56, 100], tag: "P" });
  let total = 0;
  s.tag("Table", (s) => {
    ITEMS.forEach(([item, qty, price], row) => {
      const y = 160 + 24 * row;
      const amount = qty * price;
      total += amount;
      s.tag("TR", (s) => {
        s.text(item, { font, size: 11, at: [56, y], tag: "TD" });
        const cell = (box: Rect, text: string) => s.textBox(text, { font, size: 11, box, align: "end", tag: "TD" });
        cell([330, y - 12, 40, 16], String(qty));
        cell([439, y - 12, 100, 16], `€ ${amount.toFixed(2)}`);
        s.artifact("layout", (s) =>
          s
            .strokeColor([0.8])
            .path((p) => p.moveTo(56, y + 8).lineTo(539, y + 8))
            .stroke({ width: 0.5 }),
        );
      });
    });
  });
  s.textBox(`Total € ${total.toFixed(2)}`, { font, size: 14, box: [339, 262, 200, 20], align: "end", tag: "P" });
  s.text("Pay online at example.com/pay/42", {
    font,
    size: 11,
    at: [56, 320],
    fill: [0.12, 0.47, 0.78],
    link: { target: { type: "uri", uri: "https://example.com/pay/42" }, contents: "Pay invoice 42 online" },
    tag: "P",
  });
  page.finish();
  b.metadata({ title: "Invoice 42", author: "Acme Widgets Ltd" }).lang("en-GB");
  return b.finish().save();
}
