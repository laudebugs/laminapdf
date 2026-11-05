// report.pdf: one A4 page with the heading, and the Info title, author, subject and keywords.
import { check, expect, squash } from "./lib.mjs";

const EXPECTED = {
  title: "Quarterly Report",
  author: "Ada Lovelace",
  subject: "Q3 2026",
  keywords: "finance, quarterly",
};

await check(({ open, pdf }) => {
  pdf("report.pdf");
  const doc = open("report.pdf");
  expect(doc.pageCount === 1, `expected 1 page, found ${doc.pageCount}`);
  const info = doc.info;
  for (const [key, value] of Object.entries(EXPECTED)) {
    expect(info[key] === value, `${key} is ${JSON.stringify(info[key])}, not ${JSON.stringify(value)}`);
  }
  expect(squash(doc.page(0).text().text()).includes("Quarterly report"), "the heading 'Quarterly report' is missing");
  return "title, author, subject and keywords set";
});
