// The tile planner against Rust's TilePlan (lamina-render's request.rs tests, same cases).
import { describe, expect, it } from "vitest";
import { DEFAULT_LIMITS, isRenderError, pageToTile, planTile, RenderError } from "../src/index.js";

const LETTER = { width: 612, height: 792, rotate: 0 };

function code(f: () => unknown): string | undefined {
  try {
    f();
  } catch (e) {
    return isRenderError(e) ? e.code : String(e);
  }
  return undefined;
}

describe("planTile", () => {
  it("floors points times scale for the whole page", () => {
    const plan = planTile(LETTER, { page: 0, scale: 96 / 72 }, DEFAULT_LIMITS);
    expect([plan.pageWidth, plan.pageHeight, plan.byteLength]).toEqual([816, 1056, 816 * 1056 * 4]);
  });

  it("swaps the page size for a quarter turn", () => {
    const plan = planTile(LETTER, { page: 0, scale: 1, rotation: 90 }, DEFAULT_LIMITS);
    expect([plan.pageWidth, plan.pageHeight]).toEqual([792, 612]);
  });

  it("rejects a non-finite or non-positive scale", () => {
    for (const scale of [0, -1, Number.NaN, Number.POSITIVE_INFINITY]) {
      expect(code(() => planTile(LETTER, { page: 0, scale }, DEFAULT_LIMITS))).toBe("invalid-argument");
    }
  });

  it("rejects a page without area", () => {
    expect(code(() => planTile({ width: 0, height: 792, rotate: 0 }, { page: 0, scale: 1 }, DEFAULT_LIMITS))).toBe(
      "invalid-argument",
    );
  });

  it("rejects a tile past the page, empty or fractional", () => {
    const clip = (x: number, width: number) => ({ page: 0, scale: 1, clip: { x, y: 0, width, height: 10 } });
    expect(code(() => planTile(LETTER, clip(600, 13), DEFAULT_LIMITS))).toBe("invalid-argument");
    expect(code(() => planTile(LETTER, clip(0, 0), DEFAULT_LIMITS))).toBe("invalid-argument");
    expect(code(() => planTile(LETTER, clip(0.5, 10), DEFAULT_LIMITS))).toBe("invalid-argument");
  });

  it("refuses a tile over the pixel limit but allows a small tile of a huge zoom", () => {
    expect(code(() => planTile(LETTER, { page: 0, scale: 1000 }, DEFAULT_LIMITS))).toBe("render-too-large");
    const plan = planTile(
      LETTER,
      { page: 0, scale: 1000, clip: { x: 300_000, y: 400_000, width: 512, height: 512 } },
      DEFAULT_LIMITS,
    );
    expect(plan.byteLength).toBe(512 * 512 * 4);
  });
});

describe("pageToTile", () => {
  const apply = (m: readonly number[], x: number, y: number) => [
    (m[0] ?? 0) * x + (m[2] ?? 0) * y + (m[4] ?? 0),
    (m[1] ?? 0) * x + (m[3] ?? 0) * y + (m[5] ?? 0),
  ];
  const page = { width: 100, height: 50, rotate: 0 };

  it("maps the corners, rotations and tile shift like Rust's page_to_tile", () => {
    const up = planTile(page, { page: 0, scale: 2 }, DEFAULT_LIMITS);
    expect(apply(pageToTile(up, page, 0), 100, 50)).toEqual([200, 100]);
    const turned = planTile(page, { page: 0, scale: 2, rotation: 90 }, DEFAULT_LIMITS);
    expect(apply(pageToTile(turned, page, 90), 0, 0)).toEqual([100, 0]);
    const shifted = planTile(page, { page: 0, scale: 2, clip: { x: 10, y: 20, width: 5, height: 5 } }, DEFAULT_LIMITS);
    expect(apply(pageToTile(shifted, page, 0), 5, 10)).toEqual([0, 0]);
  });
});

describe("RenderError", () => {
  it("carries the code, its kind and pending ranges", () => {
    const e = new RenderError("data-pending", "missing", [[0, 10]]);
    expect([e.name, e.code, e.kind, e.ranges]).toEqual(["RenderError", "data-pending", "state", [[0, 10]]]);
    expect(isRenderError(e, "data-pending")).toBe(true);
    expect(isRenderError(e, "render-failed")).toBe(false);
  });
});
