// public/_headers against ADR 0011 §4: every response of the website carries the ADR's headers
// with exactly the two differences the ADR gives the website. A change to the headers is an
// amendment to the ADR, so this test fails until both say the same.

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const ADR = readFileSync(
  new URL("../../../docs/decisions/0011-cloudflare-topology-and-privacy.md", import.meta.url),
  "utf8",
);
const HEADERS = readFileSync(new URL("../public/_headers", import.meta.url), "utf8");

/** The ADR's header set: the first code block of §4, one header per line. */
function adrHeaders(): string[] {
  const section = ADR.slice(ADR.indexOf("### 4. Security headers"));
  const block = section.match(/```\n([\s\S]*?)\n```/)?.[1];
  if (block === undefined) throw new Error("ADR 0011 §4 has no code block of headers");
  return block.split("\n");
}

/** The headers of one block of public/_headers (`/*`, `/_astro/*`), one per line. */
function block(path: string): string[] {
  const lines = HEADERS.split("\n");
  const start = lines.indexOf(path);
  if (start < 0) throw new Error(`public/_headers has no ${path} block`);
  const end = lines.findIndex((line, i) => i > start && !line.startsWith("  "));
  return lines.slice(start + 1, end < 0 ? undefined : end).map((line) => line.trim());
}

/** Replaces `from` with `to` in the line that names `header`, failing if `from` isn't there. */
function differ(headers: string[], header: string, from: string, to: string): string[] {
  return headers.map((line) => {
    if (!line.startsWith(`${header}:`)) return line;
    if (!line.includes(from)) throw new Error(`ADR 0011 §4's ${header} no longer has ${from}`);
    return line.replace(from, to);
  });
}

describe("public/_headers", () => {
  it("is ADR 0011 §4's set with the website's two differences", () => {
    let expected = adrHeaders();
    // script-src: the inline scripts' hashes, filled in after the build; no WebAssembly.
    expected = differ(
      expected,
      "Content-Security-Policy",
      "script-src 'self' 'wasm-unsafe-eval'",
      "script-src 'self' 'inline-script-hashes'",
    );
    // connect-src: the website fetches nothing remote.
    expected = differ(expected, "Content-Security-Policy", "connect-src 'self' https:", "connect-src 'self'");
    expect(block("/*")).toEqual(expected);
  });

  it("states both differences in the ADR", () => {
    expect(ADR).toContain("**`script-src 'self'` plus one `'sha256-…'` hash per inline script the build writes**");
    expect(ADR).toContain("**`connect-src 'self'`**: the website fetches nothing remote.");
  });

  it("keeps style-src 'self' and sets no cookie", () => {
    expect(block("/*").find((line) => line.startsWith("Content-Security-Policy:"))).toContain("style-src 'self';");
    expect(HEADERS).not.toMatch(/set-cookie/i);
  });

  it("caches Astro's hashed files for a year", () => {
    expect(block("/_astro/*")).toEqual(["Cache-Control: public, max-age=31536000, immutable"]);
  });
});
