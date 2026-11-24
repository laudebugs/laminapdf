import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { DESKTOP_RELEASES, RELEASE_0_1_0 } from "../src/data/desktop-releases.ts";
import { WORDS } from "../src/data/downloads.ts";

// A release page speaks for the app, so its claims are held to the sources the repository checks:
// the download page's words, the desktop column of the capabilities page and the runbook.
const root = new URL("../../../", import.meta.url);
const read = (path: string) => readFileSync(new URL(path, root), "utf8");
const all = JSON.stringify(RELEASE_0_1_0);

describe("the desktop release notes", () => {
  it("has one page file for each release, at the address the updater's notes name", () => {
    for (const release of DESKTOP_RELEASES) {
      const page = read(`apps/site/src/pages/desktop/releases/${release.version}.astro`);
      expect(page, release.version).toContain(`RELEASE_${release.version.replaceAll(".", "_")}`);
    }
  });

  it("claims only what the capabilities page proves for the desktop app", () => {
    const capabilities = read("docs/capabilities/README.md");
    const proven = (id: string) =>
      new RegExp(`\\| ${id.replace(".", "\\.")} \\|[^\\n]*desktop round-trip`).test(capabilities);
    // Organise pages (C.01 insert, C.03 delete and duplicate, C.04 reorder, C.08 rotate, C.09 crop,
    // C.10 resize, C.13 reverse, I.10 labels) and properties (J.01–J.03, J.10).
    for (const id of ["C.01", "C.03", "C.04", "C.08", "C.09", "C.10", "C.13", "I.10", "J.01", "J.02", "J.03", "J.10"]) {
      expect(proven(id), id).toBe(true);
    }
  });

  it("gives the Linux graphics steps the download page gives", () => {
    expect(WORDS.firstOpen.linux).toContain("Settings › Graphics › Compatibility");
    expect(all).toContain("Settings › Graphics › Compatibility");
    expect(all).toContain("--safe-graphics");
  });

  it("says that macOS follows, that the store build is separate and that the product will be paid", () => {
    for (const text of ["macOS follows", "Mac App Store build is a separate install", "will be a paid product"]) {
      expect(all, text).toContain(text);
    }
  });

  it("marks the pricing sentence for the owner until the owner confirms it", () => {
    expect(RELEASE_0_1_0.pricing).toContain("[OWNER TO CONFIRM");
  });

  it("names the runbook's verification commands", () => {
    const runbook = read("docs/guidelines/desktop-release.md");
    for (const command of [
      "Get-AuthenticodeSignature",
      "gpg --verify",
      "sha256sum --check --ignore-missing",
      "gh attestation verify",
    ]) {
      expect(runbook, command).toContain(command);
      expect(all, command).toContain(command);
    }
  });

  it("promises no date", () => {
    expect(all).not.toMatch(/\bsoon\b|\bnext (week|month)\b|\b20\d\d\b/);
  });
});
