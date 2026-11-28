import type { Op } from "@laminapdf/sdk";
import { describe, expect, it } from "vitest";
import {
  INK_WIDTH,
  placementChoices,
  placeSignatureOp,
  type ReadySignature,
  SIGNATURE_SIDE,
  type SignatureContext,
  signatureBox,
} from "./placement.ts";

/** A letter page shown upright: page space to user space flips y. */
const context: SignatureContext = {
  pageId: "p1",
  toUser: ([x, y]) => ({ x, y: 792 - y }),
  rotation: 0,
  author: "A. Okafor",
  created: new Date(Date.UTC(2026, 10, 27, 9, 30, 0)),
  contentId: "sig-1",
};

const drawn: ReadySignature = {
  blob: "b1",
  width: 300,
  height: 100,
  strokes: [
    [
      [0, 0],
      [300, 100],
    ],
  ],
  ink: [0, 0, 0.5],
};

const args = (op: Op) => (op as unknown as { args: Record<string, unknown> }).args;

describe("a signature's box", () => {
  it("centres on the click with its longer side at the default, its proportions kept", () => {
    expect(signatureBox({ width: 300, height: 100 }, { at: [300, 400] })).toEqual([
      300 - SIGNATURE_SIDE / 2,
      400 - SIGNATURE_SIDE / 6,
      300 + SIGNATURE_SIDE / 2,
      400 + SIGNATURE_SIDE / 6,
    ]);
    const [x0, , x1, y1] = signatureBox({ width: 100, height: 200 }, { at: [0, 0] });
    expect([x1 - x0, y1 * 2]).toEqual([SIGNATURE_SIDE / 2, SIGNATURE_SIDE]);
  });

  it("fits inside a field, centred, so a wide picture in a tall field leaves room above and below", () => {
    expect(signatureBox({ width: 300, height: 100 }, { field: [100, 100, 220, 200] })).toEqual([100, 130, 220, 170]);
  });
});

describe("which placements a file allows", () => {
  it("offers Ink only for a drawn signature", () => {
    expect(placementChoices(false, false)).toEqual(["stamp", "content"]);
    expect(placementChoices(true, false)).toEqual(["stamp", "ink", "content"]);
  });

  it("withholds the content placement from a certified file", () => {
    expect(placementChoices(true, true)).toEqual(["stamp", "ink"]);
  });
});

describe("writing a signature", () => {
  const box = [100, 100, 220, 140] as const;

  it("is a Stamp annotation of the picture in user space, with its author and date", () => {
    const op = placeSignatureOp(drawn, "stamp", box, context);
    expect(op).toMatchObject({ op: "addAnnot" });
    expect(args(op).spec).toEqual({
      type: "stamp",
      rect: { x0: 100, y0: 652, x1: 220, y1: 692 },
      stamp: { image: "b1" },
    });
    expect(args(op).common).toMatchObject({ author: "A. Okafor" });
  });

  it("turns the stamp upright on a rotated page", () => {
    const spec = args(placeSignatureOp(drawn, "stamp", box, { ...context, rotation: 90 })).spec;
    expect(spec).toMatchObject({ rotation: 90 });
  });

  it("is an Ink annotation whose strokes are scaled into the box, in the ink's colour", () => {
    const op = placeSignatureOp(drawn, "ink", box, context);
    expect(args(op).spec).toEqual({
      type: "ink",
      width: INK_WIDTH,
      strokes: [
        {
          points: [
            { x: 100, y: 692 },
            { x: 220, y: 652 },
          ],
        },
      ],
    });
    expect(args(op).common).toMatchObject({ color: [0, 0, 0.5] });
  });

  it("is an image added to the page's content, the unit square mapped to the box in page space", () => {
    const op = placeSignatureOp(drawn, "content", box, context);
    expect(op).toMatchObject({ op: "addContentObject" });
    expect(args(op)).toMatchObject({
      id: "sig-1",
      page: "p1",
      object: { image: { blob: "b1" } },
      matrix: [120, 0, 0, 40, 100, 100],
      alt: { text: "Signature image" },
    });
  });
});
