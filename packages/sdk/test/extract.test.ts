// doc.extract(): each format, the JSON byte for byte as Rust writes it (the goldens
// crates/laminapdf/tests/extract.rs shares), option errors, and the base module's refusal.
import { type ChunkOptions, Document, type ExtractOptions, init, isLaminaPdfError, loadExtract } from "@laminapdf/sdk";
import * as glue from "@laminapdf/wasm/extract";
import { afterAll, beforeAll, describe, expect, expectTypeOf, it } from "vitest";
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

  it("names the same options as the binding", () => {
    // The SDK's option types are written out for their docs; the binding's come from Rust.
    // `models` is the binding's extractJob's alone, which the SDK doesn't wrap yet.
    expectTypeOf<keyof ExtractOptions>().toEqualTypeOf<Exclude<keyof glue.ExtractInit, "models">>();
    expectTypeOf<keyof ChunkOptions>().toEqualTypeOf<keyof NonNullable<glue.ExtractInit["chunk"]>>();
  });

  it("refuses an unknown format at compile time", () => {
    const doc = Document.open(read("fixtures/reading-order/table-ordered.pdf"));
    // @ts-expect-error: "html" is not a format.
    const e = thrown(() => doc.extract({ format: "html" }));
    expect(isLaminaPdfError(e, "invalid-argument")).toBe(true);
  });
});

describe("WasmDocument.extractJob with a model the host runs", () => {
  beforeAll(() => loadExtract());

  const layout = { name: "host-layout", version: "1", sha256: "0".repeat(64), licence: "Apache-2.0" };

  it("asks for each page's regions, then names the model in the JSON", () => {
    const doc = glue.WasmDocument.openBytes(read("fixtures/reading-order/two-columns-ordered.pdf"), {});
    const job = doc.extractJob({ format: "json", models: { layout } });
    const requests: unknown[] = [];
    let step = job.step(100);
    while (step.state === "progress") {
      expect(step.phase).toBe("model");
      requests.push(step.request);
      job.provide({ kind: "layout", regions: [] });
      step = job.step(100);
    }
    expect(requests).toEqual([{ kind: "layout", page: 0, dpi: 150, langs: [] }]);
    const json = JSON.parse(step.output);
    expect(json.generator.mode).toBe("rules+models");
    expect(json.generator.models).toEqual([layout]);
    doc.free();
  });

  it("refuses a result nobody asked for", () => {
    const doc = glue.WasmDocument.openBytes(read("fixtures/reading-order/two-columns-ordered.pdf"), {});
    const job = doc.extractJob({ format: "text", models: { layout } });
    // The glue throws the ADR 0026 object itself; the SDK is what wraps it in LaminaPdfError.
    const e = thrown(() => job.provide({ kind: "ocr", words: [] })) as { code: string };
    expect(e.code).toBe("model-result-unexpected");
    doc.free();
  });
});
