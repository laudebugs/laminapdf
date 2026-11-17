// doc.pageAnnotations() and the updateAnnot and deleteAnnot ops over fixtures/annotations/ (written
// by tools/fixtures/annotations.py), as crates/laminapdf/tests/annotations.rs reads them.
import { type AddAnnot, Document, init, loadExtract } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
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
  /** One mark of every subtype on page `page`, in page space, which every turn of the page shows the same way. */
  function everyMark(doc: Document, page: number): AddAnnot[] {
    return [
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
  }

  /** The marks `ids` as reopened from `doc`'s incremental save: rectangle and whether drawn. */
  function saved(doc: Document, ids: readonly string[]): { doc: Document; marks: [number[] | undefined, boolean][] } {
    const reopened = Document.open(doc.save({ mode: "incremental" }));
    const all = reopened.pages.flatMap((_, i) => reopened.pageAnnotations(i));
    const marks = ids.map((id): [number[] | undefined, boolean] => {
      const mark = all.find((a) => a.id === id);
      return [mark?.rect, mark?.hasAppearance ?? false];
    });
    return { doc: reopened, marks };
  }

  it("[cap D.16 pdf round-trip] draws every mark on pages at every turn, and keeps each drawing through a save and a move", () => {
    using doc = Document.open(read("fixtures/pages/boxes.pdf"));
    const turns = doc.pages.map((p) => p.rotation);
    const perPage = doc.pages.map((_, page) => everyMark(doc, page));
    const ids = doc.apply(perPage.flat().map((args) => ({ op: "addAnnot", args }))).createdAnnotations;
    const first = saved(doc, ids);
    using reopened = first.doc;
    // A move is in user space: one unit is `userUnit` points of page space.
    reopened.apply(ids.map((id) => ({ op: "updateAnnot", args: { id, translate: { dx: 1, dy: 1 } } })));
    const second = saved(reopened, ids);
    using moved = second.doc;
    const expected = perPage.flatMap((marks, page) => {
      const unit = doc.pageBoxes(page).userUnit;
      return marks.map(() => [unit, unit, unit, unit]);
    });
    const shifts = second.marks.map(([rect], i) => {
      const before = first.marks[i]?.[0] ?? [];
      return (rect ?? []).map((v, k) => Math.round(Math.abs(v - (before[k] ?? 0)) * 100) / 100);
    });
    expect([turns, ids.length, moved.pages.length]).toEqual([[0, 90, 180, 270], 56, 4]);
    expect(first.marks.every(([rect, drawn]) => rect !== undefined && drawn)).toBe(true);
    expect(second.marks.every(([, drawn]) => drawn)).toBe(true);
    expect(shifts).toEqual(expected);
  });
});
