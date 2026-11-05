// doc.extract(): each format, the JSON byte for byte as Rust writes it (the goldens
// crates/laminapdf/tests/extract.rs shares), option errors, and the base module's refusal.
import { Document, type ExtractOptions, init, isLaminaPdfError, loadExtract } from "@laminapdf/sdk";
import * as glue from "@laminapdf/wasm/extract";
import { afterAll, beforeAll, describe, expect, it } from "vitest";
import { extractForAnLlm } from "../examples/extract-markdown.js";
import { read, readText, thrown } from "./helpers.js";

beforeAll(() => init());

/** The goldens' files, as `SHARED` in crates/laminapdf/tests/extract.rs lists them. */
const SHARED = [
  "reading-order/table-ordered.pdf",
  "reading-order/two-columns-ordered.pdf",
  "corpus/hello-broken.pdf",
] as const;

/**
 * The strings the extract module's `WasmDocument.extract` returned, newest last: what
 * `doc.extract` parsed, as Rust wrote it (JSON.stringify of the parsed object would write
 * `1.0` as `1`).
 */
const raw: string[] = [];
const original = glue.WasmDocument.prototype.extract;
glue.WasmDocument.prototype.extract = function (this: glue.WasmDocument, options: never): string {
  const out = original.call(this, options);
  raw.push(out);
  return out;
};
afterAll(() => {
  glue.WasmDocument.prototype.extract = original;
});

/** The JSON with the generator's version replaced by `*`, as the Rust test writes it. */
function withoutVersion(json: string): string {
  return json.replace(/"version":"[^"]*"/, '"version":"*"');
}

describe("doc.extract() before the extract module", () => {
  it("throws extract-module-not-loaded for a document opened in the base module", () => {
    const doc = Document.open(read("fixtures/golden/examples/hello.pdf"));
    const e = thrown(() => doc.extract({ format: "markdown" }));
    expect(isLaminaPdfError(e, "extract-module-not-loaded")).toBe(true);
  });
});

describe("doc.extract() with the extract module", () => {
  beforeAll(() => loadExtract());

  it("gives the JSON Rust gives, byte for byte", () => {
    for (const rel of SHARED) {
      const doc = Document.open(read(`fixtures/${rel}`));
      const extraction = doc.extract({ format: "json", pageMarkers: true, chunk: { maxTokens: 400 } });
      const stem = rel
        .split("/")
        .pop()
        ?.replace(/\.pdf$/, "");
      const golden = readText(`fixtures/extract/json/${stem}.json`);
      expect(withoutVersion(raw.at(-1) ?? ""), rel).toBe(golden);
      expect({ ...extraction, generator: { ...extraction.generator, version: "*" } }, rel).toEqual(JSON.parse(golden));
    }
  });

  it("runs the recipe's example: the same JSON as the Rust example's options", () => {
    const out = extractForAnLlm(read("fixtures/reading-order/table-ordered.pdf"));
    expect(withoutVersion(raw.at(-1) ?? "")).toBe(readText("fixtures/extract/json/table-ordered.json"));
    expect(out.markdown.startsWith("<!-- page 1 -->")).toBe(true);
    expect(out.chunks).toEqual(out.json.chunks);
  });

  it("returns the Markdown Rust writes", () => {
    const doc = Document.open(read("fixtures/reading-order/table-ordered.pdf"));
    const markdown: string = doc.extract({ format: "markdown", pageMarkers: true });
    expect(markdown).toBe(readText("fixtures/extract/markdown/table-ordered.md"));
  });

  it("returns plain text and typed chunks", () => {
    const doc = Document.open(read("fixtures/reading-order/two-columns-ordered.pdf"));
    const text: string = doc.extract({ format: "text" });
    const chunks = doc.extract({ format: "chunks", chunk: { maxTokens: 50 } });
    expect(text.length).toBeGreaterThan(0);
    expect(chunks.length).toBeGreaterThan(1);
    expect(chunks[0]?.id).toBe("c0");
    expect(chunks.every((c) => c.tokensApprox > 0 && c.blockIds.length > 0)).toBe(true);
  });

  it("extracts only the pages asked for", () => {
    const doc = Document.open(read("fixtures/reading-order/table-ordered.pdf"));
    const json = doc.extract({ format: "json", pages: [0] });
    expect(json.pages.map((p) => p.index)).toEqual([0]);
    const e = thrown(() => doc.extract({ format: "json", pages: [5] }));
    expect(isLaminaPdfError(e, "page-index-out-of-range")).toBe(true);
  });

  it("refuses an unknown option with a suggestion", () => {
    const doc = Document.open(read("fixtures/reading-order/table-ordered.pdf"));
    const e = thrown(() => doc.extract({ format: "markdown", pageMarker: true } as unknown as ExtractOptions));
    expect(isLaminaPdfError(e) && [e.code, e.didYouMean]).toEqual(["unknown-option", ["pageMarkers"]]);
  });

  it("refuses an unknown format at compile time", () => {
    const doc = Document.open(read("fixtures/reading-order/table-ordered.pdf"));
    // @ts-expect-error: "html" is not a format.
    const e = thrown(() => doc.extract({ format: "html" }));
    expect(isLaminaPdfError(e, "invalid-argument")).toBe(true);
  });
});
