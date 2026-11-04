// final.pdf: draft.pdf's bytes unchanged at its start, the new title and author, the same pages
// and text.
import { check, expect, squash, startsWith, taskInput } from "./lib.mjs";

await check(({ sdk, read, open, pdf }) => {
  pdf("final.pdf");
  const original = taskInput("edit-metadata-incremental", "draft.pdf");
  const bytes = read("final.pdf");
  expect(startsWith(bytes, original), "final.pdf doesn't start with draft.pdf's bytes: not an incremental update");
  expect(bytes.length > original.length, "final.pdf has nothing appended to draft.pdf");
  const before = sdk.Document.open(original);
  const after = open("final.pdf");
  expect(after.info.title === "Final report", `the title is ${JSON.stringify(after.info.title)}`);
  expect(after.info.author === "Ada Lovelace", `the author is ${JSON.stringify(after.info.author)}`);
  expect(after.pageCount === before.pageCount, `expected ${before.pageCount} pages, found ${after.pageCount}`);
  const same = squash(after.page(0).text().text()) === squash(before.page(0).text().text());
  expect(same, "page 1's text changed");
  return `title and author set, ${bytes.length - original.length} bytes appended`;
});
