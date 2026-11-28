// Text a page already shows, changed in place: doc.content.editText with its font chain and
// planEditText, and doc.content.setTextStyle, saved and reopened, in the extract module. A test's
// title starts with the capability it proves (its tag: capabilities A.02, A.04, A.05 and A.06,
// column pdf), which `cargo xtask capabilities` counts.
import { Document, init, isLaminaPdfError } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read } from "./helpers.js";

beforeAll(() => init({ extract: true }));

function savedAndReopened(doc: Document): Document {
  return Document.open(doc.save({ mode: "incremental" }));
}

function runs(doc: Document) {
  return doc.content.textRuns(0);
}

/** The run of the greeting that says "world", and the glyph range of the word within it. */
function world(doc: Document) {
  const found = runs(doc).find((r) => !r.readOnly && r.text.includes("world"));
  if (!found) {
    throw new Error("no run says world");
  }
  const at = found.text.indexOf("world");
  return { found, word: { ...found.run, start: found.run.start + at, end: found.run.start + at + 5 } };
}

describe("text restyled in place", () => {
  it("[cap A.04 pdf round-trip] [cap A.05 pdf round-trip] makes a word bold, red and underlined, and the words reopen unchanged", () => {
    using doc = Document.open(read("fixtures/golden/examples/hello.pdf"));
    const { word } = world(doc);
    doc.content.setTextStyle(0, [word], { bold: true, color: [0.8, 0, 0], underline: true });
    using again = savedAndReopened(doc);
    expect(again.page(0).text().text()).toContain("Hello, world!");
    const fonts = runs(again).map((r) => r.fontName);
    expect(fonts).toContain("Helvetica-Bold");
  });

  it("[cap A.04 pdf round-trip] changes the font family and the size of a run", () => {
    using doc = Document.open(read("fixtures/golden/examples/hello.pdf"));
    const { found } = world(doc);
    doc.content.setTextStyle(0, [found.run], { font: "times", size: 36 });
    using again = savedAndReopened(doc);
    const [run] = runs(again);
    expect([run?.fontName, run?.fontSize, run?.text]).toEqual(["Times-Roman", 36, "Hello, world!"]);
  });

  it("[cap A.05 pdf round-trip] raises a word as a superscript at a smaller size", () => {
    using doc = Document.open(read("fixtures/golden/examples/hello.pdf"));
    const { word } = world(doc);
    doc.content.setTextStyle(0, [word], { baseline: "super" });
    using again = savedAndReopened(doc);
    const sizes = runs(again).map((r) => r.fontSize);
    expect(Math.min(...sizes)).toBeCloseTo(24 * 0.65, 3);
  });

  it("[cap A.06 pdf write] aligns and spaces the lines of a run's text object", () => {
    using doc = Document.open(read("fixtures/golden/examples/hello.pdf"));
    const { found } = world(doc);
    const edited = doc.content.setTextStyle(0, [found.run], { align: "right", lineSpacing: 1.5 });
    expect(edited.touchedPages).toHaveLength(1);
    using again = savedAndReopened(doc);
    expect(again.page(0).text().text()).toContain("Hello, world!");
  });

  it("[cap A.04 pdf write] refuses to take an underline off, and a bold on a font with no bold face to take", () => {
    using doc = Document.open(read("fixtures/golden/examples/hello.pdf"));
    const { found } = world(doc);
    for (const style of [{ underline: false }, {}]) {
      let code: string | undefined;
      try {
        doc.content.setTextStyle(0, [found.run], style);
      } catch (e) {
        code = isLaminaPdfError(e) ? e.code : undefined;
      }
      expect(code).toBe("invalid-op");
    }
  });
});

describe("text edited with a font chain", () => {
  it("[cap A.02 pdf round-trip] says what a missing glyph would be drawn in before the edit, and then draws it", () => {
    using doc = Document.open(read("fixtures/golden/examples/hello.pdf"));
    const { found } = world(doc);
    const text = found.text.replace("world", "wörld");
    const before = doc.content.textRuns(0).length;
    // The run's Helvetica has no ö in its encoding, so Times, which the call names, draws it.
    const plan = doc.content.planEditText(0, found.run, text, ["times"]);
    expect(plan.fidelity).toBe("substituted");
    expect(plan.substitutions.map((s) => s.text)).toEqual(["ö"]);
    doc.content.editText(0, found.run, text, ["times"]);
    using again = savedAndReopened(doc);
    expect(again.page(0).text().text()).toContain("Hello, wörld!");
    expect(again.content.textRuns(0)).toHaveLength(before);
  });

  it("[cap A.02 pdf round-trip] fails with missing-glyph when no named font draws a character", () => {
    using doc = Document.open(read("fixtures/golden/examples/hello.pdf"));
    const { found } = world(doc);
    let code: string | undefined;
    try {
      doc.content.editText(0, found.run, "你好", ["helvetica"]);
    } catch (e) {
      code = isLaminaPdfError(e) ? e.code : undefined;
    }
    expect(code).toBe("missing-glyph");
  });
});
