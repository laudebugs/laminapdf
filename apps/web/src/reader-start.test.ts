// Where reader mode's column starts, from the point at the top of the page view.
import { describe, expect, it } from "vitest";
import { blockAt } from "./reader-start.ts";
import type { DocumentReflow } from "./session.ts";

type Block = DocumentReflow["blocks"][number];

// A whole paragraph block, so a change to the block type fails the typecheck here.
const block = (page: number, bbox: Block["bbox"]): Block => ({ kind: "paragraph", text: "", page, bbox });

// Page 0: a heading at the top and a paragraph below it; page 1: a crop with no box, then a paragraph.
const BLOCKS = [
  block(0, [72, 72, 400, 20]),
  block(0, [72, 120, 400, 200]),
  block(1, null),
  block(1, [72, 72, 400, 100]),
];

describe("blockAt", () => {
  it("starts at the first block without a point or blocks", () => {
    expect(blockAt(BLOCKS, undefined)).toBe(0);
    expect(blockAt([], { page: 0, y: 0 })).toBe(0);
  });

  it("starts at the first block whose bottom is below the point on its page", () => {
    expect(blockAt(BLOCKS, { page: 0, y: 0 })).toBe(0);
    expect(blockAt(BLOCKS, { page: 0, y: 100 })).toBe(1);
  });

  it("takes the next page's first block when the point is past the page's last", () => {
    expect(blockAt(BLOCKS, { page: 0, y: 500 })).toBe(2);
  });

  it("skips a crop with no box on the point's own page", () => {
    expect(blockAt(BLOCKS, { page: 1, y: 0 })).toBe(3);
  });

  it("ends at the last block past the last page's blocks", () => {
    expect(blockAt(BLOCKS, { page: 1, y: 700 })).toBe(3);
    expect(blockAt(BLOCKS, { page: 5, y: 0 })).toBe(3);
  });
});
