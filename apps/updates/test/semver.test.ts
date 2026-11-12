import { describe, expect, it } from "vitest";
import { compare, isVersion, parseVersion } from "../src/semver.ts";

describe("parseVersion", () => {
  it.each(["0.0.1", "1.2.3", "0.1.1-nightly.20261028.42", "1.0.0-alpha", "1.0.0+build.7", "10.20.30-rc.1+meta"])(
    "accepts %s",
    (v) => expect(isVersion(v)).toBe(true),
  );

  it.each(["", "1", "1.2", "v1.2.3", "01.2.3", "1.02.3", "1.2.3-01", "1.2.3-", "1.2.3+", "1.2.3 ", "1.2.3-a..b"])(
    "rejects %j",
    (v) => expect(isVersion(v)).toBe(false),
  );

  it("rejects an overlong version", () => {
    expect(isVersion(`1.2.3-${"a".repeat(200)}`)).toBe(false);
  });

  it("reads numeric pre-release identifiers as numbers", () => {
    expect(parseVersion("0.1.1-nightly.20261028.42")?.prerelease).toEqual(["nightly", 20261028, 42]);
  });
});

describe("compare", () => {
  // semver.org §11's own ordering, plus nightly versions.
  const ascending = [
    "0.0.1",
    "0.1.0",
    "0.1.1-nightly.20261028.9",
    "0.1.1-nightly.20261028.42",
    "0.1.1-nightly.20261029.1",
    "0.1.1",
    "1.0.0-alpha",
    "1.0.0-alpha.1",
    "1.0.0-alpha.beta",
    "1.0.0-beta",
    "1.0.0-beta.2",
    "1.0.0-beta.11",
    "1.0.0-rc.1",
    "1.0.0",
    "1.0.1",
    "1.10.0",
    "2.0.0",
  ];

  it("orders versions by precedence", () => {
    const shuffled = [...ascending].reverse();
    expect(shuffled.sort(compare)).toEqual(ascending);
  });

  it.each(ascending.slice(1).map((v, i) => [ascending[i], v]))("%s < %s", (a, b) => {
    expect(compare(a as string, b as string)).toBeLessThan(0);
    expect(compare(b as string, a as string)).toBeGreaterThan(0);
  });

  it("ignores build metadata", () => {
    expect(compare("1.0.0+a", "1.0.0+b")).toBe(0);
  });

  it("refuses a string that isn't a version", () => {
    expect(() => compare("1.0", "1.0.0")).toThrow(RangeError);
  });
});
