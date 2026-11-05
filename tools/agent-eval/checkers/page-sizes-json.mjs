// sizes.json: every page of shapes.pdf with its displayed size (crop box, then /Rotate) in points.
import { check, expect, near, taskInput } from "./lib.mjs";

/** @param {Uint8Array} bytes */
function parse(bytes) {
  try {
    return JSON.parse(new TextDecoder().decode(bytes));
  } catch {
    return undefined;
  }
}

await check(({ sdk, read }) => {
  const sizes = parse(read("sizes.json"));
  expect(Array.isArray(sizes), "sizes.json is not a JSON array");
  const doc = sdk.Document.open(taskInput("page-sizes-json", "shapes.pdf"));
  expect(sizes.length === doc.pageCount, `sizes.json lists ${sizes.length} pages, the file has ${doc.pageCount}`);
  for (const page of doc.pages) {
    const entry = sizes[page.index];
    expect(entry?.page === page.index + 1, `entry ${page.index} has page ${entry?.page}, not ${page.index + 1}`);
    const ok = near(entry.width, page.width, 0.01) && near(entry.height, page.height, 0.01);
    expect(
      ok,
      `page ${page.index + 1} is ${page.width.toFixed(2)} × ${page.height.toFixed(2)}, sizes.json says ${entry.width} × ${entry.height}`,
    );
  }
  return `${sizes.length} pages, displayed sizes match`;
});
