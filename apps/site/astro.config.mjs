// The Lamina PDF website at https://laminapdf.com (todo W1b; website plan §4): static pages in the
// Paper look with the website's strict headers (ADR 0011 §4, public/_headers). After the build,
// @laminapdf/site-kit checks every page against the website's rules (ADR 0011 §3), fills the
// CSP's inline-script hashes into dist/_headers, and marks every build but production's noindex.
//
// Build setting (environment variable): LAMINA_PDF_ENV=production for the production deploy. Any
// other value, or none (a local build, a PR preview), adds `X-Robots-Tag: noindex`.
import { readdirSync } from "node:fs";
import { inlineScriptHashes, previewNoindex, siteChecks } from "@laminapdf/site-kit/integrations";
import { defineConfig } from "astro/config";

/** Production's address: canonical links use it, previews too (they're noindex). */
const ORIGIN = "https://laminapdf.com";

/**
 * The page files, as `/src/pages/…` paths: the header links to a page of this site only once it
 * exists (src/nav.ts). Read here, once per build, so no component imports every page to list them.
 */
const PAGES = readdirSync(new URL("./src/pages/", import.meta.url), { recursive: true })
  .map((file) => `/src/pages/${String(file).replaceAll("\\", "/")}`)
  .filter((file) => /\.(astro|md)$/.test(file));

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
    define: { __SITE_PAGES__: JSON.stringify(PAGES) },
  },
  integrations: [siteChecks({ origin: ORIGIN }), inlineScriptHashes, previewNoindex(process.env.LAMINA_PDF_ENV)],
});
