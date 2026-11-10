// The licence policy the How we test page states, read from Cargo.toml and deny.toml at build time:
// what the parser takes from each table, and that it reads the committed files whole.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { licencePolicy } from "../src/data/sources.js";

const read = (path: string) => readFileSync(new URL(`../../../${path}`, import.meta.url), "utf8");

const CARGO = `[workspace]
members = []

[workspace.package]
edition = "2024"
license = "MIT OR Apache-2.0"

[workspace.dependencies]
`;

const DENY = `[advisories]
ignore = []

[licenses]
# A comment above the list.
allow = [
    "MIT",
    # "GPL-3.0", a commented-out entry
    "Apache-2.0",
]
exceptions = [
    { name = "uniffi", allow = ["MPL-2.0"] },
    # A comment between entries.
    { name = "selectors", allow = ["MPL-2.0"] },
    { name = "borrow-or-share", allow = ["MIT-0"] },
]

[licenses.private]
ignore = true

[bans]
deny = [
    { name = "mupdf", reason = "AGPL-3.0 (ADR 0001, ADR 0019)" },
]
skip = []
`;

describe("the licence policy", () => {
  it("takes the licence, the allow list, the exceptions by licence and the bans", () => {
    expect(licencePolicy(CARGO, DENY)).toEqual({
      licence: "MIT OR Apache-2.0",
      allowed: ["MIT", "Apache-2.0"],
      exceptions: [
        { licence: "MPL-2.0", crates: ["uniffi", "selectors"] },
        { licence: "MIT-0", crates: ["borrow-or-share"] },
      ],
      banned: [{ name: "mupdf", reason: "AGPL-3.0 (ADR 0001, ADR 0019)" }],
    });
  });

  it("fails when a table it needs is missing", () => {
    expect(() => licencePolicy("[workspace]\n", DENY)).toThrow("no [workspace.package] table");
    expect(() => licencePolicy(CARGO, DENY.replace("[bans]", "[other]"))).toThrow("no [bans] table");
  });

  it("reads every entry of the committed files", () => {
    const deny = read("deny.toml");
    const policy = licencePolicy();
    expect(policy.licence).toBe("MIT OR Apache-2.0");
    // Every quoted name in the allow list and the ban list, counted in the file itself.
    const allowList = deny.slice(deny.indexOf("allow = ["), deny.indexOf("]", deny.indexOf("allow = [")));
    const quoted = [...allowList.matchAll(/^\s+"([^"]+)",/gm)].map(([, name]) => name);
    expect(policy.allowed).toEqual(quoted);
    expect(policy.banned.map((ban) => ban.name)).toEqual(["mupdf", "mupdf-sys"]);
    const exceptionLines = deny.match(/^\s+\{ name = "[^"]+", allow = /gm) ?? [];
    expect(policy.exceptions.flatMap((e) => e.crates)).toHaveLength(exceptionLines.length);
    expect(policy.exceptions.find((e) => e.licence === "MPL-2.0")?.crates).toContain("uniffi");
  });
});
