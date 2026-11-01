// ADR 0018 §5, TS half: the frozen op samples (fixtures/ops/v1) apply through doc.apply and
// round-trip through the TS types; unknown ops and fields are refused.
import { Document, init, isLaminaPdfError, type Op, type OpLog } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read, readText, thrown } from "./helpers.js";

const sample = <T>(name: string): T => JSON.parse(readText(`fixtures/ops/v1/${name}.json`)) as T;

/** A document whose page 0 has the ID the samples name (`p3.0`, `p4.0` are retargeted). */
function doc(): { readonly doc: Document; readonly page: string } {
  const d = Document.open(read("fixtures/golden/examples/links_and_outlines.pdf"));
  return { doc: d, page: d.page(0).id };
}

beforeAll(() => init());

describe("fixtures/ops/v1", () => {
  it("applies set-info.json", () => {
    const { doc: d } = doc();
    const op = sample<Op>("set-info");
    d.apply([op]);
    expect(d.info.keywords).toBe("Привет, 世界");
  });

  it("applies rotate-page.json to the page it names", () => {
    const { doc: d, page } = doc();
    const op = sample<{ op: "rotatePage"; args: { page: string; rotate: 90 } }>("rotate-page");
    d.apply([{ ...op, args: { ...op.args, page } }]);
    expect(d.pageInfo(0).rotation).toBe(90);
  });

  it("applies envelope.json as an op log", () => {
    const { doc: d, page } = doc();
    const log = sample<OpLog>("envelope");
    const ops = log.ops.map((o) => (o.op === "rotatePage" ? { ...o, args: { ...o.args, page } } : o));
    d.applyLog({ v: 1, ops });
    expect([d.info.title, d.pageInfo(0).rotation]).toEqual(["Quarterly report", 270]);
  });

  it("round-trips every sample through JSON unchanged", () => {
    for (const name of ["set-info", "rotate-page", "envelope"]) {
      const value = sample<unknown>(name);
      expect(JSON.parse(JSON.stringify(value))).toEqual(JSON.parse(readText(`fixtures/ops/v1/${name}.json`)));
    }
  });

  it("refuses an unknown op", () => {
    const e = thrown(() => doc().doc.apply([{ op: "deletePage", args: {} } as never]));
    expect(isLaminaPdfError(e, "invalid-argument")).toBe(true);
  });

  it("refuses an unknown field", () => {
    const op = { op: "setInfo", args: { patch: { title: "x" }, when: "now" } } as never;
    const e = thrown(() => doc().doc.apply([op]));
    expect(isLaminaPdfError(e) && ["invalid-argument", "unknown-option"].includes(e.code)).toBe(true);
  });

  it("applies nothing when one op of a batch fails", () => {
    const { doc: d } = doc();
    const bad = { op: "rotatePage", args: { page: "p999.0", rotate: 90 } } as Op;
    thrown(() => d.apply([sample<Op>("set-info"), bad]));
    expect(d.hasUnsavedChanges).toBe(false);
  });
});
