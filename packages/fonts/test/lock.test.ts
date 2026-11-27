// fonts.lock.json against the files it describes: every WOFF2 and PDF file, every licence, the tool pins.

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

  it("lists every TrueType file in pdf/ with its hash and size", () => {
    const { pdf } = lock();
    const files = readdirSync(packagePath("pdf")).map((name) => `pdf/${name}`);
    expect(files.sort()).toEqual(Object.keys(pdf).sort());
    for (const [file, output] of Object.entries(pdf)) {
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
      "noto-sans-arabic": ["package/LICENSE", "NotoSansArabic-OFL.txt"],
      "jetbrains-mono": ["jetbrains-mono-OFL.txt", "JetBrainsMono-OFL.txt"],
      "noto-sans-symbols-2": ["noto-sans-symbols-2-OFL.txt", "NotoSansSymbols2-OFL.txt"],
    };
    const sources = lock().sources;
    for (const source of sources) {
      // Newsreader's TTF has no copy of its own: it ships the licence of the same project.
      const owner = source.licenceFrom ?? source.id;
      const [member, file] = licences[owner] ?? ["", ""];
      const expected = sources.find((s) => s.id === owner)?.files[member];
      expect(sha256(readFileSync(packagePath(`LICENSES/${file}`)))).toBe(expected);
    }
  });

  it("pins google/fonts' own Newsreader licence, which shares its text with the shipped copy", () => {
    const source = lock().sources.find((s) => s.id === "newsreader-ttf");
    expect(source?.licenceFrom).toBe("newsreader");
    expect(source?.files["newsreader-ttf-OFL.txt"]).toMatch(/^[0-9a-f]{64}$/);
  });

  it("pins every Google Fonts file to one commit, in the URL that fetches it", () => {
    const pinned = lock().sources.filter((s) => s.commit !== undefined);
    expect(pinned.map((s) => s.id)).toEqual(["newsreader-ttf", "jetbrains-mono", "noto-sans-symbols-2"]);
    for (const source of pinned) {
      expect(source.commit).toMatch(/^[0-9a-f]{40}$/);
      for (const member of Object.keys(source.files)) {
        expect(source.urls?.[member]).toContain(`/google/fonts/${source.commit}/ofl/`);
      }
    }
  });

  it("ships a whole font in pdf/ for each family the picker adds, under an OFL source", () => {
    const { pdf, sources } = lock();
    for (const file of ["newsreader", "jetbrains-mono", "noto-sans-symbols-2"]) {
      expect(pdf[`pdf/${file}-regular.ttf`]?.coverage.split(",").length).toBeGreaterThan(20);
    }
    expect(sources.filter((s) => s.commit !== undefined).map((s) => s.licence)).toEqual([
      "OFL-1.1",
      "OFL-1.1",
      "OFL-1.1",
    ]);
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
    const families = new Set([...Object.values(lock().outputs), ...Object.values(lock().pdf)].map((o) => o.family));
    expect([...families].filter((family) => family.includes("Source"))).toEqual([]);
  });
});
