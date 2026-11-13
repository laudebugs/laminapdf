// The outline tab's items: bookmark ids, page labels, styles, places down the page, and targets by id.
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

const node = (id: string, title: string, rest: Partial<OutlineNode> = {}): OutlineNode => ({
  id,
  title,
  open: false,
  bold: false,
  italic: false,
  children: [],
  ...rest,
});

const NODES: OutlineNode[] = [
  node("o4.0", "Preface", { target: { page: 1, view: { kind: "fit" } }, bold: true, color: "#c00000" }),
  node("o5.0", "Chapter", {
    open: true,
    destinationName: "chapter",
    children: [node("o6.0", "Section", { target: { page: 2, view: { kind: "at", y: 200 } } })],
  }),
];

describe("outlineModel", () => {
  const { items, targets } = outlineModel(NODES, PAGES);

  it("ids items by their bookmark and shows their page labels", () => {
    expect(items.map((i) => [i.id, i.title, i.page])).toEqual([
      ["o4.0", "Preface", "ii"],
      ["o5.0", "Chapter", undefined],
    ]);
    expect(items[1]?.children[0]?.id).toBe("o6.0");
  });

  it("keeps each bookmark's style and the name it goes to", () => {
    expect([items[0]?.bold, items[0]?.color, items[1]?.destinationName]).toEqual([true, "#c00000", "chapter"]);
  });

  it("places an item by its destination's top", () => {
    expect(items[1]?.children[0]?.at).toEqual({ page: 2, y: 0.25 });
    expect(items[0]?.at).toEqual({ page: 1, y: 0 });
  });

  it("keeps each target by id", () => {
    expect(targets.get("o6.0")?.page).toBe(2);
    expect(targets.has("o5.0")).toBe(false);
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
