import type { Op } from "@laminapdf/sdk";
import { describe, expect, it } from "vitest";
import {
  calloutLine,
  finishedEdit,
  placedBox,
  placedCallout,
  type TextContext,
  type TextEdit,
} from "./text-session.ts";

/** A letter page shown upright: page space to user space flips y. */
const context: TextContext = {
  pageId: "p1",
  toUser: ([x, y]) => ({ x, y: 792 - y }),
  rotation: 0,
  fallback: "blob-arabic",
  author: "R. Osei",
  created: new Date(Date.UTC(2026, 10, 18, 9, 12, 0)),
};

const look = { font: "helvetica", size: 12, align: "left", colour: [0, 0, 0], border: 1 } as const;

/** A new text box at (72, 72), 180 × 24. */
const fresh: TextEdit = {
  page: 0,
  kind: "textBox",
  id: undefined,
  rect: [72, 72, 252, 96],
  text: [[]],
  look,
  fallbacks: [],
};

/** The same box, already in the document as `a12.0`, holding "Hello". */
const existing: TextEdit = { ...fresh, id: "a12.0", text: [[{ text: "Hello" }]] };

const args = (op: Op | undefined) =>
  (op as { args: Record<string, Record<string, unknown> | undefined> } | undefined)?.args;

describe("placing a box", () => {
  it("takes the drag, or the default size at the press for a click", () => {
    expect([placedBox("textBox", [10, 10], [100, 20]), placedBox("textBox", [10, 10], [12, 11])]).toEqual([
      [10, 10, 100, 34],
      [10, 10, 190, 34],
    ]);
  });

  it("runs a callout's line from the point it points at to the nearer side of its box, through a knee", () => {
    expect(calloutLine([10, 50], [100, 40, 200, 60])).toEqual([
      [10, 50],
      [84, 50],
      [100, 50],
    ]);
  });

  it("puts a clicked callout's box up and to the right of the point", () => {
    const { rect, line } = placedCallout([100, 300], [100, 300]);
    expect([rect, line[0]]).toEqual([
      [148, 228, 308, 252],
      [100, 300],
    ]);
  });
});

describe("finishedEdit", () => {
  it("makes nothing for a new box left blank", () => {
    expect(finishedEdit(fresh, { text: [[{ text: "  " }]], rect: fresh.rect }, context)).toBeUndefined();
  });

  it("adds a new box with its text, look and author, in user space", () => {
    const made = finishedEdit(fresh, { text: [[{ text: "Check", bold: true }]], rect: [72, 72, 252, 110] }, context);
    expect([made?.label, args(made?.ops[0])]).toEqual([
      "Add text box",
      {
        page: "p1",
        spec: {
          type: "freeText",
          rect: { x0: 72, y0: 682, x1: 252, y1: 720 },
          text: [[{ text: "Check", bold: true }]],
          font: "helvetica",
          size: 12,
          align: "left",
          intent: { type: "textBox" },
          border: { width: 1 },
        },
        common: { color: [0, 0, 0], author: "R. Osei", created: "D:20261118091200Z", modified: "D:20261118091200Z" },
      },
    ]);
  });

  it("names the fallback font for Arabic", () => {
    const made = finishedEdit(fresh, { text: [[{ text: "مرحبا" }]], rect: fresh.rect }, context);
    expect((args(made?.ops[0])?.spec as { fallbacks?: string[] } | undefined)?.fallbacks).toEqual(["blob-arabic"]);
  });

  it("writes typewriter text with no border and no fill", () => {
    const typed = { ...fresh, kind: "typewriter" as const, look: { ...look, fill: [1, 1, 1] as const } };
    const spec = args(finishedEdit(typed, { text: [[{ text: "Name" }]], rect: fresh.rect }, context)?.ops[0])?.spec;
    expect([spec?.intent, spec?.border, spec?.fill]).toEqual([{ type: "typewriter" }, { width: 0 }, undefined]);
  });

  it("writes a callout's line, ending in an open arrow", () => {
    const callout: TextEdit = {
      ...fresh,
      kind: "callout",
      line: [
        [10, 50],
        [84, 50],
        [100, 50],
      ],
    };
    const now = { text: [[{ text: "Why?" }]], rect: fresh.rect, line: callout.line ?? [] };
    expect(args(finishedEdit(callout, now, context)?.ops[0])?.spec).toMatchObject({
      intent: {
        type: "callout",
        end: "openArrow",
        line: [
          { x: 10, y: 742 },
          { x: 84, y: 742 },
          { x: 100, y: 742 },
        ],
      },
    });
  });

  it("deletes a box emptied of its text", () => {
    const made = finishedEdit(existing, { text: [[]], rect: existing.rect }, context);
    expect([made?.label, made?.ops]).toEqual(["Delete text box", [{ op: "deleteAnnot", args: { id: "a12.0" } }]]);
  });

  it("makes nothing when the text and the box are as they were", () => {
    expect(finishedEdit(existing, { text: [[{ text: "Hello" }]], rect: existing.rect }, context)).toBeUndefined();
  });

  it("changes only what changed, in one updateAnnot", () => {
    const made = finishedEdit(existing, { text: [[{ text: "Hello there" }]], rect: existing.rect }, context);
    const change = args(made?.ops[0]);
    expect([made?.label, made?.ops.length, change?.id, change?.spec]).toEqual([
      "Change text box",
      1,
      "a12.0",
      { type: "freeText", text: [[{ text: "Hello there" }]] },
    ]);
  });

  it("changes the box when it grew as the text wrapped", () => {
    const made = finishedEdit(existing, { text: existing.text, rect: [72, 72, 252, 120] }, context);
    expect(args(made?.ops[0])?.spec).toEqual({ type: "freeText", rect: { x0: 72, y0: 672, x1: 252, y1: 720 } });
  });
});

describe("finishedEdit with a new look", () => {
  it("writes the inspector's font, size and colour in the same edit as the text", () => {
    const now = {
      text: [[{ text: "Hello there" }]],
      rect: existing.rect,
      look: { ...look, font: "times", size: 14, colour: [1, 0, 0] as const },
    };
    expect(args(finishedEdit(existing, now, context)?.ops[0])).toMatchObject({
      color: [1, 0, 0],
      spec: { type: "freeText", text: [[{ text: "Hello there" }]], font: "times", size: 14 },
    });
  });

  it("changes a callout's line end through its intent", () => {
    const line = [
      [10, 50],
      [84, 50],
      [100, 50],
    ] as const;
    const callout: TextEdit = { ...existing, kind: "callout", line, look: { ...look, end: "openArrow" } };
    const now = { text: existing.text, rect: existing.rect, line, look: { ...look, end: "circle" as const } };
    expect(args(finishedEdit(callout, now, context)?.ops[0])?.spec).toMatchObject({
      intent: { type: "callout", end: "circle" },
    });
  });
});
