import { readdirSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { DESKTOP_RELEASES, notesPath, RELEASE_0_1_0 } from "../src/data/desktop-releases.ts";
import { BETA_STATUS, WORDS } from "../src/data/downloads.ts";

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
    // Only what was seen working: delete and duplicate (C.03), move (C.04), rotate (C.08) and the
    // title, author, subject and keywords (J.01).
    for (const id of ["C.03", "C.04", "C.08", "J.01"]) {
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

  it("renders no placeholder: the owner's marker appears in no source of the site", () => {
    const sources = (dir: URL): string[] =>
      readdirSync(dir, { withFileTypes: true }).flatMap((entry) =>
        entry.isDirectory() ? sources(new URL(`${entry.name}/`, dir)) : [new URL(entry.name, dir).pathname],
      );
    const found = sources(new URL("../src/", import.meta.url)).filter((file) =>
      /OWNER TO CONFIRM/i.test(readFileSync(file, "utf8")),
    );
    expect(found).toEqual([]);
  });

  it("says the product will be paid, and shows what the beta is only once the owner sets it", () => {
    expect(RELEASE_0_1_0.pricing).toBe(WORDS.pricing);
    expect(WORDS.pricing).toContain("will be a paid product");
    expect(RELEASE_0_1_0.status).toBe(BETA_STATUS);
  });

  it("names the page's route as the notes address the publish step and the app use", () => {
    for (const release of DESKTOP_RELEASES) {
      const url = `https://laminapdf.com${notesPath(release.version)}`;
      expect(url).toBe(`https://laminapdf.com/desktop/releases/${release.version}/`);
      const action = read(".github/actions/desktop-publish/action.yml");
      expect(action).toContain("https://laminapdf.com/desktop/releases/$VERSION/");
      expect(action).toContain("apps/site/src/pages/desktop/releases/$VERSION.astro");
      const commands = read("apps/desktop/src-tauri/src/update/commands.rs");
      expect(commands).toContain('const NOTES_PREFIX: &str = "https://laminapdf.com/desktop/releases/";');
    }
  });

  it("names the desktop app's own Clear recent buttons", () => {
    expect(read("packages/ui/src/messages.ts")).toContain('"recent.clear": "Clear list"');
    expect(read("apps/desktop/src/settings.tsx")).toContain('const CLEAR_RECENT = "Clear recent files"');
    expect(all).toContain("Clear list");
    expect(all).toContain("Clear recent files");
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

  it("lists the features the desktop app lacks as known issues", () => {
    const text = RELEASE_0_1_0.known.items.join(" ");
    for (const word of [
      "Find",
      "sidebar",
      "outline",
      "Reader mode",
      "Back and Forward",
      "colour theme",
      "light or dark",
    ]) {
      expect(text, word).toContain(word);
    }
  });

  it("doesn't advertise what wasn't seen to work: removing metadata, insert, crop, resize, labels", () => {
    const text = JSON.stringify(RELEASE_0_1_0.does);
    for (const word of ["removes the values", "inserts", "crops", "resizes", "page labels"]) {
      expect(text, word).not.toContain(word);
    }
    expect(RELEASE_0_1_0.known.items.join(" ")).toContain("old value can still be found in earlier parts of the file");
  });
});
