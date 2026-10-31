// The hand-written code table against fixtures/codes.json: every RenderErrorCode exists there
// with the same kind.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { RenderError, type RenderErrorCode } from "../src/index.js";

const CODES: RenderErrorCode[] = [
  "render-too-large",
  "render-failed",
  "render-backend-unavailable",
  "invalid-argument",
  "page-index-out-of-range",
  "password-required",
  "wrong-password",
  "data-pending",
  "source",
  "cancelled",
  "disposed",
];

describe("RenderErrorCode", () => {
  it("names codes of fixtures/codes.json, with their kinds", () => {
    const file = JSON.parse(readFileSync(new URL("../../../fixtures/codes.json", import.meta.url), "utf8")) as Record<
      string,
      unknown
    >;
    const kinds = new Map<string, string>();
    for (const group of Object.values(file)) {
      if (!Array.isArray(group)) continue;
      for (const entry of group as { code: string; kind: string }[]) kinds.set(entry.code, entry.kind);
    }
    for (const code of CODES) {
      expect(kinds.get(code), code).toBe(new RenderError(code, "x").kind);
    }
  });

  it("covers every renderErrors code", () => {
    const file = JSON.parse(readFileSync(new URL("../../../fixtures/codes.json", import.meta.url), "utf8")) as {
      renderErrors: { code: string }[];
    };
    for (const { code } of file.renderErrors) expect(CODES).toContain(code);
  });
});
