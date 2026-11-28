// A picture of a signature placed three ways, saved and reopened: a Stamp annotation, an Ink
// annotation and an image added to the page's content. None of them is a digital signature. A
// test's title starts with the capability it proves (its tag: capabilities F.01 and F.02, column
// pdf), which `cargo xtask capabilities` counts. Every call here works in the base module.
import { Document, init } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read } from "./helpers.js";

beforeAll(async () => {
  await init();
});

const AT = "D:20261127120000Z";
const BOX = { x0: 72, y0: 500, x1: 232, y1: 560 };

function savedAndReopened(doc: Document): Document {
  return Document.open(doc.save({ mode: "incremental" }));
}

describe("signature images", () => {
  it("[cap F.01 pdf round-trip] places a picture of a signature as a stamp and as page content that reopen where they were put", () => {
    using doc = Document.open(read("fixtures/corpus/hello.pdf"));
    const picture = doc.addBlob(read("fixtures/content/add/logo-alpha.png"));
    const page = doc.pageInfo(0).id;
    doc.apply([
      { op: "addAnnot", args: { page, spec: { type: "stamp", rect: BOX, stamp: { image: picture } }, common: {} } },
    ]);
    doc.content.add(0, {
      id: "signature-1",
      object: { image: { blob: picture } },
      matrix: [160, 0, 0, 60, 72, 100],
      alt: { text: "Signature image" },
      modified: AT,
    });
    using again = savedAndReopened(doc);
    const stamps = again.pageAnnotations(0).filter((a) => a.subtype === "Stamp");
    expect(stamps.map((a) => a.hasAppearance)).toEqual([true]);
    expect(again.content.added(0).map((o) => [o.id, o.bounds])).toEqual([["signature-1", [72, 100, 232, 160]]]);
  });

  it("[cap F.02 pdf round-trip] places a drawn signature as ink strokes that reopen as an ink annotation with an appearance", () => {
    using doc = Document.open(read("fixtures/corpus/hello.pdf"));
    const page = doc.pageInfo(0).id;
    doc.apply([
      {
        op: "addAnnot",
        args: {
          page,
          spec: {
            type: "ink",
            strokes: [
              {
                points: [
                  { x: 80, y: 520 },
                  { x: 120, y: 540 },
                  { x: 160, y: 510 },
                ],
              },
            ],
            width: 1.6,
          },
          common: { color: [0, 0, 0.5] },
        },
      },
    ]);
    using again = savedAndReopened(doc);
    const ink = again.pageAnnotations(0).filter((a) => a.subtype === "Ink");
    expect(ink.map((a) => a.hasAppearance)).toEqual([true]);
  });
});
