// photo.pdf: one A4 page, the JPEG embedded as it is (/DCTDecode, the file's bytes inside), and
// the caption below it.
import { Buffer } from "node:buffer";
import { check, expect, near, pdfObjects, runs, taskInput } from "./lib.mjs";

/**
 * A JPEG's pixel size, from its first start-of-frame marker.
 *
 * @param {Uint8Array} jpeg
 * @returns {{ width: number, height: number }}
 */
function jpegSize(jpeg) {
  const view = new DataView(jpeg.buffer, jpeg.byteOffset, jpeg.byteLength);
  for (let at = 2; at + 9 < jpeg.length; at += 2 + view.getUint16(at + 2)) {
    const marker = view.getUint16(at);
    if (marker >= 0xffc0 && marker <= 0xffc3) return { height: view.getUint16(at + 5), width: view.getUint16(at + 7) };
  }
  throw new Error("no start-of-frame marker in photo.jpg");
}

await check(({ read, open, pdf }) => {
  pdf("photo.pdf");
  const doc = open("photo.pdf");
  expect(doc.pageCount === 1, `expected 1 page, found ${doc.pageCount}`);
  const { width, height } = doc.pageInfo(0);
  expect(near(width, 595.28) && near(height, 841.89), `expected A4, found ${width} × ${height}`);
  const bytes = read("photo.pdf");
  const objects = pdfObjects(bytes);
  expect(/\/Subtype\s*\/Image\b/.test(objects), "no image XObject");
  expect(/\/DCTDecode\b/.test(objects), "the image isn't embedded as JPEG (/DCTDecode)");
  const jpeg = taskInput("image-with-caption", "photo.jpg");
  expect(Buffer.from(bytes).includes(Buffer.from(jpeg)), "the JPEG's bytes aren't in the file as they are");
  const photo = jpegSize(jpeg);
  const bottom = 72 + (200 * photo.height) / photo.width;
  const caption = runs(doc, 0).find((r) => r.text.includes("Figure 1: Test photo"));
  expect(caption, "the caption 'Figure 1: Test photo' is missing");
  expect(
    caption.y >= bottom - 1,
    `the caption's top is at y = ${caption.y.toFixed(1)}, above the photo's bottom (${bottom.toFixed(1)})`,
  );
  return "JPEG passed through, caption below the photo";
});
