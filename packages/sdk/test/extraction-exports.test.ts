// A document's images, links and outline as files from the SDK: `extractImages` gives the same
// images as Rust (fixtures/images/extract.pdf), JPEGs byte for byte, and the links and outline
// JSON and CSV are the Rust goldens' (fixtures/navigation/).
import { Document, init, isLaminaPdfError, loadExtract } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read, readText, thrown } from "./helpers.js";

beforeAll(async () => {
  await init();
  await loadExtract();
});

/** The goldens write the generator's version as `x.y.z`, so a release changes none. */
const unversioned = (json: string) => json.replace(/"version": "[^"]+"/, '"version": "x.y.z"');

describe("extractImages", () => {
  it("B.03 (pdf, read): gives each image once, JPEGs and JPEG 2000 byte for byte, the rest as PNG", () => {
    using doc = Document.open(read("fixtures/images/extract.pdf"));
    const { images, skipped } = doc.extractImages();
    // Every image of the fixture but JBIG2-free: nothing is skipped.
    expect(skipped).toEqual([]);
    const jpeg = images.find((i) => i.object === 10);
    expect(jpeg?.format).toBe("jpeg");
    expect(jpeg?.extension).toBe("jpg");
    expect(jpeg?.data).toEqual(read("fixtures/images/photo-rgb.jpg"));
    expect(images.find((i) => i.object === 14)?.format).toBe("jpeg2000");
    // The Flate image page 2 draws again is given once, on page 1.
    expect(images.filter((i) => i.object === 15).map((i) => i.page)).toEqual([0]);
    const png = images.find((i) => i.object === 15)?.data;
    expect(png?.slice(0, 8)).toEqual(new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]));
    // A JPEG's soft mask is its own image; the inline image has no object.
    expect(images.find((i) => i.object === 13)?.kind).toBe("softMask");
    expect(images.some((i) => i.object === undefined && i.filters[0] === "ASCIIHexDecode")).toBe(true);
  });

  it("reads only the pages asked for, and refuses one past the last", () => {
    using doc = Document.open(read("fixtures/images/extract.pdf"));
    expect(doc.extractImages({ pages: [1] }).images.map((i) => [i.page, i.object])).toEqual([[1, 15]]);
    const error = thrown(() => doc.extractImages({ pages: [9] }));
    expect(isLaminaPdfError(error) && error.code).toBe("page-index-out-of-range");
  });
});

describe("the links and outline as files", () => {
  it("H.08 (pdf, read): gives the links as the Rust golden's JSON and CSV", () => {
    using doc = Document.open(read("fixtures/links/links.pdf"));
    expect(unversioned(doc.linksJson())).toBe(readText("fixtures/navigation/links.json"));
    expect(doc.linksCsv()).toBe(readText("fixtures/navigation/links.csv"));
  });

  it("H.08 (pdf, read): gives the outline as the Rust golden's JSON and CSV", () => {
    using doc = Document.open(read("fixtures/golden/links-and-outlines.pdf"));
    expect(unversioned(doc.outlineJson())).toBe(readText("fixtures/navigation/outline.json"));
    expect(doc.outlineCsv()).toBe(readText("fixtures/navigation/outline.csv"));
  });
});
