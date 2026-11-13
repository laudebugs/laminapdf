// The Lamina PDF website at https://laminapdf.com (todo W1b; website plan §4): static pages in the
// Paper look with the website's strict headers (ADR 0011 §4, public/_headers). After the build,
// @laminapdf/site-kit checks every page against the website's rules (ADR 0011 §3), fills the
// CSP's inline-script hashes into dist/_headers, marks every build but production's noindex, and
// writes robots.txt (production names the sitemap). The sitemap dates each page by its sources'
// last commit (@laminapdf/site-kit's lastmod.mjs).
//
// Build setting (environment variable): LAMINA_PDF_ENV=production for the production deploy. Any
// other value, or none (a local build, a PR preview), adds `X-Robots-Tag: noindex`.
import { existsSync } from "node:fs";
import { inlineScriptHashes, pageFiles, previewNoindex, siteChecks } from "@laminapdf/site-kit/integrations";
import { datesAvailable, lastCommitDate } from "@laminapdf/site-kit/lastmod";
import { isProduction, robots } from "@laminapdf/site-kit/robots";
import { defineConfig } from "astro/config";

/** Production's address: canonical links use it, previews too (they're noindex). */
const ORIGIN = "https://laminapdf.com";

/**
 * The page files, as `/src/pages/…` paths: the header links to a page of this site only once it
 * exists (@laminapdf/site-kit's nav.mjs). Read here, once per build, so no component imports every
 * page to list them; the docs site reads the same folder.
 */
const PAGES = pageFiles(new URL("./src/pages/", import.meta.url));

/**
 * What a page is made from, for its sitemap date: its file, and the components folder named
 * after it (`home` for the home page) when there is one.
 *
 * @param {string} file a page as `/src/pages/…`
 * @returns {URL[]}
 */
function pageSources(file) {
  const name = file.replace(/^\/src\/pages\//, "").replace(/\.astro$/, "");
  const components = new URL(`./src/components/${name === "index" ? "home" : name}/`, import.meta.url);
  return [new URL(`.${file}`, import.meta.url), ...(existsSync(components) ? [components] : [])];
}

/** Each page's last commit date by file, for the sitemap; none from a shallow checkout. */
const ROOT = new URL("../../", import.meta.url);
const LASTMOD = datesAvailable(ROOT, isProduction(process.env.LAMINA_PDF_ENV))
  ? Object.fromEntries(PAGES.map((file) => [file, lastCommitDate(ROOT, pageSources(file))]))
  : {};

// A default export because that is how Astro reads its configuration.
export default defineConfig({
  site: ORIGIN,
  // Telemetry is off in the package scripts (ASTRO_TELEMETRY_DISABLED).
  devToolbar: { enabled: false },
  // Every style sheet is a file, since the CSP's style-src is 'self' (ADR 0011 §4).
  build: { inlineStylesheets: "never" },
  vite: {
    // Every asset is a file too: font-src 'self' refuses fonts inlined as data: URLs.
    build: { assetsInlineLimit: 0 },
    define: {
      __SITE_PAGES__: JSON.stringify(PAGES),
      __SITE_LASTMOD__: JSON.stringify(LASTMOD),
    },
  },
  integrations: [
    siteChecks({ origin: ORIGIN }),
    inlineScriptHashes,
    previewNoindex(process.env.LAMINA_PDF_ENV),
    robots({ env: process.env.LAMINA_PDF_ENV, sitemap: new URL("/sitemap.xml", ORIGIN) }),
  ],
});
