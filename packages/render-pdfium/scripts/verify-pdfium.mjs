// ADR 0004 §3: the PDFium build is pinned by hash. Fails (exit 1) unless @embedpdf/pdfium's
// pdfium.wasm has the SHA-256 recorded here; `build`, `typecheck` and `test` run it first.
// A bump re-pins the package version in package.json, this hash and ADR 0004 together.
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { argv, exit } from "node:process";
import { fileURLToPath } from "node:url";

/** The pinned `@embedpdf/pdfium` 2.15.1 `dist/pdfium.wasm` (ADR 0004 §3). */
export const PDFIUM_WASM_SHA256 = "5e4cd023c3dad4a895b3571ca573d3fc51bac6de48360d95283134385b954eaa";

/** Throws unless `bytes` hash to `expected`. */
export function verify(bytes, expected = PDFIUM_WASM_SHA256) {
  const actual = createHash("sha256").update(bytes).digest("hex");
  if (actual !== expected) {
    throw new Error(
      `pdfium.wasm SHA-256 mismatch: expected ${expected}, got ${actual} (ADR 0004 §3: re-pin deliberately)`,
    );
  }
  return actual;
}

/** The installed pdfium.wasm's path. */
export function wasmPath() {
  return createRequire(import.meta.url).resolve("@embedpdf/pdfium/pdfium.wasm");
}

if (argv[1] === fileURLToPath(import.meta.url)) {
  try {
    verify(readFileSync(wasmPath()));
  } catch (e) {
    console.error(String(e instanceof Error ? e.message : e));
    exit(1);
  }
}
