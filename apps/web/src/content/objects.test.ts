import type { AddedObject } from "@laminapdf/sdk";
import { describe, expect, it } from "vitest";
import { frameOf, type Matrix, multiply, reshaped, sizeOf } from "./geometry.ts";
import { itemOf, matrixFor, nameOf, pathBox, resizedShape } from "./objects.ts";

const added = (object: unknown, matrix: Matrix, extra: object = {}): AddedObject =>
  ({ id: "o", object, matrix, opacity: 1, layer: "front", bounds: [0, 0, 1, 1], ...extra }) as unknown as AddedObject;

describe("what an added object is on the page", () => {
  it("is a picture's matrix", () => {
    const item = itemOf(2, added({ image: { blob: "b" } }, [100, 0, 0, 50, 10, 20]));
    expect(item).toMatchObject({ kind: "image", page: 2, frame: [100, 0, 0, 50, 10, 20] });
    expect(item === undefined ? "" : nameOf(item)).toBe("object.image");
  });

  it("is a shape's box moved by its matrix", () => {
    const path = { ellipse: { rect: [50, 60, 150, 100] } };
    expect(pathBox(path as never)).toEqual([50, 60, 150, 100]);
    const item = itemOf(0, added({ shape: { path } }, [1, 0, 0, 1, 10, 0]));
    expect(item?.frame).toEqual(multiply([1, 0, 0, 1, 10, 0], frameOf(50, 60, 100, 40)));
    expect(item === undefined ? "" : nameOf(item)).toBe("object.ellipse");
  });

  it("is a text box above its matrix's origin, which is its bottom-left", () => {
    const item = itemOf(0, added({ text: { width: 200, height: 30, runs: [{ text: "x" }] } }, [1, 0, 0, -1, 72, 130]));
    expect(item?.frame).toEqual([200, 0, 0, 30, 72, 100]);
    if (item === undefined) throw new Error("no item");
    // Moving the frame moves the origin with it, a height below the top.
    expect(matrixFor(item, [200, 0, 0, 30, 80, 110])).toEqual([1, 0, 0, -1, 80, 140]);
  });

  it("gives a shape the matrix that makes the frame it is dragged to", () => {
    const path = { rect: { rect: [0, 0, 100, 50] } };
    const item = itemOf(0, added({ shape: { path } }, [1, 0, 0, 1, 0, 0]));
    if (item === undefined) throw new Error("no item");
    expect(matrixFor(item, frameOf(10, 10, 200, 100))).toEqual([2, 0, 0, 2, 10, 10]);
  });

  it("leaves out an object another version stored differently", () => {
    expect(itemOf(0, added(undefined, [1, 0, 0, 1, 0, 0]))).toBeUndefined();
  });
});

describe("frames and matrices", () => {
  const near = (a: readonly number[], b: readonly number[]) => {
    for (const [i, v] of b.entries()) expect(a[i] ?? Number.NaN).toBeCloseTo(v, 6);
  };
  const turned = frameOf(50, 60, 120, 40, 30);
  // A picture drawn mirrored (a negative height) and one turned and mirrored.
  const mirrored: Matrix = [100, 0, 0, -50, 20, 90];
  const both = reshaped(mirrored, { rotate: 40 });

  it("give an image back its own matrix, turned or mirrored", () => {
    for (const m of [turned, mirrored, both]) {
      const item = itemOf(0, added({ image: { blob: "b" } }, m));
      if (item === undefined) throw new Error("no item");
      near(matrixFor(item, item.frame), m);
    }
  });

  it("round-trip a shape's frame through its matrix, whatever the matrix", () => {
    const path = { rect: { rect: [10, 20, 110, 70] } };
    for (const m of [
      [1, 0, 0, 1, 0, 0],
      [0.8, 0.6, -0.6, 0.8, 30, 5],
      [1, 0, 0, -1, 0, 100],
    ] as Matrix[]) {
      const item = itemOf(0, added({ shape: { path } }, m));
      if (item === undefined) throw new Error("no item");
      near(matrixFor(item, item.frame), m);
    }
  });

  it("read a text box through a turned matrix and write it back, a new width included", () => {
    const m: Matrix = [0, 1, 1, 0, 40, 40];
    const item = itemOf(0, added({ text: { width: 100, height: 20, runs: [{ text: "x" }] } }, m));
    if (item === undefined) throw new Error("no item");
    near(matrixFor(item, item.frame), m);
    // Its width read along its own axis.
    expect(sizeOf(item.frame).width).toBeCloseTo(100);
    near(matrixFor(item, item.frame, 100), m);
  });

  it("rebuild a resized shape's path and keep its matrix, so the outline keeps its width", () => {
    const path = { rect: { rect: [10, 20, 110, 70] } };
    const item = itemOf(0, added({ shape: { path } }, [1, 0, 0, 1, 0, 0]));
    if (item === undefined) throw new Error("no item");
    expect(resizedShape(item, frameOf(10, 20, 200, 50))).toEqual({ rect: { rect: [10, 20, 210, 70] } });
  });

  it("rebuild a line without giving its flat box a size", () => {
    const path = { line: { from: { x: 10, y: 50 }, to: { x: 110, y: 50 } } };
    const item = itemOf(0, added({ shape: { path } }, [1, 0, 0, 1, 0, 0]));
    if (item === undefined) throw new Error("no item");
    const next = resizedShape(item, reshaped(item.frame, { width: 200 }));
    expect(next).toEqual({ line: { from: { x: 10, y: 50 }, to: { x: 210, y: 50 } } });
  });

  it("leave a shape under a scaling matrix to the matrix", () => {
    const item = itemOf(0, added({ shape: { path: { rect: { rect: [0, 0, 10, 10] } } } }, [2, 0, 0, 2, 0, 0]));
    if (item === undefined) throw new Error("no item");
    expect(resizedShape(item, frameOf(0, 0, 40, 40))).toBeUndefined();
  });
});
