# @laminapdf/site-kit

This package holds what Lamina PDF's two Astro sites share: the website (`apps/site`, laminapdf.com) and the docs site (`apps/docs`, docs.laminapdf.com). It has three parts, all used at build time only: the build steps that check and finish each site's output, the site header with its one list of links, and the select. It is private to the workspace, and the sites import its source files directly.

## The site header

Both sites show the same header, from design language [§7.39](../../docs/design/paper/design-language.md#739-site-and-product-pages-proposed): the wordmark, the links Download · Docs · Pricing · Performance · Privacy, and "Open the web app".

- **One list of links.** `src/nav.mjs` lists the links once. `siteNav(pages, here)` resolves them for the site and page being built: a link to the other site is a full address, a link on the same site is a path, and the current page gets `aria-current="page"`. A link gets `aria-current="true"` on the pages below its page, such as Download on `/download/linux/`, and Docs gets it on every docs page.
- **A link appears once its page exists.** Each site's `astro.config.mjs` reads the website's `src/pages/` folder with `pageFiles()` and passes it to the pages as `__SITE_PAGES__`. The todo that writes `/privacy/` therefore adds its link on both sites by adding the page.
- **"Open the web app"** appears on both sites once `APP_LAUNCHED` in `src/nav.mjs` is `true`. The reader's launch todo sets it.
- **The version the pages name.** Pages write `{{version}}` where they name the version: install lines, docs.rs links. `withVersion(text, version)` in `src/version.mjs` fills it in with the fixed group's version. The docs site calls it on its pages and Markdown twins, and the website on the install lines it copies from the docs site's install page.
- **One component.** `src/SiteHeader.astro` renders the row; each site wraps it in its own header element. The docs site puts Starlight's search, social icons, theme switch and language switch in its `tools` slot (`apps/docs/src/components/Header.astro`, Starlight's `Header` override). `src/SiteNav.astro` renders the links alone, for the header and for the docs site's phone menu.
- **One style sheet.** `src/header.css` draws the header from the Paper tokens, which each site loads first. A site sizes it with variables: the website keeps §7.39's 64 px bar, at most 1120 px wide. The docs site fits it into Starlight's 56 px bar with a 22 px wordmark, §7.39's smaller variant.
- **The docs site's narrow widths.** Below Starlight's 50 rem breakpoint the header hides the links, as Starlight hides its own header items, and the docs site's phone menu lists them instead. Above it, the links scroll sideways when the bar is too narrow for them all.

## The select

`src/Select.astro` draws design language [§7.19](../../docs/design/paper/design-language.md#719-form-controls-consistent-with-the-page-input-and-note-field)'s Select for both sites: a native `<select>` in the Paper look. The platform opens its own list, so the keyboard and touch work as in any other select.

- **What it takes.** It takes a `label` and the `options`. The label shows above the field (12/600 `text-3`, as §7.19 places a field's label), or with `hideLabel` only names it for assistive technology. Each option has a `value`, a `label` and whether it starts `selected`. A site's script listens to the `<select>` inside.
- **Its look.** `src/select.css` draws it from the Paper tokens, so it follows the site's `data-theme`: the field's fill and edge, 32 px high, with the Paper chevron from `@laminapdf/icons/svg/`. On a touch screen it is 44 px high with 16 px text, as `@laminapdf/ui`'s fields are. It has no inline style, which the website's checks refuse. It repeats `@laminapdf/ui`'s `field.css`, which is React-only and layered and so can't be shared; a change to §7.19 changes both.
- **Icons.** `decorativeIcon(svg, className)` in `src/icon.mjs` turns a Paper icon's file, imported with `?raw`, into HTML hidden from assistive technology. The select's chevron and the docs site's example buttons use it.
- **Where it is used.** The docs site's theme switch (`apps/docs/src/components/ThemeSelect.astro`, Starlight's `ThemeSelect` override), in the header and the phone menu. It keeps Starlight's theme contract: the `<starlight-theme-select>` element, the values `dark`, `light` and `auto`, and the choice in `localStorage` under `starlight-theme`.

## The build steps

`src/integrations.mjs` holds the Astro integrations that run after a build, on the files in `dist/`:

- **`siteChecks({ origin })`** fails the website's build when a page breaks [the website's privacy rules](../../docs/decisions/0011-cloudflare-topology-and-privacy.md#3-the-v01-privacy-guarantees-testable). A page breaks them when it loads a file from another origin or one the build didn't write, links to a missing page, has an inline style, sets a cookie or sends a beacon. It also fails when a page lacks a title, a description or a canonical URL. The [website's README](../../apps/site/README.md#what-every-build-checks) lists each rule.
- **`inlineScriptHashes`** puts the SHA-256 of each inline script into the CSP in `dist/_headers`, for both sites.
- **`previewNoindex(env)`** adds `X-Robots-Tag: noindex` to every build except production's (`LAMINA_PDF_ENV=production`).

`src/robots.mjs` (`@laminapdf/site-kit/robots`) holds the one robots.txt policy of the website, the docs site and the web reader:

- **`robotsTxt({ production, sitemap })`** lets every crawler in, training crawlers included (the owner's choice, 2026-11-11), and names the sitemap in production. Other builds don't block crawlers either: they rely on `noindex`, which a crawler reads only on a page it may fetch.
- **`robots(build)`** writes it into an Astro build's `dist/`; **`writeRobotsTxt(dir, build)`** does the same for the reader's Vite build.
- **`robotsProblems(text, build)`** names what is wrong with a built file; the docs site's release checks call it.
- **`pageFiles(dir)`** lists a pages folder for the header, and **`pageExists(dir, path)`** says whether a build serves a path. The docs site's link check uses `pageExists` too.

## Tests

```sh
pnpm --filter @laminapdf/site-kit test                                         # the build steps and the links (Node)
pnpm --filter @laminapdf/site-kit exec vitest run -c vitest.browser.config.ts  # the header and the select in Chromium, Firefox and WebKit
```

The browser tests render the header and the select with Astro's container API in Node, before the tests start (`test/browser/render.ts`). They then check them in each browser, in light and dark: the header's tab order, current page's underline and phone width; the select's name, label, keyboard and pointer use, focus ring and colours; and axe for both. `cargo xtask pdf-runtimes chromium|firefox|webkit` runs them on every pull request, along with the docs site's e2e suite, which tries the theme switch built on the select (`apps/docs/e2e/theme.e2e.test.ts`).
