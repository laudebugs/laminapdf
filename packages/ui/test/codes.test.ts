import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { failureSentence, hasRepairSentence, repairSentence } from "../src/reader/codes.ts";

const codes = JSON.parse(readFileSync(new URL("../../../fixtures/codes.json", import.meta.url), "utf8")) as Record<
  string,
  unknown
>;
const list = (group: string) => (codes[group] as { code: string }[]).map((entry) => entry.code);

describe("the reader's sentences for codes", () => {
  it("give every warning in codes.json a sentence of its own or its family's", () => {
    expect(list("warnings").filter((code) => !hasRepairSentence(code))).toEqual([]);
  });

  it("never show a code in place of words", () => {
    for (const code of [...list("warnings"), "a-code-from-the-future"])
      expect(repairSentence(code)).not.toContain(code);
    for (const code of [...list("errors"), "a-code-from-the-future"]) expect(failureSentence(code)).not.toContain(code);
  });

  it("explain the errors R04 draws in their own words, and the rest generically", () => {
    expect(failureSentence("not-pdf")).toBe("This file isn't a PDF.");
    expect(failureSentence("missing-root")).toContain("couldn't find the document's list of pages");
    expect(failureSentence("decompress")).toBe(failureSentence("a-code-from-the-future"));
  });

  it("say to try again, not to ask for a new copy, when opening failed for reasons outside the file", () => {
    for (const code of ["fetch-failed", "out-of-memory", "worker-crashed", "unknown"]) {
      expect(failureSentence(code)).toBe(
        "Lamina PDF couldn't finish opening this file. Try again, or reload the page.",
      );
    }
  });

  it("can't mistake a label for a code's sentence", () => {
    expect(repairSentence("title")).toBe(repairSentence("a-code-from-the-future"));
    expect(failureSentence("copy")).toBe(failureSentence("a-code-from-the-future"));
  });
});
