import type { ObjectRef, PageObject } from "@laminapdf/sdk";
import { describe, expect, it } from "vitest";
import { apply, boxAround, corners, frameOf } from "./geometry.ts";
import type { Item } from "./objects.ts";
import {
  boxFrame,
  carrying,
  deleteEdits,
  followed,
  hitOriginal,
  inMarquee,
  ordinalOf,
  originalsOn,
  pickKind,
  pickOf,
  pickRefusal,
  reorderEdits,
  restacked,
  transformEdits,
} from "./original.ts";

const ref = (ordinal: number): ObjectRef => ({ ordinal, hash: `h${ordinal}` }) as unknown as ObjectRef;
const object = (ordinal: number, kind: PageObject["kind"], bounds?: [number, number, number, number], extra = {}) =>
  ({ object: ref(ordinal), kind, ...(bounds === undefined ? {} : { bounds }), ...extra }) as unknown as PageObject;

const page = [
  object(0, "path", [0, 0, 600, 800]),
  object(1, "image", [100, 100, 300, 200]),
  object(2, "text", [50, 300, 250, 320]),
  object(3, "path", [120, 120, 180, 160]),
  object(4, "path"),
  object(5, "shading", [0, 0, 10, 10]),
  object(6, "path", [400, 400, 500, 500], { readOnly: "clipping-path" }),
];

describe("which objects Select lists", () => {
  const originals = originalsOn(0, page, []);

  it("leaves out an object that draws nothing", () => {
    expect(originals.map((o) => ordinalOf(o.ref))).toEqual([0, 1, 2, 3, 5, 6]);
  });

  it("lists a kind it doesn't edit yet as read-only, with a reason", () => {
    expect(originals.find((o) => ordinalOf(o.ref) === 5)?.readOnly).toBe("unsupported");
    expect(originals.find((o) => ordinalOf(o.ref) === 6)?.readOnly).toBe("clipping-path");
    expect(originals.find((o) => ordinalOf(o.ref) === 1)?.readOnly).toBeUndefined();
  });

  it("leaves out a form that is one of the objects Lamina PDF added", () => {
    const mine = { frame: boxFrame([10, 10, 60, 40]) } as unknown as Item;
    const forms = [object(0, "form", [10, 10, 60, 40]), object(1, "form", [200, 10, 260, 40])];
    expect(originalsOn(0, forms, [mine]).map((o) => ordinalOf(o.ref))).toEqual([1]);
  });
});

describe("what a press lands on", () => {
  const originals = originalsOn(0, page, []);

  it("is the topmost object under the point", () => {
    expect(ordinalOf(hitOriginal(originals, { x: 130, y: 130 })?.ref ?? ref(-1))).toBe(3);
    expect(ordinalOf(hitOriginal(originals, { x: 250, y: 150 })?.ref ?? ref(-1))).toBe(1);
  });

  it("has a couple of points of slop around an object", () => {
    expect(hitOriginal(originals, { x: 301, y: 150 })).toBeDefined();
    expect(ordinalOf(hitOriginal(originals, { x: 303, y: 150 })?.ref ?? ref(-1))).toBe(0);
  });

  it("takes into a marquee only what it holds whole and can change", () => {
    const taken = inMarquee(originals, { x: 90, y: 90 }, { x: 310, y: 210 });
    expect(taken.map((o) => ordinalOf(o.ref))).toEqual([1, 3]);
    expect(inMarquee(originals, { x: 390, y: 390 }, { x: 510, y: 510 })).toEqual([]);
  });
});

describe("what a pick is", () => {
  const originals = originalsOn(0, page, []);
  const by = (...ordinals: number[]) =>
    pickOf(
      0,
      originals.filter((o) => ordinals.includes(ordinalOf(o.ref))),
    );

  it("is boxed around its objects", () => {
    expect(by(1, 2)?.box).toEqual([50, 100, 300, 320]);
  });

  it("is an image, a text block, shapes or a mix", () => {
    expect(pickKind(by(1) ?? never())).toBe("image");
    expect(pickKind(by(2) ?? never())).toBe("text");
    expect(pickKind(by(0, 3) ?? never())).toBe("shape");
    expect(pickKind(by(1, 2) ?? never())).toBe("mixed");
  });

  it("can't be changed when one of its objects can't", () => {
    expect(pickRefusal(by(1, 6) ?? never())).toBe("clipping-path");
    expect(pickRefusal(by(1) ?? never())).toBeUndefined();
  });

  it("is followed by the ordinals its objects keep", () => {
    const moved = originalsOn(
      0,
      [...page].reverse().map((o, i) => ({ ...o, object: ref(i) }) as PageObject),
      [],
    );
    expect(followed(0, [1], moved)?.objects).toHaveLength(1);
    expect(followed(0, [99], moved)).toBeUndefined();
  });
});

function never(): never {
  throw new Error("no pick");
}

describe("the edits a change makes", () => {
  const originals = originalsOn(0, page, []);
  const pick = pickOf(
    0,
    originals.filter((o) => [1, 3].includes(ordinalOf(o.ref))),
  );

  it("moves what drew in the old frame into the new one", () => {
    if (pick === undefined) throw new Error("no pick");
    const from = boxFrame(pick.box);
    const to = frameOf(150, 140, 400, 200);
    const [first] = transformEdits(pick, from, to) as unknown as { transform: { matrix: number[] } }[];
    const matrix = first?.transform.matrix as [number, number, number, number, number, number];
    const [c0, , c2] = corners(from);
    const [m0, , m2] = corners(to);
    expect(apply(matrix, c0)).toEqual(expect.objectContaining({ x: expect.closeTo(m0.x), y: expect.closeTo(m0.y) }));
    expect(apply(matrix, c2)).toEqual(expect.objectContaining({ x: expect.closeTo(m2.x), y: expect.closeTo(m2.y) }));
    expect(transformEdits(pick, from, to)).toHaveLength(2);
  });

  it("makes no edit for a frame that collapses", () => {
    if (pick === undefined) throw new Error("no pick");
    expect(carrying([0, 0, 0, 0, 0, 0], frameOf(0, 0, 1, 1))).toBeUndefined();
    expect(transformEdits(pick, [0, 0, 0, 0, 0, 0], frameOf(0, 0, 1, 1))).toEqual([]);
  });

  it("deletes each object once", () => {
    if (pick === undefined) throw new Error("no pick");
    expect(deleteEdits(pick)).toEqual([{ delete: { object: ref(1) } }, { delete: { object: ref(3) } }]);
  });

  it("brings the pick to the front or sends it back, object by object", () => {
    if (pick === undefined) throw new Error("no pick");
    expect(reorderEdits(pick, page, "front")).toEqual([
      { reorder: { object: ref(1), to: "front" } },
      { reorder: { object: ref(3), to: "front" } },
    ]);
  });

  it("steps past the nearest object that isn't picked", () => {
    const one = pickOf(
      0,
      originals.filter((o) => ordinalOf(o.ref) === 1),
    );
    if (one === undefined) throw new Error("no pick");
    expect(reorderEdits(one, page, "forward")).toEqual([{ reorder: { object: ref(1), to: { after: ref(2) } } }]);
    expect(reorderEdits(one, page, "backward")).toEqual([{ reorder: { object: ref(1), to: { before: ref(0) } } }]);
    const bottom = pickOf(
      0,
      originals.filter((o) => ordinalOf(o.ref) === 0),
    );
    if (bottom === undefined) throw new Error("no pick");
    expect(reorderEdits(bottom, page, "backward")).toEqual([]);
  });

  it("knows where the picked objects are after a restack", () => {
    expect(restacked([0, 1, 2, 3], [1], "front")).toEqual([0, 2, 3, 1]);
    expect(restacked([0, 1, 2, 3], [1], "back")).toEqual([1, 0, 2, 3]);
    expect(restacked([0, 1, 2, 3], [1], "forward")).toEqual([0, 2, 1, 3]);
    expect(restacked([0, 1, 2, 3], [1, 2], "forward")).toEqual([0, 3, 1, 2]);
    expect(restacked([0, 1, 2, 3], [2], "backward")).toEqual([0, 2, 1, 3]);
    expect(restacked([0, 1, 2, 3], [0], "backward")).toEqual([0, 1, 2, 3]);
  });

  it("boxes a frame by its corners", () => {
    expect(boxAround(corners(frameOf(10, 20, 30, 40)))).toEqual([10, 20, 40, 60]);
  });
});
