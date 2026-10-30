// Smoke test of the built modules in Node (todo B6a): both load, the base module opens, reads,
// edits, saves and builds vector and image pages, and the text module builds the `hello`
// example byte for byte like the Rust golden. Run by `cargo xtask wasm` after it fills dist/.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import * as base from "../dist/lamina_base.js";
import * as text from "../dist/lamina_text.js";

const root = new URL("../../../", import.meta.url);
const read = (path: string): Uint8Array => new Uint8Array(readFileSync(new URL(path, root)));

base.initSync({ module: read("packages/wasm/dist/lamina_base.wasm") });
text.initSync({ module: read("packages/wasm/dist/lamina_text.wasm") });

/** What a failed call threw: the ADR 0026 §1 object. */
function thrown(f: () => unknown): Record<string, unknown> {
  try {
    f();
  } catch (e) {
    return e as Record<string, unknown>;
  }
  throw new Error("expected the call to throw");
}

const hex = (s: string): string =>
  [...new TextEncoder().encode(s)].map((b) => b.toString(16).padStart(2, "0")).join("");

// ISO 216 A4 as lamina-model's `Pt::from_mm` computes it (mm * 72 / 25.4).
const A4 = [(210 * 72) / 25.4, (297 * 72) / 25.4] as const;

describe("lamina_base", () => {
  it("is the module without text", () => {
    expect(base.hasText()).toBe(false);
  });

  it("opens a document and reads its pages", () => {
    const doc = base.WasmDocument.openBytes(read("fixtures/golden/examples/hello.pdf"), undefined);
    expect(doc.pageCount()).toBe(1);
    expect(Math.round(doc.pageInfo(0).width)).toBe(595);
  });

  it("applies the op samples, undoes and saves incrementally", () => {
    const doc = base.WasmDocument.openBytes(read("fixtures/golden/examples/hello.pdf"), undefined);
    const id = doc.page(0).id;
    const sample = JSON.parse(new TextDecoder().decode(read("fixtures/ops/v1/set-info.json")));
    doc.apply([sample]);
    doc.apply([{ op: "rotatePage", args: { page: id, rotate: 90 } }]);
    expect(doc.pageInfo(0).rotation).toBe(90);
    expect(doc.undo()).toBe("applied");
    const saved = doc.save({ mode: "incremental" });
    expect(saved.report.mode).toBe("incremental");
    const again = base.WasmDocument.openBytes(saved.bytes, undefined);
    expect(again.info().title).toBe(sample.args.patch.title);
  });

  it("steps a save job to the end", () => {
    const doc = base.WasmDocument.openBytes(read("fixtures/golden/examples/hello.pdf"), undefined);
    const job = doc.saveJob({ mode: "fullRewrite" });
    let step = job.step(1);
    while (step.state === "progress") {
      step = job.step(1);
    }
    expect(step.bytes.length).toBeGreaterThan(0);
  });

  it("builds vector and image pages", () => {
    const b = new base.WasmBuilder({ id: hex("lamina-wasm-base"), creationDate: "D:20261030120000Z" });
    const page = b.page(A4[0], A4[1]);
    page.fillColor(new Float64Array([0.1, 0.4, 0.8]));
    page.fillPath(new Float64Array([5, 56, 56, 200, 120]), "nonZero");
    page.strokeStyle(2, "round", "round", 10, new Float64Array(), 0);
    page.strokePath(new Float64Array([0, 56, 200, 1, 539, 200]));
    const image = base.WasmImage.rgba(2, 1, new Uint8Array([255, 0, 0, 255, 0, 0, 255, 128]));
    page.image(image, 440, 56, 100, 50);
    page.finish();
    const doc = b.finish();
    const bytes = doc.save(undefined).bytes;
    expect(base.WasmDocument.openBytes(bytes, undefined).pageCount()).toBe(1);
  });

  it("throws the ADR 0026 object for a file that isn't a PDF", () => {
    const e = thrown(() => base.WasmDocument.openBytes(new TextEncoder().encode("not a pdf"), undefined));
    expect([e.code, e.kind, e.docsUrl]).toEqual([
      "not-pdf",
      "input",
      "https://github.com/laudebugs/lamina/blob/main/docs/errors/not-pdf.md",
    ]);
  });

  it("refuses an unknown option and suggests the known one", () => {
    const options = { pasword: "x" } as unknown as base.OpenOptionsInit;
    const e = thrown(() => base.WasmDocument.openBytes(read("fixtures/golden/examples/hello.pdf"), options));
    expect([e.code, e.didYouMean]).toEqual(["unknown-option", ["password"]]);
  });
});

describe("lamina_text", () => {
  it("is the module with text", () => {
    expect(text.hasText()).toBe(true);
  });

  it("builds the hello example byte for byte like the Rust golden", () => {
    const b = new text.WasmBuilder({
      id: hex("lamina-ex-hello!"),
      producer: "lamina examples",
      creationDate: "D:20261030120000Z",
    });
    const helvetica = b.standardFont("Helvetica");
    const page = b.page(A4[0], A4[1]);
    page.text("Hello, world!", helvetica, 24, 72, 96, undefined);
    page.finish();
    const bytes = b.finish().save(undefined).bytes;
    expect(bytes).toEqual(read("fixtures/golden/examples/hello.pdf"));
  });
});
