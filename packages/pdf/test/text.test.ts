// page.text() (B9c): the extract module, the shared snapshot with the Rust output, encrypted
// copies, and the error for a document opened in the base module.
import { Document, init, isExtractLoaded, isLaminaError, loadExtract, type PageText } from "@lamina/pdf";
import { beforeAll, describe, expect, it } from "vitest";
import { read, readText, thrown } from "./helpers.js";

beforeAll(() => init());

/** Rust's `f64::round`: half away from zero (JS's Math.round goes up). */
function roundHalfAway(v: number): number {
  return Math.sign(v) * Math.round(Math.abs(v));
}

/** The snapshot's rounding (crates/lamina/tests/page_text_snapshot.rs `round`). */
function round(x: number, step: number): number {
  const r = roundHalfAway(x / step) * step;
  return r === 0 ? 0 : roundHalfAway(r * 1000) / 1000;
}

/** A page's text as the snapshot writes it. */
function pageJson(text: PageText): unknown {
  return {
    lines: text.lines,
    unmapped: text.unmapped,
    runs: text.runs.map((r) => {
      const b = r.bounds;
      return {
        text: r.text,
        font: r.font,
        size: round(r.size, 0.001),
        vertical: r.vertical,
        mcid: r.mcid ?? null,
        actualText: r.actualText,
        invisible: r.invisible,
        bounds: [round(b.x, 0.01), round(b.y, 0.01), round(b.width, 0.01), round(b.height, 0.01)],
        glyphs: r.glyphRange,
      };
    }),
    glyphs: Array.from({ length: text.glyphCount }, (_, i) => {
      const g = text.glyph(i);
      return { text: g.text, quad: g.quad.flatMap((p) => [round(p.x, 0.01), round(p.y, 0.01)]) };
    }),
  };
}

interface Snapshot {
  files: Record<string, unknown[]>;
}

describe("page.text() before the extract module", () => {
  it("throws extract-module-not-loaded for a document opened in the base module", () => {
    expect(isExtractLoaded()).toBe(false);
    const doc = Document.open(read("fixtures/golden/examples/hello.pdf"));
    const e = thrown(() => doc.page(0).text());
    expect(isLaminaError(e, "extract-module-not-loaded")).toBe(true);
  });
});

describe("page.text() with the extract module", () => {
  beforeAll(() => loadExtract());

  it("extracts the hello example", () => {
    const text = Document.open(read("fixtures/golden/examples/hello.pdf")).page(0).text();
    expect([text.text(), text.runs.length, text.glyphCount, text.runs[0]?.glyphs().length]).toEqual([
      "Hello, world!",
      1,
      13,
      13,
    ]);
  });

  it("gives what Rust gives for every snapshot file: lines, runs and quads", () => {
    const snapshot = JSON.parse(readText("fixtures/text/page-text.json")) as Snapshot;
    for (const [file, pages] of Object.entries(snapshot.files)) {
      const doc = Document.open(read(`fixtures/${file}`));
      const ours = doc.pages.map((_, i) => pageJson(doc.page(i).text()));
      expect(ours, file).toEqual(pages);
    }
  });

  it("extracts encrypted copies like their originals", () => {
    const snapshot = JSON.parse(readText("fixtures/text/page-text.json")) as Snapshot;
    const pairs: [string, string][] = [
      ["text/encrypted/text-shaped-aes-256.pdf", "golden/text-shaped.pdf"],
      ["text/encrypted/build-text-multiscript-rc4-128.pdf", "golden/build-text-multiscript.pdf"],
    ];
    for (const [encrypted, original] of pairs) {
      const doc = Document.open(read(`fixtures/${encrypted}`), { password: "user" });
      const lines = doc.pages.map((_, i) => doc.page(i).text().lines);
      expect(lines, encrypted).toEqual(snapshot.files[original]?.map((p) => (p as { lines: string[] }).lines));
    }
  });

  it("leaves out invisible text when asked", () => {
    const doc = Document.open(read("fixtures/golden/build-text-latin.pdf"));
    const all = doc.page(0).text();
    const visible = doc.page(0).text({ skipInvisible: true });
    expect(all.runs.some((r) => r.invisible)).toBe(true);
    expect(visible.runs.some((r) => r.invisible)).toBe(false);
    expect(visible.lines).not.toContain("Invisible but extractable");
  });

  it("hands over its arrays without copies, for a worker", () => {
    const text = Document.open(read("fixtures/golden/examples/hello.pdf")).page(0).text();
    const { quads, glyphOffsets } = text.buffers;
    expect([quads instanceof Float32Array, quads.length, glyphOffsets.length]).toEqual([true, 13 * 8, 14]);
  });
});
