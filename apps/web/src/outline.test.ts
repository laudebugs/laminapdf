// The outline tab's items: path ids, page labels, places down the page, and targets by id.
import type { ViewerClient } from "@laminapdf/viewer";
import { describe, expect, it } from "vitest";
import { downThePage, outlineModel, type PageInfo } from "./outline.ts";

type OutlineNode = Awaited<ReturnType<ViewerClient["outline"]>>[number];

const page = (index: number, label?: string): PageInfo =>
  ({
    index,
    id: `p${index}.0`,
    width: 400,
    height: 800,
    rotation: 0,
    ...(label === undefined ? {} : { label }),
  }) as PageInfo;
const PAGES = [page(0, "i"), page(1, "ii"), page(2, "1")];

const NODES: OutlineNode[] = [
  { title: "Preface", open: false, target: { page: 1, view: { kind: "fit" } }, children: [] },
  {
    title: "Chapter",
    open: true,
    children: [{ title: "Section", open: false, target: { page: 2, view: { kind: "at", y: 200 } }, children: [] }],
  },
];

describe("outlineModel", () => {
  const { items, targets } = outlineModel(NODES, PAGES);

  it("ids items by their path and shows their page labels", () => {
    expect(items.map((i) => [i.id, i.title, i.page])).toEqual([
      ["0", "Preface", "ii"],
      ["1", "Chapter", undefined],
    ]);
    expect(items[1]?.children[0]?.id).toBe("1.0");
  });

  it("places an item by its destination's top", () => {
    expect(items[1]?.children[0]?.at).toEqual({ page: 2, y: 0.25 });
    expect(items[0]?.at).toEqual({ page: 1, y: 0 });
  });

  it("keeps each target by id", () => {
    expect(targets.get("1.0")?.page).toBe(2);
    expect(targets.has("1")).toBe(false);
  });

  it("numbers a page without a label", () => {
    expect(outlineModel(NODES, [page(0), page(1), page(2)]).items[0]?.page).toBe("2");
  });
});

describe("downThePage", () => {
  // A point a tenth down the page and a quarter across it, as the turned view shows it.
  it.each([
    [0, 0.25, 0.1],
    [90, 0.9, 0.25],
    [180, 0.75, 0.9],
    [270, 0.1, 0.75],
  ] as const)("undoes a view turned %i°", (rotation, x, y) => {
    expect(downThePage(x, y, rotation)).toBeCloseTo(0.1);
  });
});
