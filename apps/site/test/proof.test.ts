// The committed snapshot against its type: the How we test page reads it at build time, so a file
// that `cargo xtask proof snapshot` and readProof disagree about fails here first.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { readProof } from "../src/data/proof.js";

const committed: unknown = JSON.parse(readFileSync(new URL("../src/data/proof.json", import.meta.url), "utf8"));

/** The committed file with `change` applied to a copy. */
// biome-ignore lint/suspicious/noExplicitAny: a test sets wrong values on purpose
function changed(change: (file: any) => void): unknown {
  const copy = structuredClone(committed);
  change(copy);
  return copy;
}

describe("proof.json", () => {
  it("is what readProof describes", () => {
    const proof = readProof(committed);
    expect(proof.version).toBe(1);
    expect(proof.nights.passed).toBeLessThanOrEqual(proof.nights.recorded);
    expect(proof.nights.recorded).toBeLessThanOrEqual(30);
    expect(proof.night.commit).toMatch(/^[0-9a-f]{40}$/);
  });

  it("holds no file name from the corpus and no person", () => {
    const text = readFileSync(new URL("../src/data/proof.json", import.meta.url), "utf8");
    expect(text).not.toMatch(/github\.com|Co-Authored|Claude-Session|\/[\w-]+\.pdf"/);
  });

  it("keeps a missing section null", () => {
    expect(readProof(changed((f) => (f.night.render = null))).night.render).toBeNull();
  });

  it("fails on a file the type doesn't describe, naming the field", () => {
    expect(() => readProof(changed((f) => (f.version = 2)))).toThrow("proof.json: the file.version is not 1");
    expect(() => readProof(changed((f) => (f.night.ran = "10 November")))).toThrow("the file.night.ran is not a date");
    expect(() => readProof(changed((f) => (f.night.corpus.sources.pdfjs.opened = -1)))).toThrow(
      "the file.night.corpus.sources.pdfjs.opened is not a count",
    );
    expect(() => readProof(changed((f) => (f.night.verapdf.validations[0].profile = "1b")))).toThrow(
      "the file.night.verapdf.validations[0].profile",
    );
    expect(() => readProof(changed((f) => delete f.nights))).toThrow("the file.nights is not an object");
  });
});
