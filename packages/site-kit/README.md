# @laminapdf/site-kit

This package holds what Lamina PDF's two Astro sites share: the website (`apps/site`, laminapdf.com) and the docs site (`apps/docs`, docs.laminapdf.com). It has four parts: the build steps that check and finish each site's output, the site header with its one list of links, the select, and the states every control on both sites shares. All three run at build time, but the select also has a script that runs in the browser. It is private to the workspace, and the sites import its source files directly.

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

`src/MenuSelect.astro` is the sites' select: a button that opens a list of choices, each with its Paper icon. The list is design language [§7.26](../../docs/design/paper/design-language.md#726-menu)'s listbox popup, not the platform's own.

- **What it takes.** It takes a `label`, the `options` and the `value` shown at first. The label shows before the button (12/600 `text-3`), or with `hideLabel` only names it for assistive technology. Each option has a `value`, a `label` and an `icon`: a Paper icon's file from `@laminapdf/icons/svg/`, imported with `?raw`.
- **How it behaves.** `src/menu-select.ts` defines `<site-menu-select>`, ARIA's [select-only combobox](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/examples/combobox-select-only/). The button keeps the focus while the list is open. Down, Up, Enter, Space or a letter opens the list; Down, Up, Home, End and letters move through it; Enter, Space or Tab chooses; Escape closes it, as does a click outside. Choosing sets the element's `value` attribute and dispatches `change`, which bubbles. The list opens upwards when the window has no room below.
- **Showing a stored value.** Every element marked `data-for` shows only for its value. So a page's inline script can show a stored value before the first paint: it sets the element's `value` attribute and each of those elements' `hidden`.
- **Its look.** `src/menu-select.css` draws it from the Paper tokens, so it follows the site's `data-theme`. The button is §7.19's select field (32 px high, the field's fill and edge, the Paper chevron), with the value's icon before its label. The list is §7.26's listbox popup: 28 px items with their icon, the active one in `accent-tint` with an `accent` ring, the chosen one marked with the Check icon. On a touch screen the button and the items are 44 px high with 16 px text, as `@laminapdf/ui`'s fields are. It has no inline style, which the website's checks refuse.
- **Icons.** `decorativeIcon(svg, className)` in `src/icon.mjs` turns a Paper icon's file, imported with `?raw`, into HTML hidden from assistive technology. The select's icons and the docs site's example buttons use it.
- **Where it is used.** The docs site's theme switch (`apps/docs/src/components/ThemeSelect.astro`, Starlight's `ThemeSelect` override), in the header and the phone menu, with Light (Sun), Dark (Moon) and Auto (Laptop). It keeps Starlight's theme contract: the `<starlight-theme-select>` element, the values `dark`, `light` and `auto`, and the choice in `localStorage` under `starlight-theme`.

## The states

`src/states.css` (`@laminapdf/site-kit/states.css`) gives both sites [design language §7.0](../../docs/design/paper/design-language.md#70-shared-states)'s states, in the [motion plan's](../../docs/plans/motion-and-states.md#42-the-state-model) order. Each site loads it after the tokens and before its own style sheet, and the header and the select import it too.

- **The hand.** Every control that acts shows `--paper-cursor-action`, every link `--paper-cursor-link`, every disclosure's `summary` the hand too. A disabled control shows the arrow and the disabled text; a busy one (`aria-busy`, such as the docs runner's Run while it runs) shows the busy arrow and keeps its focus.
- **Hover and pressed.** A control takes the class `site-control`: hover and pressed are one translucent layer over its own fill, so a control sets its fill with `background-color`, not the `background` shorthand. Hover fades in over 80 ms and out over 150 ms, press shows at once, and hover applies only under a pointer that hovers (`@media (hover: hover) and (pointer: fine)`), so a tap leaves nothing lit. A control whose hover darkens its text sets `--site-hover: transparent`; the website's primary button swaps solid fills instead.
- **One focus ring.** 2 px of the focus colour, 2 px out, on both sites, with no radius of its own.
- **Reduced motion.** Under the system's preference only colours, fills, edges, shadows and opacity may fade, whatever a rule of the sites or a library transitions: a turn, a slide or a scale lands at once. Starlight's sidebar caret and Expressive Code's "Copied!" follow it.
- **Links.** A visited link looks like any other, and the current page is marked by `aria-current` with an underline.

The header's and the select's browser tests check their cursors, the hover layer and the pressed layer in Chromium, Firefox and WebKit. Each site's e2e suite checks every clickable element's cursor over its built pages with `cursorProblems` from `@laminapdf/test-utils/cursors`.

## The build steps

`src/integrations.mjs` holds the Astro integrations that run after a build, on the files in `dist/`:

- **`siteChecks({ origin })`** fails the website's build when a page breaks [the website's privacy rules](../../docs/decisions/0011-cloudflare-topology-and-privacy.md#3-the-v01-privacy-guarantees-testable). A page breaks them when it loads a file from another origin or one the build didn't write, links to a missing page, has an inline style, sets a cookie or sends a beacon. It also fails when a page lacks a title, a description or a canonical URL. The [website's README](../../apps/site/README.md#what-every-build-checks) lists each rule.
- **`inlineScriptHashes`** puts the SHA-256 of each inline script into the CSP in `dist/_headers`, for both sites. It leaves out data blocks, such as JSON-LD: a `<script>` whose `type` isn't JavaScript, `module` or `importmap` never runs, so the CSP doesn't apply to it (`isDataBlock`).
- **`previewNoindex(env)`** adds `X-Robots-Tag: noindex` to every build except production's (`LAMINA_PDF_ENV=production`).
- **`pageFiles(dir)`** lists a pages folder for the header, and **`pageExists(dir, path)`** says whether a build serves a path. The docs site's link check uses `pageExists` too.

`src/inline-styles.mjs` (`@laminapdf/site-kit/inline-styles`) lets the docs site's CSP refuse inline styles:

- **`inlineStylesToClasses({ scripted })`** turns every style attribute in the built pages into a class in one style sheet, its declarations `!important` so they win as the attribute did. It gives a style sheet it changes a new name, since the old content-hashed one stays cached for a year. It rewrites Expressive Code's selector on the style attribute to the classes, and fails the build on any other such selector or on a page left with an inline style. `scripted` names the styles a page's own script removes; each gets a class that the site's CSS styles instead.

`src/markup.mjs` (`@laminapdf/site-kit/markup`) reads built HTML for `inline-styles.mjs` and the checks in `integrations.mjs`. **`tokens(html)`** splits a page as a browser does, so text that looks like a tag inside an attribute, a script or a comment is never read as one. **`startTags(html)`** gives each start tag's attributes, which the website's checks read.

`src/robots.mjs` (`@laminapdf/site-kit/robots`) holds the one robots.txt policy of the website, the docs site and the web reader:

- **`robotsTxt({ production, sitemap })`** lets every crawler in, training crawlers included (the owner's choice, 2026-11-11), and names the sitemap in production. Other builds don't block crawlers either: they rely on `noindex`, which a crawler reads only on a page it may fetch.
- **`robots({ env, sitemap })`** writes it into an Astro build's `dist/` for the build's `LAMINA_PDF_ENV`; the reader's Vite build emits the same file (`apps/web/plugins/search.ts`).
- **`isProduction(env)`** is the one test of `LAMINA_PDF_ENV` that robots.txt and `markNoindex` share, so the two rules can't disagree.
- **`robotsProblems(text, build)`** names what is wrong with a built file; the docs site's release checks call it.

`src/structured-data.mjs` (`@laminapdf/site-kit/structured-data`) describes Lamina PDF to search engines, as JSON-LD in the website's and the docs' home pages:

- **`structuredData({ site, cargoToml })`** builds one schema.org graph: the publisher (`Organization`), the site (`WebSite`) and the code (`SoftwareSourceCode`). The licence, the repository and the home page come from the workspace's `Cargo.toml` (`projectFacts`), so they can't drift from what the crates publish. It carries no rating, review or offer: Lamina PDF has none, and Google treats invented ones as spam.
- **`structuredDataProblems(data)`** checks a graph's shape: known types, their required properties, `https:` addresses, references that resolve, and no rating, review or offer. Nothing is fetched.
- **`jsonLd(data)`** writes the graph for a `<script type="application/ld+json">`, with `<` escaped so no string can close the element.

`src/lastmod.mjs` (`@laminapdf/site-kit/lastmod`) dates the sitemaps' entries by their sources' last commit:

- **`commitDates(root)`** reads the history in one `git log` pass and returns a function: the committer date of the last commit to any of the given files or folders, or none for a file never committed.
- **`datesAvailable(root, production)`** refuses a shallow checkout in a production build, which would date every page by its one commit; other builds write no dates.
- **`withLastmod(xml, dateOf)`** adds a `<lastmod>` to each entry of a built sitemap. The website looks its dates up in `astro.config.mjs`; the docs site rewrites Starlight's sitemap after the build (`plugins/sitemap.mjs`).

## Tests

```sh
pnpm --filter @laminapdf/site-kit test                                         # the build steps and the links (Node)
pnpm --filter @laminapdf/site-kit exec vitest run -c vitest.browser.config.ts  # the header and the select in Chromium, Firefox and WebKit
```

The browser tests render the header and the select with Astro's container API in Node, before the tests start (`test/browser/render.ts`). They then check them in each browser, in light and dark: the header's tab order, current page's underline and phone width; the select's name, label, keyboard and pointer use, a click outside, focus ring and colours. Axe checks both, the select open and closed. `cargo xtask pdf-runtimes chromium|firefox|webkit` runs them on every pull request, along with the docs site's e2e suite, which tries the theme switch built on the select (`apps/docs/e2e/theme.e2e.test.ts`).
