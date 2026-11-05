# @laminapdf/site-kit

This package holds what Lamina PDF's two Astro sites share: the website (`apps/site`, laminapdf.com) and the docs site (`apps/docs`, docs.laminapdf.com). It has two parts, both used at build time only: the build steps that check and finish each site's output, and the site header with its one list of links. It is private to the workspace, and the sites import its source files directly.

## The site header

Both sites show the same header, from design language [§7.39](../../docs/design/paper/design-language.md#739-site-and-product-pages-proposed): the wordmark, the links Download · Docs · Pricing · Performance · Privacy, and "Open the web app".

- **One list of links.** `src/nav.mjs` lists the links once. `siteNav(pages, here)` resolves them for the site and page being built: a link to the other site is a full address, a link on the same site is a path, and the current page or section gets `aria-current`.
- **A link appears once its page exists.** Each site's `astro.config.mjs` reads the website's `src/pages/` folder with `pageFiles()` and passes it to the pages as `__SITE_PAGES__`. The todo that writes `/privacy/` therefore adds its link on both sites by adding the page.
- **"Open the web app"** appears on both sites once `APP_LAUNCHED` in `src/nav.mjs` is `true`. The reader's launch todo sets it.
- **One component.** `src/SiteHeader.astro` renders the row; each site wraps it in its own header element. The docs site puts Starlight's search, social icons and theme switch in its `tools` slot (`apps/docs/src/components/Header.astro`, Starlight's `Header` override).
- **One style sheet.** `src/header.css` draws it from the Paper tokens, which each site loads first. A site sizes it with variables: the website keeps §7.39's 64 px bar, at most 1120 px wide. The docs site fits it into Starlight's 56 px bar with a 22 px wordmark, §7.39's smaller variant, and hides the links below Starlight's 50 rem breakpoint, as Starlight hides its own header items.

## The build steps

`src/integrations.mjs` holds the Astro integrations that run after a build, on the files in `dist/`:

- **`siteChecks({ origin })`** fails the website's build when a page breaks [the website's privacy rules](../../docs/decisions/0011-cloudflare-topology-and-privacy.md#3-the-v01-privacy-guarantees-testable). A page breaks them when it loads a file from another origin or one the build didn't write, links to a missing page, has an inline style, sets a cookie or sends a beacon. It also fails when a page lacks a title, a description or a canonical URL. The [website's README](../../apps/site/README.md#what-every-build-checks) lists each rule.
- **`inlineScriptHashes`** puts the SHA-256 of each inline script into the CSP in `dist/_headers`, for both sites.
- **`previewNoindex(env)`** adds `X-Robots-Tag: noindex` to every build except production's (`LAMINA_PDF_ENV=production`).
- **`pageFiles(dir)`** lists a pages folder for the header, and **`pageExists(dir, path)`** says whether a build serves a path. The docs site's link check uses `pageExists` too.

## Tests

```sh
pnpm --filter @laminapdf/site-kit test                                         # the build steps and the links (Node)
pnpm --filter @laminapdf/site-kit exec vitest run -c vitest.browser.config.ts  # the header in Chromium, Firefox and WebKit
```

The browser tests render the header with Astro's container API in Node, before the tests start (`test/browser/render.ts`). They then check it in each browser, in light and dark: the tab order, the current page's underline, phone width and axe. `cargo xtask pdf-runtimes chromium|firefox|webkit` runs them on every pull request.
