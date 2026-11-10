import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { LATER, PROMISES } from "../src/data/privacy.ts";

// The privacy page states ADR 0011's guarantees: every one of §3's ten is on the page, each
// promise names the guarantees it states, and each proof is a test that exists, by its file and
// its own name, so a renamed or deleted test fails here instead of leaving a promise unchecked.
const root = new URL("../../../", import.meta.url);
const read = (path: string) => readFileSync(new URL(path, root), "utf8");

describe("the privacy page", () => {
  it("states every one of ADR 0011's ten guarantees", () => {
    const stated = new Set(PROMISES.flatMap((p) => p.rules));
    expect([...Array(10)].map((_, i) => String(i + 1)).filter((rule) => !stated.has(rule))).toEqual([]);
  });

  it("names only guarantees the ADR has", () => {
    const adr = read("docs/decisions/0011-cloudflare-topology-and-privacy.md");
    const section = (heading: string) =>
      adr.slice(adr.indexOf(heading), adr.indexOf("\n### ", adr.indexOf(heading) + 1));
    const three = section("### 3. The v0.1 privacy guarantees");
    const six = section("### 6. The hosted service");
    for (const rule of [...PROMISES, LATER].flatMap((p) => p.rules)) {
      const [part, n] = rule.includes(".") ? (rule.split(".") as [string, string]) : ["3", rule];
      expect((part === "6" ? six : three).includes(`\n${n}. **`), rule).toBe(true);
    }
  });

  it("checks each promise with a test that exists", () => {
    for (const promise of PROMISES) {
      expect(promise.proofs.length, promise.heading).toBeGreaterThan(0);
      for (const { file, test } of promise.proofs) {
        expect(read(file).includes(`"${test}"`), `${file}: ${test}`).toBe(true);
      }
    }
    expect(LATER.proofs).toEqual([]);
    expect(LATER.untested).toBeDefined();
  });
});
