// Flat-form preset helpers (E8n): borderless FreeText ops for Text, Check and Date.

import { describe, expect, it } from "vitest";
import {
  CHECK_GLYPH,
  flatStampOp,
  flatTextEdit,
  flatTextFinished,
  localeDate,
  needsGlyphFallback,
} from "./flat-tools.ts";

const context = {
  pageId: "p1",
  toUser: ([x, y]: readonly [number, number]) => ({ x, y: 792 - y }),
  rotation: 0,
  fallback: "blob-lamina-sans",
  created: new Date("2026-11-23T12:00:00Z"),
};

describe("flat-form presets (E8n)", () => {
  it("opens a borderless typewriter edit for the Text tool", () => {
    const edit = flatTextEdit(0, [100, 200]);
    expect(edit.kind).toBe("typewriter");
    expect(edit.look.border).toBe(0);
    expect(edit.look.font).toBe("helvetica");
    expect(edit.rect).toEqual([100, 192, 220, 208]);
  });

  it("drops a blank Text box and commits a non-blank one as one addAnnot", () => {
    expect(flatTextFinished({ text: [[{ text: "   " }]], rect: [10, 20, 130, 36] }, context)).toBeUndefined();
    const made = flatTextFinished({ text: [[{ text: "Ada Lovelace" }]], rect: [10, 20, 130, 36] }, context);
    expect(made?.label).toBe("Add text");
    expect(made?.ops).toHaveLength(1);
    const args = made?.ops[0] as { op: string; args: { spec: Record<string, unknown> } };
    expect(args.op).toBe("addAnnot");
    expect(args.args.spec.border).toEqual({ width: 0 });
    expect(args.args.spec.intent).toEqual({ type: "typewriter" });
    expect(args.args.spec.fallbacks).toBeUndefined();
  });

  it("places a Check tick with the bundled fallback font", () => {
    const made = flatStampOp("check", [150, 250], context);
    expect(made.label).toBe("Add check mark");
    expect(made.ops).toHaveLength(1);
    const op = made.ops[0] as { op: string; args: { spec: Record<string, unknown> } };
    expect(op.op).toBe("addAnnot");
    expect(op.args.spec.text).toEqual([[{ text: CHECK_GLYPH }]]);
    expect(op.args.spec.fallbacks).toEqual(["blob-lamina-sans"]);
    expect(op.args.spec.border).toEqual({ width: 0 });
  });

  it("places today's Date formatted in the requested locale", () => {
    const made = flatStampOp("date", [200, 300], context, "en-GB");
    expect(made.label).toBe("Add date");
    const op = made.ops[0] as { op: string; args: { spec: { text: { text: string }[][] } } };
    expect(op.args.spec.text[0]?.[0]?.text).toBe(localeDate(context.created, "en-GB"));
    expect(needsGlyphFallback([[{ text: "23/11/2026" }]])).toBe(false);
    expect(needsGlyphFallback([[{ text: CHECK_GLYPH }]])).toBe(true);
  });
});
