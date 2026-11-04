// Trap recovery, over the base module built with laminapdf-wasm's test hooks (vitest.node.config.ts
// points `@laminapdf/wasm/base` at it; its maximum memory is 64 MiB). A trap in a call throws
// `wasm-trap`, growth past the maximum `out-of-memory`; the next call runs in a fresh instance, and
// handles from the old one throw `instance-reset`.
import { readFileSync } from "node:fs";
import { Document, init, isLaminaPdfError, wasmMemory } from "@laminapdf/sdk";
import { fromRoot } from "@laminapdf/test-utils/root";
import { beforeAll, describe, expect, it } from "vitest";
import { baseModule } from "../../dist/init.js";
import { read, thrown } from "../helpers.js";

/** The hooks, which only the test build exports. */
interface Hooks {
  __trap(): void;
  __grow(bytes: number): void;
}

const hello = read("fixtures/golden/examples/hello.pdf");

/** Calls a hook the way every call of the package runs: through the instance's guard. */
function hook(f: (hooks: Hooks) => void): unknown {
  return thrown(() => baseModule().call((glue) => f(glue as unknown as Hooks)));
}

beforeAll(() => init({ module: readFileSync(fromRoot("target/wasm/test-hooks/lamina_pdf_base_bg.wasm")) }));

describe("a trap", () => {
  it("throws wasm-trap; the next open works and older handles throw instance-reset", () => {
    const before = Document.open(hello);
    const error = hook((h) => h.__trap());
    expect(
      isLaminaPdfError(error, "wasm-trap") && [error.data, error.cause instanceof WebAssembly.RuntimeError],
    ).toEqual([{ module: "base" }, true]);
    using after = Document.open(hello);
    expect([
      after.pageCount,
      isLaminaPdfError(
        thrown(() => before.pageCount),
        "instance-reset",
      ),
    ]).toEqual([1, true]);
    before.free();
  });
});

describe("growth past the maximum memory", () => {
  it("throws out-of-memory; the next open works in a fresh instance", () => {
    const before = Document.open(hello);
    const error = hook((h) => h.__grow(128 << 20));
    const data = isLaminaPdfError(error, "out-of-memory") ? error.data : undefined;
    expect([Object.keys(data ?? {}).sort(), data?.module]).toEqual([["memoryBytes", "module"], "base"]);
    const grown = Number(data?.memoryBytes);
    expect(grown).toBeGreaterThan(48 << 20);
    using after = Document.open(hello);
    expect([
      after.pageCount,
      isLaminaPdfError(
        thrown(() => before.pageCount),
        "instance-reset",
      ),
    ]).toEqual([1, true]);
    expect(wasmMemory().base).toBeLessThan(grown);
    before.free();
  });
});
