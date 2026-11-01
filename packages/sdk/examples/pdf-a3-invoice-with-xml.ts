// A PDF/A-3b invoice that carries the same invoice as XML, attached for software to read.
import { Builder, Font, PAGE_SIZES, type Surface } from "@laminapdf/sdk";

const MARGIN = 56;
/** The page's right edge, less the margin: where the rule under the items ends. */
const RIGHT = PAGE_SIZES.A4[0] - MARGIN;
const AMOUNTS = 460;
const ROW = 22;
const TITLE_BASELINE = 80;
const FIRST_ROW = 130;
/**
 * How far above the next row's baseline the rule sits, and how far below it the total and
 * the note are.
 */
const RULE_RISE = 7;
const TOTAL_DROP = 18;
const NOTE_DROP = 68;
/** How far left of the amounts column the total starts, to fit its "Total" label. */
const TOTAL_INDENT = 40;
const TITLE = 24;
const TOTAL = 14;
const BODY = 11;
const ITEMS = [
  ["Sprockets", 12, 1.25],
  ["Gadget", 1, 79.9],
  ["Delivery", 1, 4.5],
] as const;

/** The invoice's total. */
function total(): number {
  return ITEMS.reduce((sum, [, qty, price]) => sum + qty * price, 0);
}

/** The invoice as XML, in a made-up schema (a real e-invoice follows EN 16931, as Factur-X does). */
function invoiceXml(): string {
  const lines = ITEMS.map(
    ([item, qty, price]) => `  <line item="${item}" quantity="${qty}" price="${price.toFixed(2)}"/>\n`,
  );
  return `<?xml version="1.0" encoding="UTF-8"?>\n<invoice number="42" currency="EUR">\n${lines.join("")}  <total>${total().toFixed(2)}</total>\n</invoice>\n`;
}

/**
 * The invoice's bytes. Needs `init({ text: true })`.
 *
 * @param notoSans - the bytes of the Noto Sans font file.
 */
export function pdfA3InvoiceWithXml(notoSans: Uint8Array): Uint8Array {
  using b = new Builder({ profiles: ["PDF/A-3b"] });
  b.metadata({ title: "Invoice 42", author: "Acme Widgets Ltd" });
  const font = Font.fromBytes(notoSans);
  b.font(font);
  const page = b.page("A4");
  draw(page.surface(), font);
  page.finish();
  b.attach("invoice-42.xml", new TextEncoder().encode(invoiceXml()), "text/xml", {
    relationship: "alternative",
    description: "Invoice 42 as XML",
  });
  using doc = b.finish();
  return doc.save();
}

function draw(s: Surface, font: Font): void {
  s.text("Invoice 42", { font, size: TITLE, at: [MARGIN, TITLE_BASELINE] });
  let y = FIRST_ROW;
  for (const [item, qty, price] of ITEMS) {
    s.text(`${qty} × ${item}`, { font, size: BODY, at: [MARGIN, y] });
    s.text(`€ ${(qty * price).toFixed(2)}`, { font, size: BODY, at: [AMOUNTS, y] });
    y += ROW;
  }
  const rule = y - RULE_RISE;
  s.strokeColor([0.5])
    .path((p) => p.moveTo(MARGIN, rule).lineTo(RIGHT, rule))
    .stroke();
  s.text(`Total € ${total().toFixed(2)}`, { font, size: TOTAL, at: [AMOUNTS - TOTAL_INDENT, y + TOTAL_DROP] });
  s.text("The invoice is attached as invoice-42.xml.", {
    font,
    size: BODY,
    at: [MARGIN, y + NOTE_DROP],
    link: { target: { type: "uri", uri: "https://example.com/invoices/42" }, contents: "Invoice 42 online" },
  });
}
