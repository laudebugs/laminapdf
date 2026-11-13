// imagesToPdf: one page per image in order, the paper turned the image's way, and the options
// record's checks. The example suite checks its bytes against the Rust golden.
import { Image, imagesToPdf, init, isLaminaPdfError, PAGE_SIZES } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read, thrown } from "./helpers.js";

beforeAll(() => init({ extract: true }));

/** 192 × 96 pixels: wider than it is tall. */
const WIDE = "fixtures/images/photo-wide.jpg";
const PHOTO = "fixtures/images/photo-rgb.jpg";

describe("imagesToPdf", () => {
  it("makes one page per image, in order", () => {
    using doc = imagesToPdf([Image.jpeg(read(PHOTO)), Image.jpeg(read(WIDE))]);
    expect(doc.pageCount).toBe(2);
  });

  it("turns the named paper the image's way when fitting", () => {
    using doc = imagesToPdf([Image.jpeg(read(WIDE))], { fit: "A4", margin: 20 });
    const [width, height] = PAGE_SIZES.A4;
    const [page] = doc.pages;
    expect([page?.width, page?.height]).toEqual([height, width]);
  });

  it("takes a page size in points", () => {
    using doc = imagesToPdf([Image.jpeg(read(PHOTO))], { fit: [300, 400] });
    const [page] = doc.pages;
    expect([page?.width, page?.height]).toEqual([300, 400]);
  });

  it("leaves the images usable for another document", () => {
    const image = Image.jpeg(read(PHOTO));
    using first = imagesToPdf([image]);
    using second = imagesToPdf([image, image]);
    expect([first.pageCount, second.pageCount]).toEqual([1, 2]);
  });

  it("refuses no images", () => {
    expect(
      isLaminaPdfError(
        thrown(() => imagesToPdf([])),
        "invalid-geometry",
      ),
    ).toBe(true);
  });

  it("refuses a negative margin", () => {
    const run = () => imagesToPdf([Image.jpeg(read(PHOTO))], { margin: -1 });
    expect(isLaminaPdfError(thrown(run), "invalid-geometry")).toBe(true);
  });

  it("refuses a paper size it doesn't know", () => {
    const run = () => imagesToPdf([Image.jpeg(read(PHOTO))], { fit: "A9" as "A4" });
    expect(isLaminaPdfError(thrown(run), "invalid-argument")).toBe(true);
  });
});
