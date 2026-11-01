// Conformance profiles: `profiles` in the builder's options turns each
// machine-checkable PDF/UA-1 and PDF/A rule the document breaks into a coded `LaminaError` at
// `finish()`, with the same codes and data as the Rust builder; `attach` adds associated files.
import { Builder, type BuilderOptions, Font, init, isLaminaError, type Surface } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read, thrown } from "./helpers.js";

beforeAll(() => init({ text: true }));

const latin1 = new TextDecoder("latin1");

/** A one-page document with Noto Sans, a title and a language, `draw` on its page. */
function build(options: BuilderOptions, draw: (s: Surface, font: Font) => void, meta = true): Builder {
  const b = new Builder({ id: "00112233445566778899aabbccddeeff", ...options });
  if (meta) b.metadata({ title: "A report" }).lang("en-GB");
  const font = Font.fromBytes(read("fixtures/fonts/NotoSans-Regular.ttf"));
  b.font(font);
  const page = b.page("A4");
  draw(page.surface(), font);
  page.finish();
  return b;
}

const paragraph = (s: Surface, font: Font) => {
  s.text("Hello", { font, size: 12, at: [56, 80], tag: "P" });
};

/** The error `finish` throws. */
function finishError(b: Builder): unknown {
  return thrown(() => b.finish());
}

describe("PDF/UA-1", () => {
  const ua: BuilderOptions = { profiles: ["PDF/UA-1"] };

  it("builds a document that meets every rule", () => {
    const bytes = build(ua, paragraph).finish().save();
    expect(latin1.decode(bytes)).toContain("<pdfuaid:part>1</pdfuaid:part>");
  });

  it("fails a document without a title with missing-title", () => {
    const b = new Builder(ua);
    b.lang("en-GB");
    expect(isLaminaError(finishError(b), "missing-title")).toBe(true);
  });

  it("fails a document without a language with missing-lang", () => {
    const b = new Builder(ua);
    b.metadata({ title: "A report" });
    expect(isLaminaError(finishError(b), "missing-lang")).toBe(true);
  });

  it("fails a figure without alternate text with missing-alt-text", () => {
    const b = build(ua, (s) => s.tag("Figure", (s) => s.path((p) => p.rect([56, 56, 10, 10])).fill()));
    const e = finishError(b);
    expect(isLaminaError(e, "missing-alt-text") && e.data).toEqual({ page: 0 });
  });

  it("fails an untagged path with untagged-content", () => {
    const b = build(ua, (s, font) => {
      paragraph(s, font);
      s.path((p) => p.rect([56, 100, 100, 1])).fill();
    });
    const e = finishError(b);
    expect(isLaminaError(e, "untagged-content") && e.data).toEqual({ page: 0, content: "path", count: 1 });
  });

  it("fails a skipped heading level with heading-level-skipped", () => {
    const b = build(ua, (s, font) => {
      s.text("Title", { font, size: 20, at: [56, 80], tag: "H1" });
      s.text("Detail", { font, size: 12, at: [56, 120], tag: "H3" });
    });
    const e = finishError(b);
    expect(isLaminaError(e, "heading-level-skipped") && e.data).toEqual({ page: 0, level: 3, previous: 1 });
  });

  it("fails a cell outside a row with invalid-structure-nesting", () => {
    const b = build(ua, (s, font) =>
      s.tag("Table", (s) => s.text("Cell", { font, size: 12, at: [56, 80], tag: "TD" })),
    );
    expect(isLaminaError(finishError(b), "invalid-structure-nesting")).toBe(true);
  });

  it("fails a link inside an artifact with link-without-structure", () => {
    const b = build(ua, (s, font) => {
      paragraph(s, font);
      s.artifact("layout", (s) => s.link([56, 200, 50, 10], { type: "uri", uri: "https://example.com" }, "Example"));
    });
    expect(isLaminaError(finishError(b), "link-without-structure")).toBe(true);
  });

  it("fails a link without a description with link-without-contents", () => {
    const b = build(ua, (s, font) => {
      paragraph(s, font);
      s.tag("P", (s) => s.link([56, 200, 50, 10], { type: "uri", uri: "https://example.com" }));
    });
    expect(isLaminaError(finishError(b), "link-without-contents")).toBe(true);
  });

  it("fails a standard 14 font with font-not-embedded", () => {
    const b = build(ua, (s) => s.text("Hello", { font: Font.standard("Helvetica"), size: 12, at: [56, 80], tag: "P" }));
    const e = finishError(b);
    expect(isLaminaError(e, "font-not-embedded") && e.data).toEqual({ profile: "PDF/UA-1", font: "Helvetica" });
  });
});

describe("PDF/A", () => {
  it("writes the output intent and the identification", () => {
    const text = latin1.decode(
      build({ profiles: ["PDF/A-2u"] }, paragraph, false)
        .finish()
        .save({ xref: "classic" }),
    );
    expect([text.includes("/GTS_PDFA1"), text.includes("<pdfaid:conformance>U</pdfaid:conformance>")]).toEqual([
      true,
      true,
    ]);
  });

  it("fails a DeviceCMYK colour with device-color-without-output-intent", () => {
    const b = build({ profiles: ["PDF/A-2b"] }, (s) =>
      s
        .fillColor([0, 0.5, 1, 0])
        .path((p) => p.rect([56, 56, 9, 9]))
        .fill(),
    );
    expect(isLaminaError(finishError(b), "device-color-without-output-intent")).toBe(true);
  });

  it("fails an attachment under PDF/A-2b with embedded-file-forbidden", () => {
    const b = build({ profiles: ["PDF/A-2b"] }, paragraph);
    b.attach("data.csv", new TextEncoder().encode("a\n"), "text/csv");
    const e = finishError(b);
    expect(isLaminaError(e, "embedded-file-forbidden") && e.data).toEqual({ profile: "PDF/A-2b" });
  });

  it("fails PDF 2.0 with profile-version-conflict", () => {
    const b = build({ profiles: ["PDF/A-3b"], version: "2.0" }, paragraph);
    expect(isLaminaError(finishError(b), "profile-version-conflict")).toBe(true);
  });

  it("accepts an attachment under PDF/A-3b, associated with the document", () => {
    const b = build({ profiles: ["PDF/A-3b"] }, paragraph);
    b.attach("data.csv", new TextEncoder().encode("a\n"), "text/csv", { relationship: "data" });
    const text = latin1.decode(b.finish().save({ xref: "classic" }));
    expect([text.includes("/AFRelationship /Data"), text.includes("/EmbeddedFiles")]).toEqual([true, true]);
  });

  it("refuses an unknown profile name", () => {
    expect(
      isLaminaError(
        thrown(() => new Builder({ profiles: ["PDF/A-1b" as never] })),
        "invalid-argument",
      ),
    ).toBe(true);
  });
});
