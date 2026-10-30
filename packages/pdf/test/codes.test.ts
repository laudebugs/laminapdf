// ADR 0003 §4, TS half: the generated code unions are fixtures/codes.json's codes, the codes the
// package raises carry codes.json's kind and hint, and every code has its docs/errors page.
import { existsSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { DOCS_URL, ERROR_CODES, RAISED_BY_PACKAGE, WARNING_CODES } from "../src/codes.generated.js";
import { readText } from "./helpers.js";

interface Entry {
  readonly code: string;
  readonly kind: string;
  readonly hint?: string;
}
const codes = JSON.parse(readText("fixtures/codes.json")) as Record<string, unknown>;
const groups = Object.entries(codes).filter(([k]) => !["version", "docsUrl", "limits"].includes(k)) as [
  string,
  Entry[],
][];
const all = groups.flatMap(([, list]) => list);
const isWarningGroup = (group: string) => group === "warnings" || group === "textWarnings";

describe("codes.generated", () => {
  it("lists every error code of codes.json", () => {
    const errors = groups.filter(([g]) => !isWarningGroup(g)).flatMap(([, l]) => l.map((e) => e.code));
    expect([...ERROR_CODES].sort()).toEqual(errors.sort());
  });

  it("lists every warning code of codes.json", () => {
    const warnings = groups.filter(([g]) => isWarningGroup(g)).flatMap(([, l]) => l.map((e) => e.code));
    expect([...WARNING_CODES].sort()).toEqual(warnings.sort());
  });

  it("gives the package's own codes codes.json's kind and hint", () => {
    for (const [code, { kind, hint }] of Object.entries(RAISED_BY_PACKAGE)) {
      const entry = all.find((e) => e.code === code);
      expect([entry?.kind, entry?.hint]).toEqual([kind, hint]);
    }
  });

  it("has a docs/errors page for every code", () => {
    const missing = all.filter((e) => !existsSync(new URL(`../../../docs/errors/${e.code}.md`, import.meta.url)));
    expect(missing).toEqual([]);
  });

  it("builds docsUrl from codes.json's constant", () => {
    expect(DOCS_URL).toEqual(codes.docsUrl);
  });
});
