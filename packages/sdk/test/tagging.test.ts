// Tagged PDF (todo B8a): `tag`, `artifact`, `alt` and the `tag` option of text and text boxes.
// The TS twin of laminapdf-build's tagged report is byte-identical to its Rust golden
// (fixtures/golden/tagged-report.pdf, whose structure crates/laminapdf-build/tests/tagging.rs
// checks with laminapdf-model, pdf.js and PDFium).
import { Builder, Document, Font, Image, init, isLaminaError, type Surface } from "@laminapdf/sdk";
import { beforeAll, describe, expect, it } from "vitest";
import { read, thrown } from "./helpers.js";

beforeAll(() => init({ text: true }));

const latin1 = new TextDecoder("latin1");

/** The tagged report of crates/laminapdf-build/tests/common/tagged.rs, call for call. */
function taggedReport(): Uint8Array {
  const b = new Builder({
    id: new TextEncoder().encode("lamina-build-B8a"),
    producer: "lamina-build golden",
    creationDate: "D:20261030120000Z",
    compress: false,
    tagged: true,
  });
  const noto = Font.fromBytes(read("fixtures/fonts/NotoSans-Regular.ttf"));
  b.font(noto);
  const line = (s: Surface, text: string, size: number, x: number, y: number) =>
    s.text(text, { font: noto, size, at: [x, y] });
  const pagination = (s: Surface, n: number) =>
    s.artifact("pagination", (s) => {
      line(s, "Acme Widgets · Quarterly report", 9, 56, 40);
      line(s, `Page ${n}`, 9, 520, 810);
    });

  const first = b.page("A4");
  const s = first.surface();
  pagination(s, 1);
  s.text("Quarterly report", { font: noto, size: 24, at: [56, 96], tag: "H1" });
  s.textBox(
    "Sales rose in every region this quarter. The chart and the table below show units and revenue per quarter.",
    { font: noto, size: 11, box: [56, 110, 483, 40], tag: "P" },
  );
  const blue = [30, 120, 200, 255];
  s.image(Image.rgba(4, 1, new Uint8Array([...blue, ...blue, ...blue, ...blue])), [56, 160, 200, 80], ALT);
  s.text("Figure 1: sales by quarter", { font: noto, size: 9, at: [56, 254], tag: "Caption" });
  s.artifact("layout", (s) =>
    s
      .strokeColor([0.6])
      .path((p) => p.moveTo(56, 266).lineTo(539, 266))
      .stroke(),
  );
  s.tag("Table", (s) => {
    s.tag("TR", (s) => {
      ["Quarter", "Units", "Sales"].forEach((h, i) => {
        s.text(h, { font: noto, size: 11, at: [56 + 120 * i, 290], tag: { type: "TH", scope: "column" } });
      });
    });
    [
      ["Q1", "120", "€ 1,200"],
      ["Q2", "310", "€ 3,100"],
    ].forEach((row, r) => {
      s.tag("TR", (s) => {
        row.forEach((cell, i) => {
          s.text(cell, { font: noto, size: 11, at: [56 + 120 * i, 310 + 18 * r], tag: "TD" });
        });
      });
    });
  });
  s.tag("L", (s) => {
    ["Widgets sold out in May.", "Gadgets ship in the autumn."].forEach((item, i) => {
      const y = 370 + 18 * i;
      s.tag("LI", (s) => {
        s.text("•", { font: noto, size: 11, at: [56, y], tag: "Lbl" });
        s.text(item, { font: noto, size: 11, at: [70, y], tag: "LBody" });
      });
    });
  });
  s.text("Read the full report at example.com/report", {
    font: noto,
    size: 11,
    at: [56, 420],
    fill: [0.12, 0.47, 0.78],
    link: { target: { type: "uri", uri: "https://example.com/report" }, contents: "The full report on example.com" },
    tag: "P",
  });
  s.tag("P", (s) => {
    line(s, "Our motto:", 11, 56, 440);
    s.text("joie de vivre", { font: noto, size: 11, at: [112, 440], tag: { type: "Span", lang: "fr" } });
  });
  s.text("Total sales: € 4,300", { font: noto, size: 14, at: [56, 470], tag: { type: "Total", role: "P" } });
  first.finish();

  const second = b.page("A4");
  const t = second.surface();
  t.artifact("background", (s) =>
    s
      .fillColor([0.97])
      .path((p) => p.rect([0, 0, 595.28, 841.89]))
      .fill()
      .fillColor([0]),
  );
  pagination(t, 2);
  t.tag("Sect", (s) => {
    s.text("Notes", { font: noto, size: 18, at: [56, 96], tag: "H2" });
    s.text("Figures are unaudited.", { font: noto, size: 11, at: [56, 120], tag: "P" });
    s.text("“Best quarter yet.”", { font: noto, size: 11, at: [56, 140], tag: "Quote" });
    s.text("1. Excluding returns.", { font: noto, size: 9, at: [56, 160], tag: "Note" });
    s.text("cargo xtask golden --update", { font: noto, size: 10, at: [56, 180], tag: "Code" });
  });
  t.tag("Link", (s) => {
    line(s, "Back to page 1", 11, 56, 210);
    s.link([56, 198, 80, 16], { type: "page", index: 0 }, "Back to the first page");
  });
  second.finish();
  b.metadata({ title: "Quarterly report" }).lang("en-GB");
  return b.finish().save({ xref: "classic" });
}

const ALT = "A bar chart of sales by quarter, rising from 120 to 310";

describe("tagging", () => {
  it("builds the tagged report byte for byte as the Rust builder does", () => {
    expect(taggedReport()).toEqual(read("fixtures/golden/tagged-report.pdf"));
  });

  it("marks a document drawn with tags", () => {
    const b = new Builder({ compress: false });
    const page = b.page("A6");
    page.surface().tag("P", (s) => s.path((p) => p.rect([10, 10, 10, 10])).fill());
    page.finish();
    const text = latin1.decode(b.finish().save({ xref: "classic" }));
    expect([text.includes("/StructTreeRoot"), text.includes("/Marked true")]).toEqual([true, true]);
  });

  it("ends the element when the drawing callback throws", () => {
    const b = new Builder();
    const page = b.page("A6");
    const s = page.surface();
    expect(() =>
      s.tag("Figure", () => {
        throw new Error("drawing failed");
      }),
    ).toThrow("drawing failed");
    s.tag({ type: "Figure", alt: "a square" }, (s) => s.path((p) => p.rect([10, 10, 10, 10])).fill());
    page.finish();
    const doc = Document.open(b.finish().save());
    expect(doc.pageCount).toBe(1);
  });

  it("reports the drawing callback's error, not ending the element on a finished page", () => {
    const b = new Builder();
    const page = b.page("A6");
    const s = page.surface();
    expect(() =>
      s.tag("P", () => {
        page.finish(); // ending the element afterwards would throw builder-finished
        throw new Error("drawing failed");
      }),
    ).toThrow("drawing failed");
  });

  it("rejects an unknown type without a role, suggesting the closest", () => {
    const b = new Builder();
    const e = thrown(() =>
      b
        .page("A6")
        .surface()
        .tag("H7" as never, () => {}),
    );
    expect(isLaminaError(e, "invalid-argument") && e.didYouMean).toContain("H1");
  });

  it("rejects an unknown artifact kind", () => {
    const b = new Builder();
    const e = thrown(() =>
      b
        .page("A6")
        .surface()
        .artifact("Layout" as never, () => {}),
    );
    expect(isLaminaError(e, "invalid-argument") && e.didYouMean).toContain("layout");
  });
});
