// doc.pageAnnotations() and the updateAnnot and deleteAnnot ops over fixtures/annotations/ (written
// by tools/fixtures/annotations.py), as crates/laminapdf/tests/annotations.rs reads them.
import { type AddAnnot, Document, init, loadExtract } from "@laminapdf/sdk";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { read } from "./helpers.js";

beforeAll(async () => {
  await init();
  await loadExtract();
});

describe("pageAnnotations", () => {
  it("gives each annotation in page space with its geometry", () => {
    using doc = Document.open(read("fixtures/annotations/every-subtype.pdf"));
    const highlight = doc.pageAnnotations(0).find((a) => a.id === "a10.0");
    expect([highlight?.subtype, highlight?.rect, highlight?.geometry]).toEqual([
      "Highlight",
      [20, 25, 120, 40],
      { kind: "quads", quads: [20, 40, 120, 40, 120, 25, 20, 25] },
    ]);
  });

  it("names a note's popup and a group's primary", () => {
    using doc = Document.open(read("fixtures/annotations/threads.pdf"));
    const annots = doc.pageAnnotations(0);
    const note = annots.find((a) => a.id === "a10.0");
    const caret = annots.find((a) => a.id === "a17.0");
    expect([note?.popup, caret?.inReplyTo, caret?.grouped]).toEqual(["a11.0", "a16.0", true]);
  });
});

describe("textBox", () => {
  it("reads a callout back where it was put, with the spec it was written from", () => {
    using doc = Document.open(read("fixtures/pages/boxes.pdf"));
    const op = doc.calloutAt(
      0,
      [300, 400, 420, 440],
      [
        [220, 480],
        [300, 420],
      ],
      [[{ text: "Why " }, { text: "now", bold: true }]],
    );
    const [id = ""] = doc.apply([{ op: "addAnnot", args: op }]).createdAnnotations;
    const box = doc.textBox(id);
    // The edit's spec is tagged with its kind; the box read back is only ever a text box.
    const { type: _, ...written } = op.spec;
    expect([box?.page, box?.line, box?.spec, box?.simplified]).toEqual([
      0,
      [
        [220, 480],
        [300, 420],
      ],
      written,
      false,
    ]);
  });

  it("is undefined for an annotation of another kind", () => {
    using doc = Document.open(read("fixtures/annotations/every-subtype.pdf"));
    expect(doc.textBox("a10.0")).toBeUndefined();
  });
});

describe("the annotation ops", () => {
  it("moves an annotation and deletes one with what goes with it", () => {
    using doc = Document.open(read("fixtures/annotations/threads.pdf"));
    doc.apply([{ op: "updateAnnot", args: { id: "a16.0", translate: { dx: 10, dy: 0 } } }]);
    const edited = doc.apply([{ op: "deleteAnnot", args: { id: "a10.0" } }]);
    const strike = doc.pageAnnotations(0).find((a) => a.id === "a16.0");
    expect([strike?.rect, [...(edited.alsoDeleted ?? [])].sort()]).toEqual([
      [30, 185, 130, 200],
      ["a11.0", "a12.0", "a13.0", "a20.0"],
    ]);
  });
});

describe("appearance streams", () => {
  const RED: [number, number, number] = [0.85, 0.1, 0.1];

  /** One mark of every subtype on page `page`, in page space, which every turn of the page shows the same way. */
  function everyMark(doc: Document, page: number): AddAnnot[] {
    const marks = [
      doc.markText(page, "highlight", [[60, 60, 160, 76]]),
      doc.markText(page, "underline", [[60, 90, 160, 106]]),
      doc.markText(page, "strikeOut", [[60, 120, 160, 136]]),
      doc.markText(page, "squiggly", [[60, 150, 160, 166]]),
      doc.noteAt(page, [220, 60], "A note"),
      doc.caretAt(page, [220, 100, 232, 116]),
      doc.inkAt(
        page,
        [
          [
            [60, 200],
            [90, 230],
            [120, 200],
            [150, 230],
          ],
        ],
        2,
      ),
      doc.shapeAt(page, "square", [220, 200, 300, 260]),
      doc.shapeAt(page, "circle", [320, 200, 400, 260]),
      doc.lineAt(page, [60, 300], [160, 360], ["circle", "closedArrow"]),
      doc.polyAt(page, "polygon", [
        [220, 300],
        [300, 300],
        [260, 360],
      ]),
      doc.polyAt(page, "polyLine", [
        [320, 300],
        [360, 360],
        [400, 300],
      ]),
      doc.textBoxAt(page, [60, 400, 200, 440], "Check the totals"),
      doc.calloutAt(
        page,
        [300, 400, 420, 440],
        [
          [220, 480],
          [260, 420],
          [300, 420],
        ],
        "Why?",
      ),
    ];
    return marks.map((mark) => ({ ...mark, common: { ...mark.common, color: RED } }));
  }

  /** What a mark must keep: everything a reader sees of it. */
  function marksOf(doc: Document, ids: readonly string[]) {
    const all = doc.pages.flatMap((_, page) => doc.pageAnnotations(page));
    return ids.map((id) => {
      const mark = all.find((a) => a.id === id);
      return {
        subtype: mark?.subtype,
        color: mark?.color,
        rect: mark?.rect,
        contents: mark?.contents,
        hasAppearance: mark?.hasAppearance,
      };
    });
  }

  /** Every annotation that isn't one of the marks or their pop-ups, and every page's size and turn. */
  function everythingElse(doc: Document, ids: readonly string[]) {
    const ours = new Set(ids);
    return doc.pages.map((page, index) => ({
      size: [page.width, page.height, page.rotation],
      others: doc.pageAnnotations(index).filter((a) => !ours.has(a.id) && !ours.has(a.parent ?? "")),
    }));
  }

  /**
   * Where a move by (+1, +1) in user space (one unit right, one up) goes in page space (y down,
   * the page as shown): turned with the page, and `userUnit` points long.
   */
  function shownShift(rotation: number, unit: number): number[] {
    const [dx, dy] = { 0: [1, -1], 90: [1, 1], 180: [-1, 1], 270: [-1, -1] }[rotation] ?? [Number.NaN, Number.NaN];
    return [dx, dy, dx, dy].map((v) => (v ?? Number.NaN) * unit);
  }

  const docs: Document[] = [];
  let ids: string[];
  let perPage: number[];
  let turns: number[];
  let units: number[];
  let made: ReturnType<typeof marksOf>;
  let before: ReturnType<typeof everythingElse>;
  let kept: ReturnType<typeof marksOf>;
  let keptElse: ReturnType<typeof everythingElse>;
  let moved: Document;

  beforeAll(() => {
    const doc = Document.open(read("fixtures/pages/boxes.pdf"));
    docs.push(doc);
    turns = doc.pages.map((p) => p.rotation);
    units = doc.pages.map((_, page) => doc.pageBoxes(page).userUnit);
    const marks = doc.pages.map((_, page) => everyMark(doc, page));
    perPage = marks.map((m) => m.length);
    before = everythingElse(doc, []);
    ids = doc.apply(marks.flat().map((args) => ({ op: "addAnnot", args }))).createdAnnotations;
    made = marksOf(doc, ids);
    const saved = Document.open(doc.save({ mode: "incremental" }));
    docs.push(saved);
    kept = marksOf(saved, ids);
    keptElse = everythingElse(saved, ids);
    saved.apply(ids.map((id) => ({ op: "updateAnnot", args: { id, translate: { dx: 1, dy: 1 } } })));
    moved = Document.open(saved.save({ mode: "incremental" }));
    docs.push(moved);
  });

  afterAll(() => {
    for (const doc of docs) doc[Symbol.dispose]();
  });

  it("puts the marks on pages at every turn", () => {
    expect([turns, ids.length]).toEqual([[0, 90, 180, 270], perPage.reduce((a, b) => a + b, 0)]);
  });

  it("[cap D.16 pdf round-trip] keeps every mark's subtype, colour, rectangle, contents and appearance through a save", () => {
    expect(kept).toEqual(made);
  });

  it("draws every mark", () => {
    expect(made.every((m) => m.hasAppearance)).toBe(true);
  });

  it("keeps every mark drawn after a move and a second save", () => {
    expect(marksOf(moved, ids).every((m) => m.hasAppearance)).toBe(true);
  });

  it("moves each mark by exactly one unit right and one up, in user space", () => {
    const after = marksOf(moved, ids);
    const shifts = made.map((m, i) =>
      (after[i]?.rect ?? []).map((v, k) => Math.round((v - (m.rect?.[k] ?? 0)) * 1000) / 1000),
    );
    const expected = perPage.flatMap((count, page) =>
      Array.from({ length: count }, () => shownShift(turns[page] ?? 0, units[page] ?? 1)),
    );
    expect(shifts).toEqual(expected);
  });

  it("[cap D.16 pdf round-trip] changes nothing else: the pages, their sizes and turns, and the other annotations", () => {
    expect([keptElse, everythingElse(moved, ids)]).toEqual([before, before]);
  });
});
