import { describe, expect, it } from "vitest";
import { GATED, gate, gateTable, MARGIN } from "./gate.ts";

// The release gate: medians against §4's targets plus the margin, waivers, and a figure no run
// measured failing rather than passing.
const at = (fraction: number) =>
  Object.fromEntries(Object.entries(GATED).map(([figure, { target }]) => [figure, target * fraction]));

describe("the release gate", () => {
  it("passes figures up to their target plus the margin, by the median of the runs", () => {
    const runs = [at(0.5), at(1 + MARGIN), at(9)];
    const { rows, pass } = gate(runs, []);
    expect(pass).toBe(true);
    expect(rows.every((row) => row.median === (GATED[row.figure]?.target ?? 0) * (1 + MARGIN))).toBe(true);
  });

  it("fails a median over the limit unless a waiver names it", () => {
    const runs = [at(1), at(2), at(2)];
    expect(gate(runs, []).pass).toBe(false);
    const waivers = Object.keys(GATED).map((figure) => ({ figure, issue: "#1", until: "web v0.2" }));
    const waived = gate(runs, waivers);
    expect(waived.pass).toBe(true);
    expect(gateTable(waived.rows, 3)).toContain("waived until web v0.2 (#1)");
  });

  it("fails a figure no run measured", () => {
    const { "first-page-cold": _cold, ...rest } = at(0.5);
    const { rows, pass } = gate([rest, rest, rest], []);
    expect(pass).toBe(false);
    expect(rows.find((row) => row.figure === "first-page-cold")?.median).toBeNull();
  });
});
