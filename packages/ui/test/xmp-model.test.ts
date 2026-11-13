// The XMP tab's copy of Rust's Info-mapped properties: MAPPED lists the XMP property of every
// row of `xmp::MAPPINGS` that has an Info key, so the tab sends each one to the Description tab.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { MAPPED } from "../src/reader/xmp-model.ts";

const MAPPING_RS = new URL("../../../crates/laminapdf-model/src/xmp/mapping.rs", import.meta.url);

/** Rust's namespace constants (`pub const DC: &str = "…";`) and its rows with an Info key. */
function rustRows(): [string, string][] {
  const source = readFileSync(MAPPING_RS, "utf8");
  const consts = new Map([...source.matchAll(/pub const (\w+): &str = "([^"]+)";/g)].map((m) => [m[1], m[2]]));
  const prefixes = new Map(
    [...source.matchAll(/const (\w+)_NS: \(&str, &str\) = \("\w+", (\w+)\);/g)].map((m) => [
      m[1],
      consts.get(m[2] ?? ""),
    ]),
  );
  const rows = [...source.matchAll(/row\(\s*MappedField::\w+,\s*Some\("\w+"\),\s*(\w+)_NS,\s*"(\w+)"/g)];
  return rows.map((m) => [prefixes.get(m[1] ?? "") ?? "", m[2] ?? ""]);
}

describe("the XMP tab's mapped properties", () => {
  it("are the Info-mapped rows of xmp::MAPPINGS", () => {
    const rust = rustRows();
    expect(rust).toHaveLength(8);
    expect(MAPPED.map(([ns, name]) => [ns, name]).sort()).toEqual(rust.sort());
  });
});
