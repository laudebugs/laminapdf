import type { AddedObject } from "@laminapdf/sdk";
import { describe, expect, it } from "vitest";
import { frameOf, multiply } from "./geometry.ts";
import { itemOf, matrixFor, nameOf, pathBox } from "./objects.ts";
import { addImageOp, addShapeOp, addTextOp, deleteOp, newId, pathOf, runsOf, updateOp } from "./ops.ts";

const WHEN = new Date(Date.UTC(2026, 10, 27, 12, 0, 0));
const look = {
  fonts: ["helvetica"],
  size: 12,
  color: [0, 0, 0] as const,
  align: "left" as const,
  bold: false,
  italic: false,
};

describe("the ops Edit mode makes", () => {
  it("names a new object with a legal ID", () => {
    expect(newId(1_000_000, () => 0.5)).toMatch(/^[A-Za-z0-9_.-]{1,64}$/);
    expect(newId(1, () => 0.1)).not.toBe(newId(2, () => 0.1));
  });

  it("places a picture by its frame, with alt text and a date", () => {
    const op = addImageOp("p1", "a", "blob-1", [144, 0, 0, 44, 72, 40], "Logo", WHEN);
    expect(op).toEqual({
      op: "addContentObject",
      args: {
        id: "a",
        page: "p1",
        object: { image: { blob: "blob-1" } },
        matrix: [144, 0, 0, 44, 72, 40],
        opacity: 1,
        layer: "front",
        alt: { text: "Logo" },
        modified: "D:20261127120000Z",
      },
    });
  });

  it("draws a shape in page space with the identity matrix, as decoration", () => {
    const path = pathOf("rect", { x: 10, y: 20 }, { x: 5, y: 60 });
    const op = addShapeOp(
      "p1",
      "s",
      path,
      { fill: null, stroke: { color: [0, 0, 0], width: 2, dash: "dashed" } },
      WHEN,
    );
    const args = (op as { args: Record<string, unknown> }).args;
    expect(args.matrix).toEqual([1, 0, 0, 1, 0, 0]);
    expect(args.object).toEqual({
      shape: { path: { rect: { rect: [5, 20, 10, 60] } }, stroke: { color: [0, 0, 0], width: 2, dash: "dashed" } },
    });
    expect(args.alt).toBe("decorative");
  });

  it("sets text in a box with its runs, font and size", () => {
    const op = addTextOp("p1", "t", [72, 72, 288, 100], "Hello", { ...look, bold: true }, WHEN);
    expect((op as { args: Record<string, unknown> }).args).toMatchObject({
      id: "t",
      rect: [72, 72, 288, 100],
      runs: [{ text: "Hello", bold: true }],
      fonts: ["helvetica"],
      size: 12,
    });
    expect(runsOf("x", { bold: false, italic: false })).toEqual([{ text: "x" }]);
  });

  it("updates and deletes by page and ID", () => {
    expect(updateOp("p1", "a", { opacity: 0.5 }, WHEN)).toEqual({
      op: "updateContentObject",
      args: { id: "a", page: "p1", opacity: 0.5, modified: "D:20261127120000Z" },
    });
    expect(deleteOp("p1", "a")).toEqual({ op: "deleteContentObject", args: { id: "a", page: "p1" } });
  });
});

const added = (object: unknown, matrix: AddedObject["matrix"], extra: object = {}): AddedObject =>
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
