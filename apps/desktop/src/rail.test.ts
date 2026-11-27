// The desktop sidebar's outline reader: read once, with the pages' labels, and gone to by item.
import { describe, expect, it, vi } from "vitest";
import { DesktopOutline } from "./rail.tsx";

const NODES = [
  {
    id: "o1.0",
    title: "Preface",
    open: true,
    bold: false,
    italic: false,
    target: { page: 1, view: { kind: "fit" } },
    children: [
      {
        id: "o2.0",
        title: "Thanks",
        open: false,
        bold: false,
        italic: false,
        target: { page: 2, view: { kind: "at", y: 200 } },
        children: [],
      },
    ],
  },
];

function reader(outline = vi.fn().mockResolvedValue(NODES)) {
  const jumpToDestination = vi.fn();
  const pages = [{ height: 400 }, { height: 400, label: "i" }, { height: 400, label: "ii" }];
  const source = new DesktopOutline({ outline, document: { pages } } as never, { jumpToDestination });
  return { source, outline, jumpToDestination };
}

describe("the desktop's outline reader", () => {
  it("should read nothing until the Outline tab asks", () => {
    const { source, outline } = reader();
    expect(source.getState().outline).toBeUndefined();
    expect(outline).not.toHaveBeenCalled();
  });

  it("should read the outline once, with each item's page label and place", async () => {
    const { source, outline } = reader();
    await Promise.all([source.loadOutline(), source.loadOutline()]);
    await source.loadOutline();
    expect(outline).toHaveBeenCalledTimes(1);
    const read = source.getState().outline;
    if (typeof read !== "object") throw new Error("the outline was not read");
    expect(read.items.map((i) => [i.id, i.page, i.children.map((c) => [c.page, c.at])])).toEqual([
      ["o1.0", "i", [["ii", { page: 2, y: 0.5 }]]],
    ]);
  });

  it("should tell its listeners when it starts and finishes reading", async () => {
    const { source } = reader();
    const seen: string[] = [];
    source.subscribe(() => {
      const { outline } = source.getState();
      seen.push(typeof outline === "object" ? "read" : String(outline));
    });
    await source.loadOutline();
    expect(seen).toEqual(["loading", "read"]);
  });

  it("should say the outline failed when the engine can't read it", async () => {
    const { source } = reader(vi.fn().mockRejectedValue(new Error("boom")));
    await source.loadOutline();
    expect(source.getState().outline).toBe("failed");
  });

  it("should go to an item's destination in the viewer, and to none it hasn't read", async () => {
    const { source, jumpToDestination } = reader();
    await source.goToOutlineItem("o1.0");
    expect(jumpToDestination).not.toHaveBeenCalled();
    await source.loadOutline();
    await source.goToOutlineItem("o2.0");
    expect(jumpToDestination).toHaveBeenCalledWith({ page: 2, view: { kind: "at", y: 200 } });
  });
});
