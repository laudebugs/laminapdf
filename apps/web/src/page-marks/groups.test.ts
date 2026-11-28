import type { PageMarkInfo } from "@laminapdf/sdk";
import { describe, expect, it } from "vitest";
import { editOf, groupMarks, kindOf, managedOf } from "./groups.ts";

const LABELS = ["1", "2", "3", "4"];
const info = (id: string, mark: unknown): PageMarkInfo => ({ id, mark, hideable: false }) as unknown as PageMarkInfo;
const watermark = info("w", {
  watermark: {
    mark: { text: { text: "DRAFT", size: 80, fonts: ["helvetica"], color: [155 / 255, 35 / 255, 53 / 255] } },
    opacity: 0.4,
    angle: 30,
    hideable: true,
  },
});
const footer = info("f", {
  headerFooter: { slots: [null, null, null, null, "{page} of {pages}", null], size: 9, margins: { top: 20 } },
});
const bates = (start: number) => info("b", { bates: { prefix: "NHT-", start, slot: "bottomRight", size: 9 } });
const CONTEXT = { fontIdOf: (name: string) => (name === "helvetica" ? name : undefined), imageOf: () => undefined };

const PAGES = [
  [watermark, bates(10)],
  [watermark, footer, bates(11)],
  [watermark, footer, bates(12)],
  [watermark, bates(13)],
];

describe("the marks of a document", () => {
  it("are gathered by ID with the pages each is on", () => {
    const groups = groupMarks(PAGES);
    expect(groups.map((g) => [g.id, g.pages])).toEqual([
      ["w", [0, 1, 2, 3]],
      ["b", [0, 1, 2, 3]],
      ["f", [1, 2]],
    ]);
    expect(kindOf(groups[2]?.first as never)).toBe("headerFooter");
  });

  it("are listed with what they say and where", () => {
    const [w, b, f] = groupMarks(PAGES).map((g) => managedOf(g, LABELS));
    expect(w).toEqual({ id: "w", kind: "watermark", summary: "DRAFT", pages: "1–4" });
    expect(b?.summary).toBe("NHT-000010 … NHT-000013");
    expect(f).toEqual({ id: "f", kind: "headerFooter", summary: "[Page] of [Pages]", pages: "2–3" });
  });
});

describe("editing a mark", () => {
  it("reads a watermark's values back", () => {
    const edit = editOf(groupMarks(PAGES)[0] as never, CONTEXT);
    expect(edit).toMatchObject({
      tab: "watermark",
      pages: [0, 1, 2, 3],
      values: { text: "DRAFT", size: 80, colour: "red", opacity: 40, angle: 30, layer: "front", hideable: true },
    });
  });

  it("reads a header and footer's slots with their tokens shown", () => {
    const edit = editOf(groupMarks(PAGES)[2] as never, CONTEXT);
    expect(edit).toMatchObject({ tab: "headerFooter", pages: [1, 2], values: { top: 20, bottom: 36, size: 9 } });
    expect((edit as unknown as { values: { slots: string[] } }).values.slots[4]).toBe("[Page] of [Pages]");
  });

  it("numbers Bates from the first page's own number", () => {
    const edit = editOf(groupMarks(PAGES)[1] as never, CONTEXT);
    expect(edit).toMatchObject({ tab: "bates", values: { prefix: "NHT-", start: 10, digits: 6 } });
  });
});
