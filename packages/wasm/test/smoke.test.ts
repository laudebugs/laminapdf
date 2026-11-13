// Smoke test of the built modules in Node (todo B6a): both load, the base module opens, reads,
// edits, saves and builds vector and image pages, and the text module builds the `hello`
// example byte for byte like the Rust golden. Every error the modules throw is the ADR 0026
// object and validates against laminapdf.error/1. Run by `cargo xtask wasm` after it fills dist/.
import { readFileSync } from "node:fs";
import { fromRoot } from "@laminapdf/test-utils/root";
import { Ajv2020 } from "ajv/dist/2020.js";
import { describe, expect, it } from "vitest";
import * as base from "../dist/lamina_pdf_base.js";
import * as extract from "../dist/lamina_pdf_extract.js";
import * as hayro from "../dist/lamina_pdf_render_hayro.js";
import * as text from "../dist/lamina_pdf_text.js";
import * as tiff from "../dist/lamina_pdf_tiff.js";

const read = (path: string): Uint8Array => new Uint8Array(readFileSync(fromRoot(path)));

base.initSync({ module: read("packages/wasm/dist/lamina_pdf_base.wasm") });
text.initSync({ module: read("packages/wasm/dist/lamina_pdf_text.wasm") });
extract.initSync({ module: read("packages/wasm/dist/lamina_pdf_extract.wasm") });
hayro.initSync({ module: read("packages/wasm/dist/lamina_pdf_render_hayro.wasm") });
tiff.initSync({ module: read("packages/wasm/dist/lamina_pdf_tiff.wasm") });

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

// ISO 216 A4 as laminapdf-model's `Pt::from_mm` computes it (mm * 72 / 25.4).
const A4 = [(210 * 72) / 25.4, (297 * 72) / 25.4] as const;

describe("lamina_pdf_base", () => {
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
    expect(doc.undo()).toEqual({ outcome: "applied", touchedPages: [id] });
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
      "https://docs.laminapdf.com/reference/codes/not-pdf/",
    ]);
  });

  it("refuses an unknown option and suggests the known one", () => {
    const options = { pasword: "x" } as unknown as base.OpenOptionsInit;
    const e = thrown(() => base.WasmDocument.openBytes(read("fixtures/golden/examples/hello.pdf"), options));
    expect([e.code, e.didYouMean]).toEqual(["unknown-option", ["password"]]);
  });
});

describe("lamina_pdf_text", () => {
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

describe("lamina_pdf_extract", () => {
  it("reads and extracts text, without fonts or shaping", () => {
    expect(extract.hasText()).toBe(false);
    const doc = extract.WasmDocument.openBytes(read("fixtures/golden/examples/hello.pdf"), undefined);
    const text = doc.page(0).text(undefined);
    expect([text.lines, text.runs.length, text.quads.length]).toEqual([["Hello, world!"], 1, 13 * 8]);
  });
});

/** The module's input buffer holding `bytes`. */
function input(bytes: Uint8Array): hayro.InputBuffer {
  const buffer = new hayro.InputBuffer(bytes.length);
  buffer.write(0, bytes);
  return buffer;
}

describe("lamina_pdf_render_hayro", () => {
  it("opens a page and renders a white tile", () => {
    const doc = hayro.HayroDocument.open(
      input(read("fixtures/corpus/hello.pdf")),
      undefined,
      16_777_216,
      16_384,
      2 ** 30,
    );
    expect(doc.pageCount()).toBe(1);
    const [width, height] = Array.from(doc.page(0));
    expect([width, height]).toEqual([612, 792]);
    // The top-left 8 × 8 pixels at 1×, upright, annotations and widgets on, on white, in the
    // page's own colours (appearance 0), then in Dark (2): white paper becomes Dark's.
    const out = new Uint8Array(8 * 8 * 4);
    doc.render(0, 1, 0, 0, 8, 8, 0, 3, 0xffffffff, 0, 0, 0, out);
    expect(out.every((b) => b === 255)).toBe(true);
    doc.render(0, 1, 0, 0, 8, 8, 0, 3, 0xffffffff, 2, 0, 0, out);
    expect(Array.from(out.subarray(0, 4))).toEqual([0x22, 0x21, 0x1f, 255]);
    doc.free();
  });

  it("throws render errors with their code and kind", () => {
    const e = thrown(() =>
      hayro.HayroDocument.open(input(new TextEncoder().encode("not a pdf")), undefined, 16_777_216, 16_384, 2 ** 30),
    );
    expect([e.code, e.kind]).toEqual(["render-failed", "input"]);
  });
});

describe("lamina_pdf_tiff", () => {
  /** The image file directories a little-endian TIFF chains, as their entry counts. */
  function directories(file: Uint8Array): number[] {
    const view = new DataView(file.buffer, file.byteOffset, file.byteLength);
    const counts: number[] = [];
    for (let at = view.getUint32(4, true); at !== 0; ) {
      const entries = view.getUint16(at, true);
      counts.push(entries);
      at = view.getUint32(at + 2 + entries * 12, true);
    }
    return counts;
  }

  it("writes one page per addPage into one little-endian file", () => {
    using file = new tiff.TiffFile();
    file.addPage(2, 1, new Uint8Array([255, 0, 0, 255, 0, 0, 255, 255]), 300);
    file.addPage(1, 1, new Uint8Array([0, 255, 0, 255]), 72);
    const bytes = file.finish();
    expect(Array.from(bytes.subarray(0, 4))).toEqual([0x49, 0x49, 42, 0]);
    expect(directories(bytes)).toHaveLength(2);
  });

  it("takes no page after it finished", () => {
    using file = new tiff.TiffFile();
    file.addPage(1, 1, new Uint8Array(4), 72);
    file.finish();
    expect(thrown(() => file.addPage(1, 1, new Uint8Array(4), 72)).code).toBe("invalid-argument");
  });
});

describe("errors thrown by both modules", () => {
  const schema = JSON.parse(readFileSync(fromRoot("crates/laminapdf/schemas/error.schema.json"), "utf8")) as object;
  const validate = new Ajv2020({ allErrors: true, strict: true }).compile(schema);
  const hello = (): Uint8Array => read("fixtures/corpus/hello.pdf");
  const open = (bytes: Uint8Array, maxPixels = 16_777_216): hayro.HayroDocument =>
    hayro.HayroDocument.open(input(bytes), undefined, maxPixels, 16_384, 2 ** 30);
  // [module, code, a call that throws it]
  const cases: [string, string, () => unknown][] = [
    ["lamina_pdf_base", "not-pdf", () => base.WasmDocument.openBytes(new TextEncoder().encode("not a pdf"), undefined)],
    [
      "lamina_pdf_base",
      "unknown-option",
      () => base.WasmDocument.openBytes(hello(), { pasword: "x" } as unknown as base.OpenOptionsInit),
    ],
    [
      "lamina_pdf_base",
      "invalid-argument",
      () => base.WasmDocument.openBytes(hello(), { strict: "yes" } as unknown as base.OpenOptionsInit),
    ],
    ["lamina_pdf_render_hayro", "render-failed", () => open(new TextEncoder().encode("not a pdf"))],
    ["lamina_pdf_render_hayro", "invalid-argument", () => new hayro.InputBuffer(4).write(2, new Uint8Array(4))],
    ["lamina_pdf_render_hayro", "page-index-out-of-range", () => open(hello()).page(5)],
    ["lamina_pdf_render_hayro", "render-too-large", () => open(hello(), 0).plan(0, 1, 0, 0, 0, 0, 0, 0, undefined)],
    ["lamina_pdf_tiff", "invalid-argument", () => new tiff.TiffFile().addPage(2, 2, new Uint8Array(4), 72)],
  ];

  for (const [module, code, call] of cases) {
    it(`${module} throws ${code} as an object that matches laminapdf.error/1`, () => {
      const e = thrown(call);
      expect(e.code).toBe(code);
      expect(validate(e), JSON.stringify(validate.errors)).toBe(true);
    });
  }
});
