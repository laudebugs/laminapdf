// rotated.pdf: report.pdf's bytes unchanged at its start, page 3 rotated by 90° clockwise, the
// other pages as they were.
import { check, expect, startsWith, taskInput } from "./lib.mjs";

await check(({ sdk, read, open, pdf }) => {
  pdf("rotated.pdf");
  const original = taskInput("rotate-page-3", "report.pdf");
  const bytes = read("rotated.pdf");
  expect(startsWith(bytes, original), "rotated.pdf doesn't start with report.pdf's bytes: not an incremental update");
  expect(bytes.length > original.length, "rotated.pdf has nothing appended to report.pdf");
  const before = sdk.Document.open(original);
  const after = open("rotated.pdf");
  expect(after.pageCount === before.pageCount, `expected ${before.pageCount} pages, found ${after.pageCount}`);
  for (const [i, page] of before.pages.entries()) {
    const want = i === 2 ? (page.rotation + 90) % 360 : page.rotation;
    const got = after.pageInfo(i).rotation;
    expect(got === want, `page ${i + 1} is rotated ${got}°, expected ${want}°`);
  }
  return `page 3 rotated to ${after.pageInfo(2).rotation}°, ${bytes.length - original.length} bytes appended`;
});
