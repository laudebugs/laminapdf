// doc.extract(): each format, the JSON byte for byte as Rust writes it (the goldens
// crates/laminapdf/tests/extract.rs shares), option errors, and the base module's refusal.
import {
  type ChunkOptions,
  Document,
  type ExtractOptions,
  extractWithModels,
  init,
  isLaminaPdfError,
  type LayoutRegion,
  loadExtract,
  type ModelInfo,
  type ModelRequest,
  type RegionLabel,
} from "@laminapdf/sdk";
import * as glue from "@laminapdf/wasm/extract";
import { afterAll, beforeAll, describe, expect, expectTypeOf, it } from "vitest";
import { blocksOf } from "../examples/extract-blocks.js";
import { chunksOf } from "../examples/extract-chunks.js";
import { extractForAnLlm } from "../examples/extract-markdown.js";
import { textOf } from "../examples/extract-text.js";
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

  it("runs the extraction guide's examples: the output their Rust twins print", () => {
    const invoice = read("fixtures/golden/examples/invoice.pdf");
    expect(textOf(invoice)).toBe(readText("fixtures/extract/examples/invoice.txt"));
    expect(blocksOf(invoice)).toBe(readText("fixtures/extract/examples/invoice-blocks.txt"));
    expect(chunksOf(invoice)).toBe(readText("fixtures/extract/examples/invoice-chunks.txt"));
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
    // `models` is extractJob's alone (ExtractJobOptions).
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

describe("doc.extractJob() with a model the host runs", () => {
  beforeAll(() => loadExtract());

  const layout: ModelInfo = { name: "host-layout", version: "1", sha256: "0".repeat(64), licence: "Apache-2.0" };
  const file = "fixtures/reading-order/two-columns-ordered.pdf";

  /** One region per block the rules found, labelled `label` (the first block `first`). */
  function regionsOf(doc: Document, first: RegionLabel, label: RegionLabel): LayoutRegion[] {
    const rules = doc.extract({ format: "json" });
    return rules.blocks
      .filter((b) => b.page === 0)
      .map((b, i) => {
        const [x, y, w, h] = b.bbox as [number, number, number, number];
        // A little larger than the block, as a model's box is.
        return { label: i === 0 ? first : label, bbox: [x - 1, y - 1, w + 2, h + 2], score: 0.9 };
      });
  }

  it("asks for each page's regions, then names the model in the JSON", () => {
    using doc = Document.open(read(file));
    const regions = regionsOf(doc, "section-header", "text");
    const job = doc.extractJob({ format: "json", models: { layout } });
    const requests: ModelRequest[] = [];
    let step = job.step(100);
    while (step.state === "progress") {
      expect(step.phase).toBe("model");
      if (step.request !== undefined) requests.push(step.request);
      job.provide({ kind: "layout", regions: step.request?.page === 0 ? regions : [] });
      step = job.step(100);
    }
    expect(requests.map((r) => [r.kind, r.page, r.dpi])).toEqual(requests.map((_, i) => ["layout", i, 150]));
    expect(step.output.generator.mode).toBe("rules+models");
    expect(step.output.generator.models).toEqual([layout]);
    // The model's label wins over the rules' kind, and its score is the block's confidence.
    const first = step.output.blocks[0];
    expect(first).toMatchObject({ type: "heading", level: 2 });
    expect(first?.confidence).toBe(0.9);
  });

  it("gives with extractWithModels what stepping by hand gives", async () => {
    using doc = Document.open(read(file));
    const regions = regionsOf(doc, "text", "text");
    const byHand = (() => {
      const job = doc.extractJob({ format: "markdown", models: { layout } });
      for (;;) {
        const step = job.step(100);
        if (step.state === "done") return step.output;
        job.provide({ kind: "layout", regions: step.request?.page === 0 ? regions : [] });
      }
    })();
    const seen: number[] = [];
    const looped = await extractWithModels(
      doc,
      { format: "markdown", models: { layout } },
      {
        layout: async (request) => {
          seen.push(request.page);
          return request.page === 0 ? regions : [];
        },
      },
    );
    expect(looped).toBe(byHand);
    expect(seen[0]).toBe(0);
  });

  it("refuses a result nobody asked for", () => {
    using doc = Document.open(read(file));
    const job = doc.extractJob({ format: "text", models: { layout } });
    const e = thrown(() => job.provide({ kind: "ocr", words: [] }));
    expect(isLaminaPdfError(e, "model-result-unexpected")).toBe(true);
  });

  it("refuses a named model with no function to run it", async () => {
    using doc = Document.open(read(file));
    const e = await extractWithModels(doc, { format: "text", models: { layout } }, {}).catch((x: unknown) => x);
    expect(isLaminaPdfError(e, "invalid-argument")).toBe(true);
  });

  it("stops with cancelled when the signal aborts", async () => {
    using doc = Document.open(read(file));
    const abort = new AbortController();
    const e = await extractWithModels(
      doc,
      { format: "text", models: { layout } },
      {
        layout: async () => {
          abort.abort();
          return [];
        },
      },
      { signal: abort.signal },
    ).catch((x: unknown) => x);
    expect(isLaminaPdfError(e, "cancelled")).toBe(true);
  });

  it("ends cancelled when the model rejects because the signal aborted", async () => {
    using doc = Document.open(read(file));
    const abort = new AbortController();
    const e = await extractWithModels(
      doc,
      { format: "text", models: { layout } },
      {
        layout: async (_request, signal) => {
          abort.abort();
          throw signal?.reason;
        },
      },
      { signal: abort.signal },
    ).catch((x: unknown) => x);
    expect(isLaminaPdfError(e, "cancelled")).toBe(true);
  });

  it("is freed with its document", () => {
    const doc = Document.open(read(file));
    const job = doc.extractJob({ format: "text", models: { layout } });
    doc.free();
    expect(
      isLaminaPdfError(
        thrown(() => job.step()),
        "disposed",
      ),
    ).toBe(true);
  });
});
