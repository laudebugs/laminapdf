// The build-time hash check (ADR 0004 §3): a mismatched pdfium.wasm fails.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { PDFIUM_WASM_SHA256, verify, wasmPath } from "../scripts/verify-pdfium.mjs";

describe("verify-pdfium", () => {
  it("accepts the pinned pdfium.wasm", () => {
    expect(verify(readFileSync(wasmPath()))).toBe(PDFIUM_WASM_SHA256);
  });

  it("names the version package.json pins", async () => {
    const manifest = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8")) as {
      dependencies: Record<string, string>;
    };
    const { PDFIUM_PACKAGE } = await import("../src/index.js");
    expect(PDFIUM_PACKAGE).toBe(`@embedpdf/pdfium@${manifest.dependencies["@embedpdf/pdfium"]}`);
  });

  it("refuses any other bytes", () => {
    const bytes = new Uint8Array(readFileSync(wasmPath()));
    bytes[100] = (bytes[100] ?? 0) ^ 1;
    expect(() => verify(bytes)).toThrow(/SHA-256 mismatch/);
  });
});
