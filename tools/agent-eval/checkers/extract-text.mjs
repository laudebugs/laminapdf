// page1.txt: the text of sample.pdf's first page, compared with Lamina PDF's own extraction
// with whitespace collapsed (line breaks and spacing may differ).
import { CheckFailure, check, expect, squash, taskInput } from "./lib.mjs";

await check(({ sdk, read }) => {
  const got = squash(new TextDecoder().decode(read("page1.txt")));
  const doc = sdk.Document.open(taskInput("extract-text", "sample.pdf"));
  const want = squash(doc.page(0).text().text());
  expect(got.length > 0, "page1.txt is empty");
  if (got !== want) {
    const at = [...want].findIndex((c, i) => got[i] !== c);
    throw new CheckFailure(`page1.txt differs from the page's text at character ${at}: "${got.slice(at, at + 30)}"`);
  }
  return `${want.length} characters match`;
});
