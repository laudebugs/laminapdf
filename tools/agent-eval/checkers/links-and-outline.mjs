// guide.pdf: three A4 pages with their headings, an outline of three entries, a URI link and a
// link to page 3. The SDK can't read outlines or links back, so those are found by scanning the
// file's objects (streams inflated).
import { check, expect, hasPdfString, pdfObjects, squash, topDictionaries } from "./lib.mjs";

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
  const dictionaries = topDictionaries(objects);
  // Outline items have a /Title and a /Parent; the page headings' text is in content streams.
  const items = dictionaries.filter((d) => /\/Title\b/.test(d) && /\/Parent\b/.test(d));
  for (const title of HEADINGS) {
    expect(
      items.some((d) => hasPdfString(d, title)),
      `no outline entry titled '${title}'`,
    );
  }
  const links = dictionaries.filter((d) => /\/Subtype\s*\/Link\b/.test(d));
  expect(links.length >= 2, `found ${links.length} link annotations, expected 2`);
  expect(
    links.some((d) => /\/URI\s*\(https:\/\/laminapdf\.com\/?\)/.test(d)),
    "no link annotation to https://laminapdf.com",
  );
  expect(
    links.some((d) => /\/Dest\s*[[(/]|\/S\s*\/GoTo\b/.test(d)),
    "no link annotation to a page (/Dest or a GoTo action)",
  );
  return "three pages, an outline, a URI link and a page link";
});
