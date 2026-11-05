// fonts.lock.json against the files it describes: every WOFF2 file, every licence, the tool pins.

import { createHash } from "node:crypto";
import { readdirSync, readFileSync } from "node:fs";
import { fromRoot } from "@laminapdf/test-utils/root";
import { describe, expect, it } from "vitest";
import { lock, packagePath } from "./helpers.ts";

const sha256 = (data: Uint8Array) => createHash("sha256").update(data).digest("hex");

describe("fonts.lock.json", () => {
  it("lists every WOFF2 file in files/ with its hash and size", () => {
    const { outputs } = lock();
    const files = readdirSync(packagePath("files")).map((name) => `files/${name}`);
    expect(files.sort()).toEqual(Object.keys(outputs).sort());
    for (const [file, output] of Object.entries(outputs)) {
      const data = readFileSync(packagePath(file));
      expect({ file, sha256: sha256(data), bytes: data.length }).toEqual({
        file,
        sha256: output.sha256,
        bytes: output.bytes,
      });
    }
  });

  it("ships each source's licence unchanged in LICENSES/", () => {
    const licences: Record<string, [string, string]> = {
      "source-sans-3": ["package/LICENSE.md", "SourceSans3-OFL.txt"],
      newsreader: ["package/LICENSE", "Newsreader-OFL.txt"],
      caveat: ["package/LICENSE", "Caveat-OFL.txt"],
    };
    for (const source of lock().sources) {
      const [member, file] = licences[source.id] ?? ["", ""];
      expect(sha256(readFileSync(packagePath(`LICENSES/${file}`)))).toBe(source.files[member]);
    }
  });

  it("records the tool versions pinned in tools/fonts/requirements.txt", () => {
    const pins = Object.fromEntries(
      readFileSync(fromRoot("tools/fonts/requirements.txt"), "utf8")
        .split("\n")
        .filter((line) => line.includes("=="))
        .map((line) => line.split("==")),
    );
    expect(lock().tools).toEqual(pins);
  });

  it("renames the only source that reserves a font name", () => {
    const reserved = lock().sources.filter((s) => s.reservedFontName !== null);
    expect(reserved.map((s) => [s.id, s.reservedFontName])).toEqual([["source-sans-3", "Source"]]);
    const families = new Set(Object.values(lock().outputs).map((o) => o.family));
    expect([...families].filter((family) => family.includes("Source"))).toEqual([]);
  });
});
