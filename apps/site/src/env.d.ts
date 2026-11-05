/** The files in src/pages, as `/src/pages/…` paths, read by astro.config.mjs at build time. */
declare const __SITE_PAGES__: readonly string[];
/** Whether this is the production build (`LAMINA_PDF_ENV=production`), set by astro.config.mjs. */
declare const __PRODUCTION__: boolean;

/** An .astro component, as a TypeScript file imports it (the tests render them); Astro compiles it. */
declare module "*.astro" {
  import type { AstroComponentFactory } from "astro/runtime/server/index.js";

  const component: AstroComponentFactory;
  export default component;
}
