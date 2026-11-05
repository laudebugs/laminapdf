// invoice.pdf: two US Letter pages; the heading and a table on page 1 (each row's cells on one
// baseline); the total and the thank-you line on page 2.
import { check, expect, near, runs, squash } from "./lib.mjs";

const ROWS = [
  ["Paper stock", "10", "45.00"],
  ["Toner cartridge", "2", "160.00"],
  ["Binding service", "1", "80.50"],
  ["Delivery", "1", "25.00"],
];

/**
 * The run that holds `text` and lies on the baseline of `row` (when given).
 *
 * @param {ReturnType<typeof runs>} all
 * @param {string} text
 * @param {{ y: number, height: number }} [row]
 */
function find(all, text, row) {
  return all.find((r) => r.text.includes(text) && (!row || near(r.y + r.height, row.y + row.height, 2)));
}

await check(({ open, pdf }) => {
  pdf("invoice.pdf");
  const doc = open("invoice.pdf");
  expect(doc.pageCount === 2, `expected 2 pages, found ${doc.pageCount}`);
  for (const p of doc.pages) {
    expect(near(p.width, 612) && near(p.height, 792), `page ${p.index + 1} is ${p.width} × ${p.height}, not Letter`);
  }
  const page1 = runs(doc, 0);
  expect(find(page1, "Invoice INV-2026-042"), "page 1 lacks the heading 'Invoice INV-2026-042'");
  for (const header of ["Item", "Qty", "Price"]) expect(find(page1, header), `page 1 lacks the column '${header}'`);
  for (const [item, qty, price] of ROWS) {
    const row = find(page1, item);
    expect(row, `page 1 lacks the row '${item}'`);
    expect(find(page1, qty, row) && find(page1, price, row), `the row '${item}' lacks ${qty} or ${price} on its line`);
  }
  const page2 = squash(doc.page(1).text().text());
  expect(page2.includes("Total: 310.50 EUR"), "page 2 lacks 'Total: 310.50 EUR'");
  expect(page2.includes("Thank you for your business"), "page 2 lacks 'Thank you for your business'");
  return "two Letter pages, heading, four rows on their lines, total on page 2";
});
