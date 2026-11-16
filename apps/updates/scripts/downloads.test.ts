// The downloads list (scripts/downloads.ts): which files are installers, SHA256SUMS' lines, and
// the list a release's built folders make.
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { type BuiltFolder, downloadsFrom, installerKind, parseSums } from "./downloads.ts";

const HASH_A = "a".repeat(64);
const HASH_B = "b".repeat(64);
const INPUT = { channel: "beta", version: "0.2.0", pubDate: "2026-11-15T12:00:00Z", downloads: "https://d.test" };

/** A built folder holding `sizes`' files, with `sums` as its SHA256SUMS. */
function folder(platform: string, sizes: Record<string, number>, sums?: string): BuiltFolder {
  const files = Object.keys(sizes);
  return {
    platform,
    files: sums === undefined ? files : [...files, "SHA256SUMS"],
    read: () => sums ?? "",
    size: (file) => sizes[file] ?? 0,
  };
}

describe("installerKind", () => {
  it("names each installer a person downloads", () => {
    const names = ["a.dmg", "a_x64-setup.exe", "a.msi", "a.AppImage", "a.deb", "a.rpm"];
    expect(names.map(installerKind)).toEqual(["dmg", "nsis", "msi", "appimage", "deb", "rpm"]);
  });

  it("leaves out the updater's bundles, signatures and SBOMs", () => {
    expect(["a.app.tar.gz", "a.dmg.sig", "a.deb.asc", "sbom.cdx.json"].map(installerKind)).toEqual([
      undefined,
      undefined,
      undefined,
      undefined,
    ]);
  });
});

describe("parseSums", () => {
  it("reads text and binary mode lines", () => {
    const sums = parseSums(`${HASH_A}  Lamina PDF_0.2.0.dmg\n${HASH_B} *b.deb\n`);
    expect([sums.get("Lamina PDF_0.2.0.dmg"), sums.get("b.deb")]).toEqual([HASH_A, HASH_B]);
  });
});

describe("downloadsFrom", () => {
  it("lists each installer with its size, hash and address", () => {
    const list = downloadsFrom(INPUT, [
      folder("darwin-aarch64", { "Lamina PDF.dmg": 9, "Lamina PDF.app.tar.gz": 8 }, `${HASH_A}  Lamina PDF.dmg\n`),
    ]);
    expect(list.files).toEqual([
      {
        platform: "darwin-aarch64",
        kind: "dmg",
        name: "Lamina PDF.dmg",
        size: 9,
        sha256: HASH_A,
        url: "https://d.test/releases/0.2.0/darwin-aarch64/Lamina%20PDF.dmg",
      },
    ]);
  });

  it("links a signed file's .asc and the folder's SHA256SUMS", () => {
    const built = folder("linux-x86_64", { "a.deb": 1, "a.deb.asc": 1, "SHA256SUMS.asc": 1 }, `${HASH_A}  a.deb\n`);
    const list = downloadsFrom(INPUT, [built]);
    expect([list.files[0]?.signature, list.checksums["linux-x86_64"]]).toEqual([
      "https://d.test/releases/0.2.0/linux-x86_64/a.deb.asc",
      {
        url: "https://d.test/releases/0.2.0/linux-x86_64/SHA256SUMS",
        signature: "https://d.test/releases/0.2.0/linux-x86_64/SHA256SUMS.asc",
      },
    ]);
  });

  it("refuses an installer SHA256SUMS doesn't list", () => {
    expect(() => downloadsFrom(INPUT, [folder("linux-x86_64", { "a.deb": 1 }, "")])).toThrow(
      "linux-x86_64: a.deb isn't in SHA256SUMS",
    );
  });

  it("refuses a folder without SHA256SUMS", () => {
    expect(() => downloadsFrom(INPUT, [folder("linux-x86_64", { "a.deb": 1 })])).toThrow("has no SHA256SUMS");
  });

  it("refuses a folder that isn't a platform", () => {
    expect(() => downloadsFrom(INPUT, [folder("plan9-mips", { "a.deb": 1 }, "")])).toThrow("isn't a platform key");
  });
});

// The website reads these lists (apps/site/src/data/downloads.ts, its own copy of the shape, since
// one app can't import another): its test fixture must be a list this file could have written.
describe("the website's fixture", () => {
  const fixture = JSON.parse(
    readFileSync(new URL("../../site/e2e/downloads.fixture.json", import.meta.url), "utf8"),
  ) as Record<string, { files: Record<string, unknown>[] } | null>;
  const files = Object.values(fixture).flatMap((list) => list?.files ?? []);

  it("names each file's kind as installerKind does", () => {
    expect(files.filter((file) => installerKind(String(file.name)) !== file.kind)).toEqual([]);
  });

  it("holds only the fields a listed file has", () => {
    const fields = new Set(["platform", "kind", "name", "size", "sha256", "url", "signature"]);
    expect(files.flatMap((file) => Object.keys(file)).filter((key) => !fields.has(key))).toEqual([]);
  });
});
