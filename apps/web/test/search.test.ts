import { copyFile, mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { afterEach, describe, expect, it } from "vitest";
import { writeSearchRules } from "../plugins/search.ts";

const HEADERS = new URL("../public/_headers", import.meta.url);
let dir = "";

/** A built folder holding the reader's own `_headers`, after the search rules of `environment`. */
async function built(environment: string | undefined): Promise<{ robots: string; headers: string }> {
  dir = await mkdtemp(join(tmpdir(), "lamina-web-search-"));
  await copyFile(HEADERS, join(dir, "_headers"));
  await writeSearchRules(pathToFileURL(`${dir}/`), environment);
  return {
    robots: await readFile(join(dir, "robots.txt"), "utf8"),
    headers: await readFile(join(dir, "_headers"), "utf8"),
  };
}

afterEach(async () => {
  if (dir !== "") await rm(dir, { recursive: true, force: true });
});

describe("what the reader tells search engines", () => {
  it("production lets crawlers in, names no sitemap and adds no noindex", async () => {
    const { robots, headers } = await built("production");
    expect(robots).toBe("User-agent: *\nAllow: /\n");
    expect(headers).toBe(await readFile(HEADERS, "utf8"));
  });

  it.each([["staging"], [undefined]])(
    "%s lets crawlers in so they read noindex on every response",
    async (environment) => {
      const { robots, headers } = await built(environment);
      expect(robots).toBe("User-agent: *\nAllow: /\n");
      expect(headers).toMatch(/^\/\*\n {2}X-Robots-Tag: noindex$/m);
    },
  );
});
