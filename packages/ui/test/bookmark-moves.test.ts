// Where a bookmark goes when it moves (O11): the keyboard's moves among siblings and between
// levels, and drops before, after and onto a row, named as the outline ops take them.
import { describe, expect, it } from "vitest";
import { dropMove, keyMove } from "../src/edit/bookmark-moves.js";
import type { OutlineTreeItem } from "../src/reader/outline-tree.js";

const item = (id: string, children: OutlineTreeItem[] = []): OutlineTreeItem => ({ id, title: id, children });

/** a, b (b1, b2), c */
const TREE = [item("a"), item("b", [item("b1"), item("b2")]), item("c")];

describe("keyMove", () => {
  it("moves among siblings", () => {
    expect(keyMove(TREE, "b", "ArrowUp")).toEqual({ item: "b", parent: null, at: { before: "a" } });
    expect(keyMove(TREE, "b1", "ArrowDown")).toEqual({ item: "b1", parent: "b", at: { after: "b2" } });
  });

  it("goes nowhere past either end", () => {
    expect(keyMove(TREE, "a", "ArrowUp")).toBeUndefined();
    expect(keyMove(TREE, "b2", "ArrowDown")).toBeUndefined();
  });

  it("nests under the sibling before and comes back out after its parent", () => {
    expect(keyMove(TREE, "c", "ArrowRight")).toEqual({ item: "c", parent: "b", at: "last" });
    expect(keyMove(TREE, "b2", "ArrowLeft")).toEqual({ item: "b2", parent: null, at: { after: "b" } });
    expect(keyMove(TREE, "a", "ArrowLeft")).toBeUndefined();
    expect(keyMove(TREE, "a", "ArrowRight")).toBeUndefined();
  });
});

describe("dropMove", () => {
  it("puts an item beside a row, at the row's level", () => {
    expect(dropMove(TREE, "c", "b1", "before")).toEqual({ item: "c", parent: "b", at: { before: "b1" } });
    expect(dropMove(TREE, "b2", "a", "after")).toEqual({ item: "b2", parent: null, at: { after: "a" } });
  });

  it("nests an item dropped onto a row as its last child", () => {
    expect(dropMove(TREE, "a", "c", "on")).toEqual({ item: "a", parent: "c", at: "last" });
  });

  it("refuses a drop into the item's own branch", () => {
    expect(dropMove(TREE, "b", "b1", "on")).toBeUndefined();
    expect(dropMove(TREE, "b", "b", "after")).toBeUndefined();
  });
});
