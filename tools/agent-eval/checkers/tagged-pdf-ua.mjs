// notice.pdf: tagged (structure tree, /MarkInfo), claims PDF/UA-1 in its XMP, en-GB, the title,
// and H1 and P elements. Found by scanning the file's objects (streams inflated).
import { check, expect, pdfObjects, squash } from "./lib.mjs";

await check(({ read, open, pdf }) => {
  pdf("notice.pdf");
  const doc = open("notice.pdf");
  expect(doc.pageCount === 1, `expected 1 page, found ${doc.pageCount}`);
  expect(doc.info.title === "Accessible Notice", `the title is ${JSON.stringify(doc.info.title)}`);
  const text = squash(doc.page(0).text().text());
  for (const s of ["Accessible notice", "This document is tagged for screen readers."])
    expect(text.includes(s), `lacks '${s}'`);
  const objects = pdfObjects(read("notice.pdf"));
  expect(/\/StructTreeRoot\s/.test(objects), "no structure tree: the document isn't tagged");
  expect(/\/Marked\s+true/.test(objects), "no /MarkInfo << /Marked true >>");
  expect(/pdfuaid:part(>|=")1/.test(objects), "the XMP metadata doesn't claim PDF/UA-1 (pdfuaid:part 1)");
  expect(/\/Lang\s*\(en-GB\)/.test(objects), "the document language isn't en-GB");
  expect(/\/S\s*\/H1\b/.test(objects), "no H1 structure element");
  expect(/\/S\s*\/P\b/.test(objects), "no P structure element");
  expect(/\/FontFile[23]?\s/.test(objects), "no embedded font program");
  return "tagged, PDF/UA-1, en-GB, H1 and P";
});
