// The sitemap's `<lastmod>` dates (todo W3d). Starlight writes the sitemap through
// @astrojs/sitemap and offers no hook for dates, so this step rewrites its files after the build:
// each page is dated by the last commit to its sources (@laminapdf/site-kit's lastmod.mjs). A page
// with no committed source, such as a one-page view, gets no date; the field is optional.
import { readdir, readFile, writeFile } from "node:fs/promises";
import { commitDates, datesAvailable, withLastmod } from "@laminapdf/site-kit/lastmod";
import { contentFiles } from "./agent-docs.mjs";

const ROOT = new URL("../../../", import.meta.url);
const CONTENT = new URL("src/content/docs/", new URL("../", import.meta.url));

/**
 * The files a route is built from: a code page's `docs/errors/<code>.md`, the TypeDoc reference's
 * `@laminapdf/sdk` sources, or a page's file under src/content/docs/. None for a route the
 * build makes from other pages.
 *
 * @param {string} route `guides/save-a-pdf/`, or `` for the home page
 * @param {Map<string, string>} content `contentFiles()`
 * @returns {URL[]}
 */
export function routeSources(route, content) {
  const code = /^reference\/codes\/([a-z0-9-]+)\/$/.exec(route);
  if (code) return [new URL(`docs/errors/${code[1]}.md`, ROOT)];
  if (route.startsWith("reference/sdk/")) return [new URL("packages/sdk/src/", ROOT)];
  const name = content.get(route === "" ? "index" : route.replace(/\/$/, ""));
  return name === undefined ? [] : [new URL(name, CONTENT)];
}

/**
 * Dates every page in the build's sitemaps (`sitemap-*.xml`, not the index) by its sources' last
 * commit. A production build fails from a shallow checkout; other builds leave the dates out.
 *
 * @param {{ production: boolean, site: string }} options
 * @returns {import("astro").AstroIntegration}
 */
export function sitemapDates({ production, site }) {
  return {
    name: "lamina-sitemap-dates",
    hooks: {
      "astro:build:done": async ({ dir }) => {
        if (!datesAvailable(ROOT, production)) return;
        const content = await contentFiles();
        const datesOf = commitDates(ROOT);
        const dateOf = (/** @type {string} */ loc) =>
          loc.startsWith(`${site}/`) ? datesOf(routeSources(loc.slice(site.length + 1), content)) : undefined;
        const sitemaps = (await readdir(dir)).filter((name) => /^sitemap-\d+\.xml$/.test(name));
        if (sitemaps.length === 0) throw new Error("sitemap dates: the build wrote no sitemap-<n>.xml");
        for (const name of sitemaps) {
          const file = new URL(name, dir);
          await writeFile(file, withLastmod(await readFile(file, "utf8"), dateOf));
        }
      },
    },
  };
}
