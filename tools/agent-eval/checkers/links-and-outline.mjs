// guide.pdf: three A4 pages with their headings, an outline of three entries, a URI link and a
// link to page 3. The SDK can't read outlines or links back, so those are found by scanning the
// file's objects (streams inflated).
import { check, expect, hasPdfString, pdfObjects, squash } from "./lib.mjs";

const HEADINGS = ["Introduction", "Details", "Summary"];

await check(({ read, open, pdf }) => {
  pdf("guide.pdf");
  const doc = open("guide.pdf");
  expect(doc.pageCount === 3, `expected 3 pages, found ${doc.pageCount}`);
  for (const [i, heading] of HEADINGS.entries()) {
    expect(squash(doc.page(i).text().text()).includes(heading), `page ${i + 1} lacks the heading '${heading}'`);
  }
  const page1 = squash(doc.page(0).text().text());
  for (const text of ["Visit laminapdf.com", "Jump to the summary"])
    expect(page1.includes(text), `page 1 lacks '${text}'`);
  const objects = pdfObjects(read("guide.pdf"));
  expect(/\/Outlines\s/.test(objects), "the catalog has no /Outlines");
  for (const title of HEADINGS) expect(hasPdfString(objects, title), `no outline entry titled '${title}'`);
  const links = objects.match(/\/Subtype\s*\/Link\b/g) ?? [];
  expect(links.length >= 2, `found ${links.length} link annotations, expected 2`);
  expect(/\/URI\s*\(https:\/\/laminapdf\.com\/?\)/.test(objects), "no link to https://laminapdf.com");
  expect(/\/Dest\s*[[(/]|\/S\s*\/GoTo\b/.test(objects), "no link to a page (/Dest or a GoTo action)");
  return "three pages, an outline, a URI link and a page link";
});
