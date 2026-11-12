// The capability page's filters (O05): the search, the group, and filters read back from storage.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  type CapabilityMatrix,
  type CapabilityStatus,
  filterMatrix,
  NO_FILTER,
  SURFACES,
  storedFilter,
} from "../src/help/capabilities.ts";

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

// The page's types copy what `cargo xtask capabilities` writes (tools/xtask's Status and Surface);
// the reader bundles the file with a cast, so this pins the file to them.
describe("docs/capabilities/matrix.json", () => {
  const json = JSON.parse(readFileSync(new URL("../../../docs/capabilities/matrix.json", import.meta.url), "utf8")) as {
    version: number;
    groups: { id: string; name: string; ops: Record<string, unknown>[] }[];
  };
  const STATUSES = ["unsupported", "read", "write", "round-trip"] as const satisfies readonly CapabilityStatus[];
  // Every status the page knows is in the list: a compile-time check.
  const complete: [Exclude<CapabilityStatus, (typeof STATUSES)[number]>] extends [never] ? true : false = true;

  it("is version 1, with groups of operations", () => {
    expect([json.version, json.groups.length > 0, json.groups.every((g) => g.ops.length > 0)]).toEqual([1, true, true]);
  });

  it("gives every operation the page's fields, a status the page knows per surface, and a target status", () => {
    const wrong = json.groups.flatMap((g) =>
      g.ops.filter((op) => {
        const surfaces = op.surfaces as Record<string, unknown>;
        return (
          JSON.stringify(Object.keys(op)) !== JSON.stringify(["id", "name", "round", "target", "surfaces"]) ||
          JSON.stringify(Object.keys(surfaces)) !== JSON.stringify(SURFACES) ||
          !complete ||
          !(STATUSES as readonly string[]).includes(op.target as string) ||
          !Object.values(surfaces).every(
            (v) => v === null || v === "n/a" || (STATUSES as readonly string[]).includes(v as string),
          )
        );
      }),
    );
    expect(wrong).toEqual([]);
  });
});
