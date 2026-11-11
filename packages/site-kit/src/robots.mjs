// The one robots.txt policy for every Lamina PDF host: the website, the docs site and the web
// reader (the discoverability plan, §4). Every crawler may read every page, training crawlers
// included (the owner's choice, 2026-11-11). Production names its sitemap. Other builds don't block
// crawling either: they send `X-Robots-Tag: noindex` (markNoindex), and a crawler only reads that
// header on pages robots.txt lets it fetch, so a blocked staging page could stay in an index.
// Node-only, at build time: the Astro sites and the reader's Vite build both call it.
import { writeFile } from "node:fs/promises";

/**
 * The robots.txt for a build.
 *
 * @param {{ production: boolean, sitemap?: string | URL }} build `sitemap`: the sitemap's address,
 *   named only in production (a preview's sitemap would list the production host's pages)
 * @returns {string}
 */
export function robotsTxt({ production, sitemap }) {
  const rules = "User-agent: *\nAllow: /\n";
  if (!production || sitemap === undefined) return rules;
  return `${rules}\nSitemap: ${new URL(sitemap).href}\n`;
}

/**
 * What is wrong with a built robots.txt: no build may block crawling (crawlers must read `noindex`
 * outside production), and production must name its sitemap when the site has one.
 *
 * @param {string} text the built robots.txt
 * @param {{ production: boolean, sitemap: boolean }} build `sitemap`: whether the site has one
 * @returns {string[]}
 */
export function robotsProblems(text, { production, sitemap }) {
  const problems = [];
  if (/^\s*Disallow:\s*\/\s*$/im.test(text)) problems.push("robots.txt blocks crawling");
  const named = /^\s*Sitemap:/im.test(text);
  if (production && sitemap && !named) problems.push("robots.txt names no sitemap");
  if (!production && named) problems.push("robots.txt names a sitemap outside production");
  return problems;
}

/**
 * Writes `dir`/robots.txt (a build's output folder, a file: URL) from {@link robotsTxt}.
 *
 * @param {URL} dir
 * @param {{ production: boolean, sitemap?: string | URL }} build
 * @returns {Promise<void>}
 */
export async function writeRobotsTxt(dir, build) {
  await writeFile(new URL("robots.txt", dir), robotsTxt(build));
}

/**
 * An Astro site's robots.txt, written after the build; the reader's Vite build calls
 * {@link writeRobotsTxt} itself.
 *
 * @param {{ production: boolean, sitemap?: string | URL }} build
 * @returns {import("astro").AstroIntegration}
 */
export function robots(build) {
  return { name: "lamina-robots", hooks: { "astro:build:done": ({ dir }) => writeRobotsTxt(dir, build) } };
}
