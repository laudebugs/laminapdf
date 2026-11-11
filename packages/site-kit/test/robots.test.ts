// The one robots.txt policy: every build lets crawlers in, and only production names its sitemap.

import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { describe, expect, it } from "vitest";
import { isProduction, robots, robotsProblems, robotsTxt } from "../src/robots.mjs";

const SITEMAP = "https://laminapdf.com/sitemap.xml";

describe("robotsTxt", () => {
  it("lets every crawler in and names the sitemap in production", () => {
    expect(robotsTxt({ production: true, sitemap: SITEMAP })).toBe(`User-agent: *\nAllow: /\n\nSitemap: ${SITEMAP}\n`);
  });

  it("lets every crawler in outside production, so they read the noindex header, and names no sitemap", () => {
    expect(robotsTxt({ production: false, sitemap: SITEMAP })).toBe("User-agent: *\nAllow: /\n");
  });

  it("names no sitemap for a site that has none", () => {
    expect(robotsTxt({ production: true })).toBe("User-agent: *\nAllow: /\n");
  });

  it("passes its own check in every build", () => {
    for (const production of [true, false]) {
      expect(robotsProblems(robotsTxt({ production, sitemap: SITEMAP }), { production, sitemap: true })).toEqual([]);
    }
  });
});

describe("robotsProblems", () => {
  it("refuses a file that blocks crawling", () => {
    expect(robotsProblems("User-agent: *\nDisallow: /\n", { production: false, sitemap: true })).toEqual([
      "robots.txt blocks crawling",
    ]);
  });

  it("refuses production without its sitemap", () => {
    expect(robotsProblems("User-agent: *\nAllow: /\n", { production: true, sitemap: true })).toEqual([
      "robots.txt names no sitemap",
    ]);
  });

  it("refuses a sitemap outside production", () => {
    expect(
      robotsProblems(`User-agent: *\nAllow: /\nSitemap: ${SITEMAP}\n`, { production: false, sitemap: true }),
    ).toEqual(["robots.txt names a sitemap outside production"]);
  });
});

describe("robots", () => {
  it("writes the build's robots.txt into dist/ for its LAMINA_PDF_ENV", async () => {
    const dir = await mkdtemp(join(tmpdir(), "lamina-robots-"));
    try {
      for (const env of ["production", "staging", undefined]) {
        const hook = robots({ env, sitemap: SITEMAP }).hooks["astro:build:done"] as (options: {
          dir: URL;
        }) => Promise<void>;
        await hook({ dir: pathToFileURL(`${dir}/`) });
        expect(await readFile(join(dir, "robots.txt"), "utf8"), String(env)).toBe(
          robotsTxt({ production: isProduction(env), sitemap: SITEMAP }),
        );
      }
    } finally {
      await rm(dir, { recursive: true, force: true });
    }
  });

  it("counts only `production` as production", () => {
    expect([isProduction("production"), isProduction("staging"), isProduction(undefined)]).toEqual([
      true,
      false,
      false,
    ]);
  });
});
