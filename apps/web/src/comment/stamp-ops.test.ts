import type { Op } from "@laminapdf/sdk";
import { describe, expect, it } from "vitest";
import {
  attachFileOp,
  dynamicLines,
  type PlaceContext,
  placeStampOp,
  replaceFileOps,
  stampBox,
  stampSize,
} from "./stamp-ops.ts";

/** A letter page shown upright: page space to user space flips y. */
const context: PlaceContext = {
  pageId: "p1",
  toUser: ([x, y]) => ({ x, y: 792 - y }),
  rotation: 0,
  author: "R. Osei",
  created: new Date(Date.UTC(2026, 9, 29, 10, 14, 0)),
};

const args = (op: Op | undefined) => (op as { args: Record<string, unknown> } | undefined)?.args;

describe("a stamp's place", () => {
  it("centres its box on the click, at its kind's size, an image's proportions kept", () => {
    expect([
      stampBox([300, 100], stampSize({ kind: "standard", name: "approved" })),
      stampSize({ kind: "image", blob: "b", width: 400, height: 200 }),
    ]).toEqual([
      [210, 75, 390, 125],
      [150, 75],
    ]);
  });

  it("writes a dynamic stamp's name and time when it is placed, or the time alone with no name", () => {
    const when = new Date(Date.UTC(2026, 9, 29, 10, 14));
    const [title, line] = dynamicLines("approved", "R. Osei", when);
    const [, alone] = dynamicLines("received", undefined, when);
    expect([title, line?.startsWith("by R. Osei · "), alone?.startsWith("by")]).toEqual(["Approved", true, false]);
  });
});

describe("placeStampOp", () => {
  it("adds a standard stamp in user space, with the author and dates", () => {
    expect(args(placeStampOp({ kind: "standard", name: "notApproved" }, [300, 100], context))).toEqual({
      page: "p1",
      spec: { type: "stamp", rect: { x0: 210, y0: 667, x1: 390, y1: 717 }, stamp: { standard: "notApproved" } },
      common: { author: "R. Osei", created: "D:20261029101400Z", modified: "D:20261029101400Z" },
    });
  });

  it("writes a dynamic stamp's lines in its colour, and an image by its blob, turned with the page", () => {
    const turned = { ...context, rotation: 90 };
    const dynamic = args(
      placeStampOp({ kind: "dynamic", name: "notApproved", lines: ["Not approved", "by A"] }, [0, 0], turned),
    );
    const image = args(placeStampOp({ kind: "image", blob: "img", width: 1, height: 1 }, [0, 0], context));
    expect([dynamic?.spec, (image?.spec as { stamp?: unknown }).stamp]).toEqual([
      expect.objectContaining({
        rotation: 90,
        stamp: { dynamic: { lines: ["Not approved", "by A"], style: "red" } },
      }),
      { image: "img" },
    ]);
  });
});

describe("attaching a file", () => {
  const file = { blob: "f1", name: "readings.csv", mime: "text/csv", modified: new Date(Date.UTC(2026, 0, 2)) };

  it("attaches it under a paperclip at the click, in the icon's colour", () => {
    expect(args(attachFileOp(file, [40, 40], context, { color: [0.9, 0.8, 0.4] }))).toMatchObject({
      spec: {
        type: "fileAttachment",
        at: { x: 40, y: 752 },
        icon: "paperclip",
        file: { file: "f1", name: "readings.csv", mime: "text/csv", modified: "D:20260102000000Z" },
      },
      common: { color: [0.9, 0.8, 0.4] },
    });
  });

  it("replaces a file in one step: the annotation deleted, a new one where it was with its icon and description", () => {
    const ops = replaceFileOps("a12.0", { ...file, description: "Daily readings" }, [40, 40], context, {
      icon: "graph",
    });
    expect([ops[0], (args(ops[1])?.spec as { icon: string; file: { description: string } }).icon]).toEqual([
      { op: "deleteAnnot", args: { id: "a12.0" } },
      "graph",
    ]);
    expect((args(ops[1])?.spec as { file: { description: string } }).file.description).toBe("Daily readings");
  });
});
