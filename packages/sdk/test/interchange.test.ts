// doc.exportAnnotations(), doc.previewImport() and doc.importAnnotations() over
// fixtures/interchange/ (written by tools/fixtures/interchange.py) and corpus/hello.pdf, as
// crates/laminapdf/tests/interchange.rs reads them.
import { Document, init, isLaminaPdfError, loadExtract } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read } from "./helpers.js";

beforeAll(async () => {
  await init();
  await loadExtract();
});

const hello = () => Document.open(read("fixtures/corpus/hello.pdf"));
const review = read("fixtures/interchange/review.xfdf");

describe("importAnnotations", () => {
  it("[cap D.13 pdf round-trip] imports every kind of comment as one edit, and the saved file exports it again", () => {
    using doc = hello();
    const { summary, edited } = doc.importAnnotations(review);
    using saved = Document.open(doc.save());
    const again = new TextDecoder().decode(saved.exportAnnotations({ format: "xfdf", file: "hello.pdf" }));
    expect([summary.added, summary.pages, edited.changed, saved.pageAnnotations(0).length]).toEqual([
      16,
      [0],
      true,
      18,
    ]);
    expect(again).toContain('<highlight page="0" rect="72,700,180,714" name="hl"');
  });

  it("is undone in one step", () => {
    using doc = hello();
    doc.importAnnotations(review, { mode: "merge" });
    doc.undo();
    expect(doc.pageAnnotations(0)).toEqual([]);
  });

  it("previews without changing anything, and lists what it skips", () => {
    using doc = hello();
    const summary = doc.previewImport(read("fixtures/interchange/skipped.xfdf"));
    expect([
      summary.added,
      summary.warnings.map((w) => [w.code, w.kind, w.reason]),
      doc.pageAnnotations(0).length,
    ]).toEqual([
      1,
      [
        ["interchange-skipped", "Sound", "unsupported"],
        ["interchange-skipped", "Link", "unsupported"],
        ["interchange-skipped", "field", "form-field"],
        ["interchange-skipped", "field", "form-field"],
        ["interchange-skipped", "Text", "missing-page"],
        ["interchange-skipped", "Text", "missing-parent"],
      ],
      0,
    ]);
  });

  it("names the line of a malformed file and refuses a DTD", () => {
    using doc = hello();
    const failure = (name: string) => {
      try {
        doc.importAnnotations(read(`fixtures/interchange/${name}`));
      } catch (e) {
        return isLaminaPdfError(e) ? [e.code, e.data] : String(e);
      }
      return "imported";
    };
    expect([failure("malformed.xfdf"), failure("dtd.xfdf")]).toEqual([
      ["interchange-malformed", { format: "XFDF", line: 5 }],
      ["interchange-dtd", { line: 2 }],
    ]);
  });
});

describe("exportAnnotations", () => {
  it("writes the highlights of review.pdf as Markdown with their quoted text", () => {
    using doc = Document.open(read("fixtures/annotations/review.pdf"));
    const md = new TextDecoder().decode(
      doc.exportAnnotations({ format: "markdown", comments: "highlightsAndNotes", markdown: { authors: false } }),
    );
    expect(md.startsWith("## Page 1\n\n") && md.includes("\n> Quoted words\n")).toBe(true);
  });
});
