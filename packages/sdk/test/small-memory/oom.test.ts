// Out of memory, for real: over the base module linked with a 64 MiB maximum (vitest.node.config.ts
// points `@laminapdf/wasm/base` at the build in target/wasm/small-memory/). Opening a file larger
// than that fails the allocation that copies it in, the module traps, and the call throws
// `out-of-memory`; the next call runs in a fresh instance, and older handles throw `instance-reset`.
import { readFileSync } from "node:fs";
import { Document, init, isLaminaPdfError } from "@laminapdf/sdk";
import { fromRoot } from "@laminapdf/test-utils/root";
import { beforeAll, describe, expect, it } from "vitest";
import { read, thrown } from "../helpers.js";

const hello = read("fixtures/golden/examples/hello.pdf");

beforeAll(() => init({ module: readFileSync(fromRoot("target/wasm/small-memory/lamina_pdf_base_bg.wasm")) }));

describe("growth past the maximum memory", () => {
  it("throws out-of-memory; the next open works in a fresh instance", () => {
    const before = Document.open(hello);
    const error = thrown(() => Document.open(new Uint8Array(100 << 20)));
    const data = isLaminaPdfError(error, "out-of-memory") ? error.data : undefined;
    expect([Object.keys(data ?? {}).sort(), data?.module]).toEqual([["memoryBytes", "module"], "base"]);
    expect(isLaminaPdfError(error) && error.cause instanceof WebAssembly.RuntimeError).toBe(true);
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
