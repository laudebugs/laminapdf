// An accessible report (PDF/UA-1): an embedded font only, a title and a language, headings in
// order, a paragraph, a list, a table with header cells, a chart as a figure with alternate text,
// a described link, and the running header and the rule as artifacts. The `PDF/UA-1` profile
// checks all of it when the build finishes. The TS twin of crates/lamina/examples/pdf_ua_report.rs.
import { Builder, Font } from "@laminapdf/sdk";

/** Sales per quarter, drawn as bars and listed in the table. */
const QUARTERS: readonly (readonly [quarter: string, sales: number])[] = [
  ["Q1", 120],
  ["Q2", 135],
  ["Q3", 160],
  ["Q4", 150],
];

/**
 * The document's bytes. Needs `init({ text: true })`.
 *
 * @param notoSans - `fixtures/fonts/NotoSans-Regular.ttf`.
 */
export function pdfUaReport(notoSans: Uint8Array): Uint8Array {
  const b = new Builder({
    id: new TextEncoder().encode("lamina-ex-pdfua1"),
    producer: "lamina examples",
    creationDate: "D:20261031120000Z",
    profiles: ["PDF/UA-1"],
  });
  b.metadata({ title: "Annual sales report 2026" }).lang("en-GB");
  const font = Font.fromBytes(notoSans);
  b.font(font);
  const page = b.page("A4");
  const s = page.surface();
  // The running header and the rule are decoration: artifacts, skipped by screen readers.
  s.artifact("pagination", (s) => {
    s.text("Acme Widgets Ltd · Annual report", { font, size: 8, at: [56, 40] });
    s.strokeColor([0.6])
      .path((p) => p.moveTo(56, 48).lineTo(539, 48))
      .stroke();
  });
  s.text("Annual sales report 2026", { font, size: 22, at: [56, 90], tag: "H1" });
  s.text("Sales grew in every quarter but the last.", { font, size: 11, at: [56, 115], tag: "P" });
  s.text("Highlights", { font, size: 15, at: [56, 150], tag: "H2" });
  s.tag("L", (s) => {
    ["Three new distributors", "A second warehouse"].forEach((item, i) => {
      const y = 172 + 18 * i;
      s.tag("LI", (s) => {
        s.text("•", { font, size: 11, at: [60, y], tag: "Lbl" });
        s.text(item, { font, size: 11, at: [74, y], tag: "LBody" });
      });
    });
  });
  s.text("Sales by quarter", { font, size: 15, at: [56, 230], tag: "H2" });
  s.tag("Table", (s) => {
    s.tag("TR", (s) => {
      for (const [x, label] of [
        [56, "Quarter"],
        [200, "Sales (k€)"],
      ] as const) {
        s.text(label, { font, size: 11, at: [x, 255], tag: { type: "TH", scope: "column" } });
      }
    });
    QUARTERS.forEach(([quarter, sales], row) => {
      const y = 275 + 18 * row;
      s.tag("TR", (s) => {
        s.text(quarter, { font, size: 11, at: [56, y], tag: "TD" });
        s.text(sales.toFixed(0), { font, size: 11, at: [200, y], tag: "TD" });
      });
    });
  });
  const alt = "Bar chart of sales by quarter: 120, 135, 160 and 150 thousand euros from Q1 to Q4.";
  s.tag({ type: "Figure", alt }, (s) => {
    s.fillColor([0.12, 0.47, 0.78]);
    QUARTERS.forEach(([, sales], i) => {
      const x = 320 + 50 * i;
      s.path((p) => p.rect([x, 360 - sales, 36, sales])).fill();
    });
  });
  s.text("Read the full accounts at example.com/accounts", {
    font,
    size: 11,
    at: [56, 400],
    fill: [0.12, 0.47, 0.78],
    link: { target: { type: "uri", uri: "https://example.com/accounts" }, contents: "The full 2026 accounts on example.com" },
    tag: "P",
  });
  page.finish();
  return b.finish().save();
}
