/** The files in src/pages, as `/src/pages/…` paths, read by astro.config.mjs at build time. */
declare const __SITE_PAGES__: readonly string[];

/** Each page's last commit date (ISO 8601) by its `__SITE_PAGES__` path, for the sitemap; empty from a shallow checkout. */
declare const __SITE_LASTMOD__: Readonly<Record<string, string | undefined>>;

/** An .astro component, as a TypeScript file imports it (the tests render them); Astro compiles it. */
declare module "*.astro" {
  import type { AstroComponentFactory } from "astro/runtime/server/index.js";

  const component: AstroComponentFactory;
  export default component;
}
