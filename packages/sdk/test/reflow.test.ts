// doc.reflow(): reader mode's blocks for fixtures/reflow/mixed.pdf (as crates/laminapdf/tests/reflow.rs
// reads them), a refusal with its reason, and the base module's refusal (the text module refuses
// it the same way: reader mode is in the extract module only).
import { Document, init, isLaminaPdfError, loadExtract } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read, thrown } from "./helpers.js";

beforeAll(() => init());

describe("reflow in the base module", () => {
  it("throws extract-module-not-loaded", () => {
    using doc = Document.open(read("fixtures/reflow/mixed.pdf"));
    expect(
      isLaminaPdfError(
        thrown(() => doc.reflow()),
        "extract-module-not-loaded",
      ),
    ).toBe(true);
  });
});

describe("reflow", () => {
  beforeAll(() => loadExtract());

  it("lists the blocks in the tree's order, the figure, formula and table as crops with their alt", () => {
    using doc = Document.open(read("fixtures/reflow/mixed.pdf"));
    const reflow = doc.reflow();
    expect(reflow.available).toBe(true);
    expect(reflow.blocks.map((b) => b.kind)).toEqual([
      "heading",
      "paragraph",
      "figure",
      "caption",
      "formula",
      "quote",
      "list-item",
      "list-item",
      "table",
    ]);
    const figure = reflow.blocks[2];
    expect(figure).toMatchObject({ kind: "figure", text: "", alt: "Four coloured squares in a grid", page: 0 });
    expect(figure?.bbox).toEqual([56, 180, 160, 110]);
  });

  it("refuses a reversed tree for its order", () => {
    using doc = Document.open(read("fixtures/reflow/two-columns-tree-reversed.pdf"));
    const reflow = doc.reflow();
    expect({ available: reflow.available, reason: reflow.reason, blocks: reflow.blocks }).toEqual({
      available: false,
      reason: "order",
      blocks: [],
    });
    expect(reflow.checks.agreement).toBe(-1);
  });
});
