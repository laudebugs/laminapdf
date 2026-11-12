/** The website's page files (apps/site/src/pages), as `/src/pages/…` paths, read by astro.config.mjs. */
declare const __SITE_PAGES__: readonly string[];
/** The version the docs describe and whether they are the released docs (plugins/release.mjs). */
declare const __DOCS_RELEASE__: { readonly version: string; readonly release: boolean };
/** Whether the build writes the guide PDF (plugins/guide.mjs), so pages link it only then. */
declare const __GUIDE_PDF__: boolean;
