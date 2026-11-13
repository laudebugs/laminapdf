// doc.pageAnnotations() and the updateAnnot and deleteAnnot ops over fixtures/annotations/ (written
// by tools/fixtures/annotations.py), as crates/laminapdf/tests/annotations.rs reads them.
import { Document, init, loadExtract } from "@laminapdf/sdk";
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
