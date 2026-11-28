import { BATES_DEFAULT, HEADER_FOOTER_DEFAULT, WATERMARK_DEFAULT } from "@laminapdf/ui/editing";
import { describe, expect, it } from "vitest";
import { addMarkOp, batesSpec, deleteMarkOp, headerFooterSpec, newMarkId, updateMarkOp, watermarkSpec } from "./ops.ts";

const WHEN = new Date(Date.UTC(2026, 10, 27, 12, 0, 0));
const watermark = { ...WATERMARK_DEFAULT, text: "DRAFT", colour: "red", hideable: true } as const;

describe("a mark's ID", () => {
  it("is one the core takes", () => {
    expect(newMarkId(1_700_000_000_000, () => 0.5)).toMatch(/^[A-Za-z0-9._-]{1,64}$/);
  });
});

describe("the watermark tab", () => {
  it("adds a watermark with its fonts, colour, fade, turn and layer", () => {
    const mark = watermarkSpec(watermark, ["helvetica"], undefined);
    expect(mark.kind).toBe("watermark");
    const op = addMarkOp(["p1", "p2"], "m1", mark, WHEN) as unknown as { op: string; args: Record<string, unknown> };
    expect(op.op).toBe("addWatermark");
    expect(op.args).toMatchObject({
      id: "m1",
      pages: ["p1", "p2"],
      opacity: 0.3,
      angle: 45,
      hideable: true,
      modified: "D:20261127120000Z",
      mark: { text: { text: "DRAFT", size: 96, fonts: ["helvetica"] } },
    });
  });

  it("is a background when it lies behind the content", () => {
    const mark = watermarkSpec({ ...watermark, layer: "behind", opacity: 100, angle: 0 }, ["times"], undefined);
    const op = addMarkOp(["p1"], "m1", mark, WHEN) as unknown as { op: string; args: Record<string, unknown> };
    expect(op.op).toBe("addBackground");
    expect(op.args).not.toHaveProperty("opacity");
    expect(op.args).not.toHaveProperty("angle");
  });

  it("shows a picture by its blob", () => {
    const mark = watermarkSpec({ ...WATERMARK_DEFAULT, source: "image" }, [], "abc");
    expect(mark.spec).toMatchObject({ mark: { image: { blob: "abc" } } });
  });
});

describe("the header and footer tab", () => {
  it("writes each slot's tokens as braces and leaves an empty slot null", () => {
    const mark = headerFooterSpec(HEADER_FOOTER_DEFAULT, ["helvetica"], "27 Nov 2026", "report.pdf");
    expect(mark.spec).toMatchObject({
      slots: [null, null, null, null, null, "{page} of {pages}"],
      size: 9,
      date: "27 Nov 2026",
      file: "report.pdf",
      margins: { top: 36, bottom: 36 },
    });
    const op = addMarkOp(["p1"], "m2", mark, WHEN) as unknown as { op: string };
    expect(op.op).toBe("setHeaderFooter");
  });
});

describe("the Bates tab", () => {
  it("numbers from the start number in one slot", () => {
    const mark = batesSpec({ ...BATES_DEFAULT, prefix: "NHT-", start: 7 }, ["helvetica"]);
    const op = addMarkOp(["p1", "p2"], "m3", mark, WHEN) as unknown as { op: string; args: Record<string, unknown> };
    expect(op.op).toBe("applyBates");
    expect(op.args).toMatchObject({ prefix: "NHT-", start: 7, digits: 6, slot: "bottomRight" });
  });
});

describe("update and remove", () => {
  it("replace a mark by its kind, on the pages it is on", () => {
    const mark = watermarkSpec(watermark, ["helvetica"], undefined);
    const update = updateMarkOp(["p1"], "m1", mark, WHEN) as unknown as { op: string; args: Record<string, unknown> };
    expect(update.op).toBe("updatePageMark");
    expect(update.args).toMatchObject({ id: "m1", pages: ["p1"], mark: { watermark: { opacity: 0.3 } } });
    expect(deleteMarkOp(["p1", "p2"], "m1")).toEqual({ op: "deletePageMark", args: { id: "m1", pages: ["p1", "p2"] } });
  });
});
