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
// A style sheet imported from a remote URL (with or without url()), or an @font-face whose src is one.
const REMOTE_IMPORT = /@import\s+(?:url\(\s*)?["']?https?:/;
const REMOTE_SRC = /\bsrc:[^;}]*url\(\s*["']?https?:/;
const SCANNED = /\.(astro|cjs|css|html|js|json|jsx|mdx|mjs|scss|ts|tsx)$/;

/** Every font CDN reference in `text`. */
function cdnReferences(text: string): string[] {
  const hits = HOSTS.filter((host) => text.includes(host));
  if (REMOTE_IMPORT.test(text)) hits.push("@import url(http…)");
  if (REMOTE_SRC.test(text)) hits.push("src: url(http…)");
  return hits;
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
    expect(cdnReferences(`@import "${"https"}://example.com/font.css";`)).toEqual(["@import url(http…)"]);
    const face = `@font-face { font-family: X; src: url("${"https"}://example.com/x.woff2") format("woff2"); }`;
    expect(cdnReferences(face)).toEqual(["src: url(http…)"]);
  });
});
