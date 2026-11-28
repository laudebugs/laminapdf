import { describe, expect, it } from "vitest";
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
    // With no alternative text the op carries none, so a PDF/UA file refuses it and the person is asked.
    expect(
      (addImageOp("p1", "a", "b", [1, 0, 0, 1, 0, 0], undefined, WHEN) as { args: object }).args,
    ).not.toHaveProperty("alt");
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
    const args = (op as unknown as { args: Record<string, unknown> }).args;
    expect(args.matrix).toEqual([1, 0, 0, 1, 0, 0]);
    expect(args.object).toEqual({
      shape: { path: { rect: { rect: [5, 20, 10, 60] } }, stroke: { color: [0, 0, 0], width: 2, dash: "dashed" } },
    });
    expect(args.alt).toBe("decorative");
  });

  it("sets text in a box with its runs, font and size", () => {
    const op = addTextOp("p1", "t", [72, 72, 288, 100], "Hello", { ...look, bold: true }, WHEN);
    expect((op as unknown as { args: Record<string, unknown> }).args).toMatchObject({
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
