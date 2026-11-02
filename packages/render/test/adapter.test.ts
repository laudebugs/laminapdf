// The helpers both web adapters share: page index, open document, input size, reads and pending ranges.
import { describe, expect, it } from "vitest";
import {
  checkInputSize,
  checkOpen,
  checkPageIndex,
  isRenderError,
  mergeRanges,
  pendingError,
  RenderError,
  type RenderSource,
  readFailure,
  readRange,
} from "../src/index.js";

function thrown(f: () => unknown): unknown {
  try {
    f();
  } catch (e) {
    return e;
  }
  throw new Error("expected the call to throw");
}

describe("checkPageIndex", () => {
  it("accepts a page of the document and refuses anything else", () => {
    expect(() => checkPageIndex(2, 3)).not.toThrow();
    for (const index of [3, -1, 1.5, Number.NaN]) {
      expect(
        isRenderError(
          thrown(() => checkPageIndex(index, 3)),
          "page-index-out-of-range",
        ),
      ).toBe(true);
    }
  });
});

describe("checkOpen", () => {
  it("throws disposed once the document is closed", () => {
    expect(() => checkOpen(true)).not.toThrow();
    expect(
      isRenderError(
        thrown(() => checkOpen(false)),
        "disposed",
      ),
    ).toBe(true);
  });
});

describe("checkInputSize", () => {
  it("throws render-too-large over the limit only", () => {
    expect(() => checkInputSize(10, 10)).not.toThrow();
    expect(
      isRenderError(
        thrown(() => checkInputSize(11, 10)),
        "render-too-large",
      ),
    ).toBe(true);
  });
});

describe("readRange", () => {
  const bytes = new Uint8Array([1, 2, 3, 4]);

  it("returns the bytes, or undefined while they haven't arrived", () => {
    const source: RenderSource = { size: 4, readSync: (o, l) => (o === 0 ? bytes.subarray(o, o + l) : undefined) };
    expect(readRange(source, 0, 2)).toEqual(new Uint8Array([1, 2]));
    expect(readRange(source, 2, 2)).toBeUndefined();
  });

  it("turns a short read or a throwing source into a source error, and keeps the source's own RenderError", () => {
    const short: RenderSource = { size: 4, readSync: () => bytes.subarray(0, 1) };
    expect(
      isRenderError(
        thrown(() => readRange(short, 0, 2)),
        "source",
      ),
    ).toBe(true);
    const throwing: RenderSource = {
      size: 4,
      readSync: () => {
        throw new Error("disk");
      },
    };
    expect(
      isRenderError(
        thrown(() => readRange(throwing, 0, 2)),
        "source",
      ),
    ).toBe(true);
    const own = new RenderError("render-too-large", "the source's own");
    expect(readFailure(0, 2, own)).toBe(own);
  });
});

describe("mergeRanges", () => {
  it("sorts and merges overlapping and touching ranges", () => {
    expect(
      mergeRanges([
        [65_536, 65_536],
        [0, 65_536],
        [200_000, 10],
        [131_000, 100],
      ]),
    ).toEqual([
      [0, 131_100],
      [200_000, 10],
    ]);
  });

  it("makes the data-pending error carry the merged ranges", () => {
    const e = pendingError([
      [10, 10],
      [0, 10],
    ]);
    expect(isRenderError(e, "data-pending") && e.ranges).toEqual([[0, 20]]);
  });
});
