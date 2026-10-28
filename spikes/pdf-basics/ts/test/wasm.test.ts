// SPEC §7: call the Rust core's C-ABI WASM build from TS (no wasm-bindgen).
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, test } from "vitest";
import { open, PdfError } from "../src/index.js";
import { corpusFile, corpusNames, golden, latin1, PKG, prng } from "./helpers.js";

const WASM = join(PKG, "..", "rust", "target", "wasm32-unknown-unknown", "release", "lamina_pdf_basics.wasm");
const present = existsSync(WASM);
if (!present) {
  console.warn(`[wasm.test] SKIPPED: ${WASM} not found. Build it with: cargo build --release --target wasm32-unknown-unknown (in ../rust)`);
}

interface Exports {
  memory: WebAssembly.Memory;
  alloc(len: number): number;
  dealloc(ptr: number, len: number): void;
  hello_pdf(outLenPtr: number): number;
  page_count(ptr: number, len: number): number;
}

describe.skipIf(!present)("case 7: Rust core via WASM C-ABI", async () => {
  const { instance } = present
    ? await WebAssembly.instantiate(readFileSync(WASM), {})
    : ({ instance: undefined } as unknown as WebAssembly.WebAssemblyInstantiatedSource);
  const x = instance?.exports as unknown as Exports;

  const copyIn = (bytes: Uint8Array): number => {
    const ptr = x.alloc(bytes.length);
    new Uint8Array(x.memory.buffer, ptr, bytes.length).set(bytes);
    return ptr;
  };

  test("exports the expected surface", () => {
    for (const k of ["memory", "alloc", "dealloc", "hello_pdf", "page_count"]) expect(k in x, k).toBe(true);
  });

  test("hello_pdf() is byte-identical to fixtures/golden/hello.pdf", () => {
    const lenPtr = x.alloc(4);
    const ptr = x.hello_pdf(lenPtr);
    const len = new DataView(x.memory.buffer).getUint32(lenPtr, true);
    const out = new Uint8Array(x.memory.buffer, ptr, len).slice();
    x.dealloc(ptr, len);
    x.dealloc(lenPtr, 4);
    expect(latin1(out)).toBe(latin1(golden("hello.pdf")));
  });

  test.each(corpusNames())("page_count agrees with the TS reader: %s", (f) => {
    const bytes = corpusFile(f);
    const ptr = copyIn(bytes);
    const n = x.page_count(ptr, bytes.length);
    x.dealloc(ptr, bytes.length);
    expect(n).toBe(open(bytes).pageCount());
  });

  test("page_count returns -1 on garbage", () => {
    const bytes = new Uint8Array([1, 2, 3, 4]);
    const ptr = copyIn(bytes);
    expect(x.page_count(ptr, bytes.length)).toBe(-1);
    x.dealloc(ptr, bytes.length);
  });

  test("differential: mutated corpus never traps the WASM build; disagreements are reported", () => {
    const rs = (b: Uint8Array): number => {
      const ptr = copyIn(b);
      const n = x.page_count(ptr, b.length); // a Rust panic would surface here as a WebAssembly.RuntimeError
      x.dealloc(ptr, b.length);
      return n;
    };
    const ts = (b: Uint8Array): number => {
      try {
        return open(b).pageCount();
      } catch (e) {
        if (e instanceof PdfError) return -1;
        throw e;
      }
    };
    let total = 0;
    const diffs: string[] = [];
    for (const f of corpusNames()) {
      const full = corpusFile(f);
      const rnd = prng(7 + full.length);
      const cases: Array<[string, Uint8Array]> = [];
      for (let n = 1; n <= full.length; n += Math.max(1, Math.floor(full.length / 200))) cases.push([`trunc ${n}`, full.subarray(0, n)]);
      for (let i = 0; i < 100; i++) {
        const m = full.slice();
        const at = Math.floor(rnd() * m.length);
        m[at] = Math.floor(rnd() * 256);
        cases.push([`byte ${at}=${m[at]}`, m]);
      }
      for (const [label, b] of cases) {
        total++;
        const a = ts(b);
        const r = rs(b);
        if (a !== r) diffs.push(`${f} ${label}: ts=${a} rust=${r}`);
      }
    }
    console.info(`[wasm.test] differential page_count: ${total - diffs.length}/${total} agree; first diffs:\n  ${diffs.slice(0, 8).join("\n  ")}`);
    expect(total).toBeGreaterThan(1000);
  });
});
