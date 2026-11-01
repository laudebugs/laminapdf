// A hybrid invoice (PDF/A-3b): the invoice people read, with the same invoice as XML attached for
// software to read. `attach` writes the file and associates it with the document, which PDF/A-3
// requires of every attached file. The TS twin of crates/lamina/examples/pdf_a3_invoice_with_xml.rs.
import { Builder, Font } from "@laminapdf/sdk";

const ITEMS: readonly (readonly [item: string, qty: number, price: number])[] = [
  ["Sprockets", 12, 1.25],
  ["Gadget", 1, 79.9],
  ["Delivery", 1, 4.5],
];

/** The invoice as XML, in a made-up schema (a real e-invoice follows EN 16931, as Factur-X does). */
function invoiceXml(total: number): string {
  const lines = ITEMS.map(
    ([item, qty, price]) => `  <line item="${item}" quantity="${qty}" price="${price.toFixed(2)}"/>\n`,
  ).join("");
  return `<?xml version="1.0" encoding="UTF-8"?>\n<invoice number="42" currency="EUR">\n${lines}  <total>${total.toFixed(2)}</total>\n</invoice>\n`;
}

/**
 * The document's bytes. Needs `init({ text: true })`.
 *
 * @param notoSans - `fixtures/fonts/NotoSans-Regular.ttf`.
 */
export function pdfA3InvoiceWithXml(notoSans: Uint8Array): Uint8Array {
  const b = new Builder({
    id: new TextEncoder().encode("lamina-ex-pdfa3b"),
    producer: "lamina examples",
    creationDate: "D:20261031120000Z",
    profiles: ["PDF/A-3b"],
  });
  b.metadata({ title: "Invoice 42", author: "Acme Widgets Ltd" });
  const font = Font.fromBytes(notoSans);
  b.font(font);
  const page = b.page("A4");
  const s = page.surface();
  s.text("Invoice 42", { font, size: 24, at: [56, 80] });
  let total = 0;
  ITEMS.forEach(([item, qty, price], row) => {
    const y = 130 + 22 * row;
    const amount = qty * price;
    total += amount;
    s.text(`${qty} × ${item}`, { font, size: 11, at: [56, y] });
    s.text(`€ ${amount.toFixed(2)}`, { font, size: 11, at: [460, y] });
  });
  s.strokeColor([0.5])
    .path((p) => p.moveTo(56, 205).lineTo(539, 205))
    .stroke();
  s.text(`Total € ${total.toFixed(2)}`, { font, size: 14, at: [420, 230] });
  s.text("The invoice is attached as invoice-42.xml.", {
    font,
    size: 9,
    at: [56, 280],
    link: { target: { type: "uri", uri: "https://example.com/invoices/42" }, contents: "Invoice 42 online" },
  });
  page.finish();
  b.attach("invoice-42.xml", new TextEncoder().encode(invoiceXml(total)), "text/xml", {
    relationship: "alternative",
    description: "Invoice 42 as XML",
  });
  return b.finish().save();
}
