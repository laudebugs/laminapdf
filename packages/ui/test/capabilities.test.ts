// The capability page's filters (O05): the search, the group, and filters read back from storage.

import { describe, expect, it } from "vitest";
import { type CapabilityMatrix, filterMatrix, NO_FILTER, storedFilter } from "../src/help/capabilities.ts";

const op = (id: string, name: string) => ({
  id,
  name,
  round: "E1",
  target: "round-trip" as const,
  surfaces: { core: "write" as const, pdf: null, web: null, desktop: "n/a" as const },
});

const MATRIX: CapabilityMatrix = {
  version: 1,
  groups: [
    { id: "C", name: "Page organisation & geometry", ops: [op("C.01", "Rotate pages"), op("C.06", "Merge documents")] },
    {
      id: "J",
      name: "Metadata",
      ops: [op("J.01", "Title, author, subject, keywords"), op("J.10", "Remove selected metadata")],
    },
  ],
};

describe("the capability filters", () => {
  it("leaves everything with no filter", () => {
    expect(filterMatrix(MATRIX, NO_FILTER).ops).toBe(4);
  });

  it("matches names and IDs, ignoring case and accents, and hides empty groups", () => {
    const merge = filterMatrix(MATRIX, { ...NO_FILTER, query: "MÉRGE" });
    expect([merge.ops, merge.groups.map((g) => g.id)]).toEqual([1, ["C"]]);
    expect(filterMatrix(MATRIX, { ...NO_FILTER, query: "j.1" }).groups[0]?.ops.map((o) => o.id)).toEqual(["J.10"]);
    expect(filterMatrix(MATRIX, { ...NO_FILTER, query: "stamp" })).toEqual({ groups: [], ops: 0 });
  });

  it("keeps one group", () => {
    expect(filterMatrix(MATRIX, { ...NO_FILTER, group: "J" }).groups.map((g) => g.id)).toEqual(["J"]);
  });

  it("reads stored filters back, dropping what isn't one", () => {
    expect(storedFilter({ query: "merge", group: "C", surfaces: ["web", "core", "nope"] }, MATRIX)).toEqual({
      query: "merge",
      group: "C",
      surfaces: ["core", "web"],
    });
    expect(storedFilter({ group: "Z", surfaces: "web" }, MATRIX)).toEqual(NO_FILTER);
    expect(storedFilter("junk", MATRIX)).toEqual(NO_FILTER);
  });
});
