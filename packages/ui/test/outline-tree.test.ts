// The outline tree's current-section rule: the last item in document order at or before the top
// of the viewport, which is the deepest one when the outline follows the document.
import { describe, expect, it } from "vitest";
import { currentOutlineItem, type OutlineTreeItem } from "../src/index.ts";

const item = (id: string, page: number | undefined, y = 0, children: OutlineTreeItem[] = []): OutlineTreeItem => ({
  id,
  title: id,
  ...(page === undefined ? {} : { at: { page, y } }),
  children,
});

const OUTLINE = [
  item("intro", 0),
  item("ch1", 2, 0, [item("ch1.1", 2, 0.5), item("ch1.2", 4)]),
  item("web", undefined),
  item("ch2", 6),
];

describe("currentOutlineItem", () => {
  it("is undefined above the first item", () => {
    expect(currentOutlineItem([item("late", 3)], { page: 1, y: 0 })).toBeUndefined();
  });

  it("is the deepest item at or before the top", () => {
    expect(currentOutlineItem(OUTLINE, { page: 3, y: 0.2 })).toBe("ch1.1");
  });

  it("is the parent above its first child's place", () => {
    expect(currentOutlineItem(OUTLINE, { page: 2, y: 0.4 })).toBe("ch1");
  });

  it("counts an item at the top exactly", () => {
    expect(currentOutlineItem(OUTLINE, { page: 2, y: 0.5 })).toBe("ch1.1");
  });

  it("skips items that go nowhere", () => {
    expect(currentOutlineItem(OUTLINE, { page: 5, y: 0 })).toBe("ch1.2");
  });
});
