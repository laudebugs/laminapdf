// An invoice whose table runs over three pages: its header row repeats at the top of each
// page, no row is split between pages, and the total stays on the page of the last row.
//
// The columns fit their widest cell, and numbers line up on the right. The document is plain
// objects in the `laminapdf.document/1` format, which names its two fonts; their bytes come beside
// it. The output is tagged (one `Table` element across the pages) and checked against PDF/UA-1.
import { type BlockSpec, Builder, type BuilderOptions, type CellSpec, type FlowDocument, Font } from "@laminapdf/sdk";

const TITLE = "Invoice 2026-114";
const PRODUCTS: readonly (readonly [string, number])[] = [
  ["Paper, A4, 80 g, box of 2,500 sheets", 2_450],
  ["Toner cartridge, black", 6_900],
  ["Desk lamp with a weighted base and an adjustable arm", 4_275],
  ["Stapler", 1_290],
  ["Archive boxes, set of ten, with lids", 3_150],
  ["Ballpoint pens, box of fifty", 1_875],
];
const LINES = 60;
const MARGIN = 56;
const PADDING = 6;
const RULE_WIDTH = 0.75;
const ROW_RULE_WIDTH = 0.25;
/** The frame and the rule under the header, the rules between rows, and the header row's shade. */
const RULE = "#666666";
const ROW_RULE = "#b3b3b3";
const HEADER_SHADE = "#ebebeb";

/**
 * The invoice's bytes. Needs `init({ text: true })`.
 *
 * @param regular - the body's font.
 * @param bold - a collection whose second face (index 1) is the bold one.
 * @param options - can fix the file ID, the producer and the creation date.
 */
export function flowingInvoice(
  regular: Uint8Array,
  bold: Uint8Array,
  options: BuilderOptions = {},
): Uint8Array<ArrayBuffer> {
  using b = new Builder({ ...options, profiles: ["PDF/UA-1"] });
  const document: FlowDocument = {
    schema: "laminapdf.document/1",
    title: TITLE,
    lang: "en",
    page: { size: "A4", margins: MARGIN, footer: { center: "{title}, page {page} of {pages}" }, style: "small" },
    styles: {
      body: { font: "regular", size: 10 },
      strong: { font: "bold", size: 10 },
      title: { font: "bold", size: 18 },
      small: { font: "regular", size: 8 },
    },
    blocks: [
      { type: "heading", level: 1, text: TITLE, style: "title" },
      { type: "paragraph", text: "Office supplies delivered in October, to be paid within 30 days." },
      table(),
    ],
  };
  b.flow(document, { fonts: { regular: Font.fromBytes(regular), bold: Font.fromCollection(bold, 1) } });
  using doc = b.finish();
  return doc.save();
}

/** Sixty lines of products, a header row and a footer row with the total. */
function table(): BlockSpec {
  const number = (text: string, style = "body"): CellSpec => ({ text, style, align: "end" });
  const header = ["Item", "Description", "Qty", "Amount"].map(
    (text, i): CellSpec => ({ text, style: "strong", ...(i >= 2 ? { align: "end" } : {}) }),
  );
  let total = 0;
  const rows = Array.from({ length: LINES }, (_, i) => {
    const [name, cents] = PRODUCTS[i % PRODUCTS.length] ?? ["", 0];
    const qty = (i % 5) + 1;
    total += qty * cents;
    return {
      cells: [
        { text: String(i + 1).padStart(3, "0") },
        { text: name },
        number(String(qty)),
        number(money(qty * cents)),
      ],
    };
  });
  return {
    type: "table",
    columns: ["auto", "auto", "auto", "auto"],
    padding: PADDING,
    frame: { width: RULE_WIDTH, color: RULE },
    headerRule: { width: RULE_WIDTH, color: RULE },
    rowRules: { width: ROW_RULE_WIDTH, color: ROW_RULE },
    headerBackground: HEADER_SHADE,
    header: [{ cells: header }],
    rows,
    footer: [{ cells: [{ text: "Total", style: "strong", span: 3 }, number(money(total), "strong")] }],
  };
}

function money(cents: number): string {
  return `${Math.floor(cents / 100)}.${String(cents % 100).padStart(2, "0")}`;
}
