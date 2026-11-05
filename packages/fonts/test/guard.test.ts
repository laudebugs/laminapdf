// No product file fetches a font from a third-party host (UI/UX plan principle 7): every face
// comes from @laminapdf/fonts. The design canvases in docs/design/ are mockups and are not scanned.

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fromRoot } from "@laminapdf/test-utils/root";
import { describe, expect, it } from "vitest";

// Built from parts so this file doesn't match itself.
const HOSTS = [
  ["fonts", "googleapis", "com"],
  ["fonts", "gstatic", "com"],
  ["use", "typekit", "net"],
].map((parts) => parts.join("."));
const REMOTE_IMPORT = /@import\s+url\(\s*["']?https?:/;
const SCANNED = /\.(astro|cjs|css|html|js|json|jsx|mdx|mjs|scss|ts|tsx)$/;

/** Every font CDN reference in `text`. */
function cdnReferences(text: string): string[] {
  const hits = HOSTS.filter((host) => text.includes(host));
  return REMOTE_IMPORT.test(text) ? [...hits, "@import url(http…)"] : hits;
}

describe("font CDN guard", () => {
  it("finds no font CDN in any tracked file under packages/ or apps/", () => {
    const tracked = execFileSync("git", ["ls-files", "packages", "apps"], {
      cwd: fromRoot("."),
      encoding: "utf8",
    })
      .split("\n")
      .filter((path) => SCANNED.test(path));
    expect(tracked.length).toBeGreaterThan(100);
    const offenders = tracked.flatMap((path) =>
      cdnReferences(readFileSync(fromRoot(path), "utf8")).map((hit) => `${path}: ${hit}`),
    );
    expect(offenders).toEqual([]);
  });

  it("catches a style sheet that loads a font from a CDN", () => {
    const page = `<link rel="stylesheet" href="https://${HOSTS[0]}/css2?family=Newsreader">`;
    expect(cdnReferences(page)).toEqual([HOSTS[0]]);
    const remote = `@import url("${"https"}://example.com/font.css");`;
    expect(cdnReferences(remote)).toEqual(["@import url(http…)"]);
  });
});
