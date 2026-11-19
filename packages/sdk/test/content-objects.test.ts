// Images, shapes and pages of other PDFs placed on pages as content: doc.content.add, update,
// remove and added, saved and reopened. A test's title starts with the capability it proves (its
// tag: capabilities B.01, B.02, B.04, B.05, B.09 and B.12, column pdf), which
// `cargo xtask capabilities` counts. Every call here works in the base module.
import {
  type AddedObject,
  type AnnotColor,
  Document,
  init,
  type NewContentObject,
  type PathSpec,
  type Stroke,
} from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read } from "./helpers.js";

beforeAll(async () => {
  await init();
});

const AT = "D:20261118120000Z";
const IDENTITY = [1, 0, 0, 1, 0, 0] as const;

function savedAndReopened(doc: Document): Document {
  return Document.open(doc.save({ mode: "incremental" }));
}

function summary(objects: readonly AddedObject[]) {
  return objects.map((o) => [o.id, o.object && Object.keys(o.object)[0], o.layer, o.opacity, o.bounds]);
}

describe("content placed on pages", () => {
  it("[cap B.09 pdf round-trip] places a PNG logo and a page of another PDF that reopen where they were put", () => {
    using doc = Document.open(read("fixtures/corpus/hello.pdf"));
    doc.content.add(0, {
      id: "logo",
      object: { image: { blob: doc.addBlob(read("fixtures/content/add/logo-alpha.png")) } },
      matrix: [128, 0, 0, 64, 72, 72],
      modified: AT,
    });
    doc.content.add(0, {
      id: "wordmark",
      object: { form: { blob: doc.addBlob(read("fixtures/content/add/logo.pdf")), page: 0 } },
      matrix: [120, 0, 0, 60, 300, 72],
      modified: AT,
    });
    using again = savedAndReopened(doc);
    expect(summary(again.content.added(0))).toEqual([
      ["logo", "image", "front", 1, [72, 72, 200, 136]],
      ["wordmark", "form", "front", 1, [300, 72, 420, 132]],
    ]);
  });

  it("[cap B.05 pdf round-trip] draws a line, a rectangle, an ellipse and a polygon that reopen with their paths", () => {
    using doc = Document.open(read("fixtures/corpus/hello.pdf"));
    const stroke: Stroke = { color: [0.2, 0.2, 0.6], width: 2 };
    const shapes: PathSpec[] = [
      { line: { from: { x: 72, y: 300 }, to: { x: 300, y: 320 } } },
      { rect: { rect: [72, 340, 172, 400], radius: 8 } },
      { ellipse: { rect: [200, 340, 300, 400] } },
      {
        polygon: {
          points: [
            { x: 320, y: 400 },
            { x: 380, y: 340 },
            { x: 440, y: 400 },
          ],
        },
      },
    ];
    shapes.forEach((path, i) => {
      const fill: AnnotColor | undefined = "line" in path ? undefined : [1, 0.9, 0.6];
      doc.content.add(0, {
        id: `shape-${i}`,
        object: { shape: { path, stroke, ...(fill ? { fill } : {}) } },
        matrix: IDENTITY,
        modified: AT,
      });
    });
    using again = savedAndReopened(doc);
    const paths = again.content
      .added(0)
      .map((o) => (o.object && "shape" in o.object ? Object.keys(o.object.shape.path)[0] : undefined));
    expect(paths).toEqual(["line", "rect", "ellipse", "polygon"]);
  });

  it("[cap B.04 pdf round-trip] fades an object, and the opacity reopens", () => {
    using doc = Document.open(read("fixtures/corpus/hello.pdf"));
    doc.content.add(0, {
      id: "band",
      object: { shape: { path: { rect: { rect: [36, 36, 300, 90] } }, fill: [1, 0.8, 0.2] } },
      matrix: IDENTITY,
      opacity: 0.4,
      layer: "behind",
      modified: AT,
    });
    using again = savedAndReopened(doc);
    expect(summary(again.content.added(0))).toEqual([["band", "shape", "behind", 0.4, [36, 36, 300, 90]]]);
  });

  it("[cap B.02 pdf write] moves, resizes and crops an image added before a save", () => {
    using doc = Document.open(read("fixtures/corpus/hello.pdf"));
    doc.content.add(0, {
      id: "photo",
      object: { type: "image", blob: doc.addBlob(read("fixtures/images/photo-rgb.jpg")) },
      matrix: [200, 0, 0, 100, 72, 72],
      modified: AT,
    });
    using again = savedAndReopened(doc);
    again.content.update(0, "photo", { matrix: [100, 0, 0, 50, 300, 300], crop: [0, 0, 0.5, 1], modified: AT });
    expect(again.content.added(0)[0]?.bounds).toEqual([300, 300, 350, 350]);
  });

  it("[cap B.12 pdf write] turns an added object and brings another to the front", () => {
    using doc = Document.open(read("fixtures/corpus/hello.pdf"));
    for (const id of ["a", "b"]) {
      doc.content.add(0, {
        id,
        object: { shape: { path: { rect: { rect: [0, 0, 100, 50] } }, fill: [0.5, 0.5, 0.5] } },
        matrix: IDENTITY,
        modified: AT,
      });
    }
    doc.content.update(0, "a", { matrix: [0, 1, -1, 0, 200, 200], order: -1, modified: AT });
    const objects = doc.content.added(0);
    expect(objects.map((o) => o.id)).toEqual(["b", "a"]);
    expect(objects[1]?.bounds).toEqual([150, 200, 200, 300]);
  });

  it("[cap B.01 pdf write] replaces an added image's bytes and deletes it", () => {
    using doc = Document.open(read("fixtures/corpus/hello.pdf"));
    doc.content.add(0, {
      id: "photo",
      object: { type: "image", blob: doc.addBlob(read("fixtures/images/photo-rgb.jpg")) },
      matrix: [200, 0, 0, 100, 72, 72],
      modified: AT,
    });
    const gray = doc.addBlob(read("fixtures/images/photo-gray.jpg"));
    doc.content.update(0, "photo", { blob: gray, modified: AT });
    const [photo] = doc.content.added(0);
    expect(photo?.object).toEqual({ image: { blob: gray } });
    const removed = doc.content.remove(0, "photo");
    expect([doc.content.added(0), removed.stillInEarlierRevisions]).toEqual([[], true]);
  });

  it("refuses an ID the page already has", () => {
    using doc = Document.open(read("fixtures/corpus/hello.pdf"));
    const band: NewContentObject = {
      id: "band",
      object: { shape: { path: { rect: { rect: [0, 0, 10, 10] } }, fill: [0] } },
      matrix: IDENTITY,
      modified: AT,
    };
    doc.content.add(0, band);
    expect(() => doc.content.add(0, band)).toThrow(expect.objectContaining({ code: "content-id-in-use" }));
  });
});
