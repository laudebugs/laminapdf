// Memory: freed documents leave memory for the next ones, `reset()` gives grown memory back, and
// `onLeak` reports handles collected without a `free()`. Byte counts only, never timings.
import {
  Builder,
  callbackSource,
  Document,
  Image,
  init,
  isLaminaPdfError,
  type Leak,
  onLeak,
  reset,
  wasmMemory,
} from "@laminapdf/sdk";
import { afterEach, beforeAll, describe, expect, it } from "vitest";
import { baseModule } from "../dist/init.js";
import { read, thrown } from "./helpers.js";

/** One WebAssembly page. */
const PAGE = 65_536;

const outline = read("fixtures/corpus/outline.pdf");
let fresh = 0;

/** Opens the corpus file, reads every page, and frees it. */
function openReadFree(): void {
  using doc = Document.open(outline);
  for (let i = 0; i < doc.pageCount; i++) {
    using page = doc.page(i);
    expect(page.info.index).toBe(i);
  }
  expect(doc.info).toBeDefined();
}

/** A document with a 1,000 × 1,000 RGBA image, built and saved: about 8 MB of memory. */
function growMemory(): void {
  using image = Image.rgba(1000, 1000, new Uint8Array(4_000_000).fill(200));
  const builder = new Builder();
  builder.page("A4").surface().image(image, [0, 0, 500, 500]);
  using doc = builder.finish();
  doc.save();
}

beforeAll(async () => {
  await init();
  fresh = wasmMemory().base ?? 0;
});

afterEach(() => onLeak());

describe("memory", () => {
  it("stays within one page of its warmed-up size over 1,000 open, read and free loops", () => {
    for (let i = 0; i < 20; i++) openReadFree();
    const warm = wasmMemory().base ?? 0;
    for (let i = 0; i < 1000; i++) openReadFree();
    expect((wasmMemory().base ?? 0) - warm).toBeLessThanOrEqual(PAGE);
  });

  it("returns to its size after init() on reset()", () => {
    growMemory();
    expect(wasmMemory().base).toBeGreaterThan(fresh + 100 * PAGE);
    reset();
    expect(wasmMemory().base).toBe(fresh);
  });

  it("makes every handle from before reset() throw instance-reset, and later calls work", () => {
    const doc = Document.open(outline);
    const page = doc.page(0);
    const builder = new Builder();
    const image = Image.rgba(1, 1, new Uint8Array([0, 0, 0, 255]));
    expect(image.width).toBe(1);
    reset();
    const codes = [() => doc.pageCount, () => page.info, () => builder.page("A6")].map(
      (f) => (thrown(f) as { code?: string }).code,
    );
    expect(codes).toEqual(["instance-reset", "instance-reset", "instance-reset"]);
    // Freeing them is harmless: their memory went with the old instance.
    doc.free();
    builder.free();
    // An image keeps its bytes and decodes again in the new instance.
    using again = Document.open(outline);
    const b = new Builder();
    const drawn = b.page("A6");
    drawn.surface().image(image, [0, 0, 10, 10]);
    drawn.finish();
    using built = b.finish();
    expect([again.pageCount, built.pageCount, image.width]).toEqual([3, 1, 1]);
  });
});

// A trap as the engine throws it, from inside the guard every call of the package runs through: a
// real one comes only from a bug, or from memory running out (test/small-memory/).
describe("a trap", () => {
  const trapWith = (error: Error): unknown =>
    thrown(() =>
      baseModule().call(() => {
        throw error;
      }),
    );

  it("throws wasm-trap; the next call runs in a fresh instance and older handles throw instance-reset", () => {
    const before = Document.open(outline);
    const error = trapWith(new WebAssembly.RuntimeError("unreachable"));
    expect(isLaminaPdfError(error, "wasm-trap") && error.data).toEqual({ module: "base" });
    using after = Document.open(outline);
    expect([
      after.pageCount,
      isLaminaPdfError(
        thrown(() => before.pageCount),
        "instance-reset",
      ),
    ]).toEqual([3, true]);
    before.free();
  });

  it("is also what a stack overflow inside the module is", () => {
    const v8 = trapWith(new RangeError("Maximum call stack size exceeded"));
    const firefox = trapWith(Object.assign(new Error("too much recursion"), { name: "InternalError" }));
    expect([isLaminaPdfError(v8, "wasm-trap"), isLaminaPdfError(firefox, "wasm-trap")]).toEqual([true, true]);
  });

  it("is not what an ordinary error is: the instance and its handles stay", () => {
    using doc = Document.open(outline);
    const error = trapWith(new TypeError("not a trap"));
    expect([error instanceof TypeError, doc.pageCount]).toEqual([true, 3]);
  });
});

describe("a reset() during a range open", () => {
  it("ends the open with instance-reset, and the fresh instance works", async () => {
    const source = callbackSource(outline.length, async (offset, length) => {
      reset();
      return outline.slice(offset, offset + length);
    });
    await expect(Document.openRange(source)).rejects.toSatisfy((e: unknown) => isLaminaPdfError(e, "instance-reset"));
    using doc = Document.open(outline);
    expect(doc.pageCount).toBe(3);
  });
});

describe("onLeak", () => {
  const gc = (globalThis as { gc?: () => void }).gc;

  /** Collects garbage until `leaks` has `count` reports, or gives up after 50 rounds. */
  async function collect(leaks: Leak[], count: number): Promise<void> {
    for (let round = 0; round < 50 && leaks.length < count; round++) {
      gc?.();
      await new Promise((resolve) => setTimeout(resolve, 0));
    }
  }

  it.runIf(gc !== undefined)("reports nothing when every handle is freed", async () => {
    const leaks: Leak[] = [];
    onLeak((leak) => leaks.push(leak));
    for (let i = 0; i < 10; i++) openReadFree();
    await collect(leaks, 1);
    expect(leaks).toEqual([]);
  });

  it.runIf(gc !== undefined)("reports nothing for a builder that finished", async () => {
    const leaks: Leak[] = [];
    onLeak((leak) => leaks.push(leak));
    (() => {
      const builder = new Builder();
      builder.page("A6").finish();
      builder.finish().free();
    })();
    await collect(leaks, 1);
    expect(leaks).toEqual([]);
  });

  it.runIf(gc !== undefined)("reports a document the garbage collector reclaimed", async () => {
    const leaks: Leak[] = [];
    onLeak((leak) => leaks.push(leak));
    (() => {
      expect(Document.open(outline).pageCount).toBe(3);
    })();
    await collect(leaks, 1);
    expect(leaks).toEqual([{ handle: "Document" }]);
  });
});
