# Plan: the website at laminapdf.com, and the reader at app.laminapdf.com (Round W)

- **Status:** Draft
- **Date:** 2026-11-05
- **Asked for by:** the project owner (2026-11-05): "the app rests on app.laminapdf.com and laminapdf.com is the landing page"; then, for the home page: "the home page will contain the roadmap for now, including a dashboard on what's coming up next", "this dashboard will be updated from time to time", and "the landing page will also contain information on the packages"
- **Changes:** [ADR 0011 §1–§4](../decisions/0011-cloudflare-topology-and-privacy.md#1-topology), which put the reader at the apex; [W1a](../todos/done/W1a-domain-map.md) amends its hosts (§1), its `#url=` links (§2), the origin its guarantees name (§3), and its headers and HSTS preload (§4)
- **Builds on:**
  - the [web reader plan](web-app.md) (C4's deploys, C7's launch pages)
  - the [Paper design plan](paper-design.md) and [design language §7.39](../design/paper/design-language.md#739-site-and-product-pages-proposed) (the site's header and product pages)
  - the docs site (`apps/docs`), whose Workers setup the website copies
- **Todos:** Round W on the [todo board](../todos/README.md); [§6](#6-todos) lists them.

## 1. Goal

Someone who types `laminapdf.com` today would land in a PDF reader with no word about what Lamina PDF is. The project now ships an SDK, a Rust library and a docs site, with a web reader and a desktop app to follow, so the apex needs a page that explains all five. The download, pricing, privacy and performance pages in the existing plans need that page too, because each assumes a site header with links between them.

Round W puts a static website at `laminapdf.com` and moves the reader to `app.laminapdf.com`. The reader has never been deployed, so the move costs a set of edits to plans and todos, with no redirect to keep and no link to break.

## 2. The domain map

| Host | Serves | Worker | Deployed from |
| --- | --- | --- | --- |
| `laminapdf.com` | the website (`apps/site`) | `lamina-pdf-site` | `main`, with a preview per PR |
| `www.laminapdf.com` | a 301 to `laminapdf.com`, path and query kept | none (a Cloudflare redirect rule) | the owner sets it once |
| `app.laminapdf.com` | the web reader (`apps/web`) | `lamina-pdf-web` | `web-v*` release tags only (C4d, C7c) |
| `staging-app.laminapdf.com` | the reader's staging | `lamina-pdf-web-staging` | `main` (C4d) |
| `docs.laminapdf.com` | the docs site, unchanged | `lamina-pdf-docs` | `main` |
| `updates.` and `downloads.laminapdf.com` | the desktop updater and downloads, unchanged | ADR 0014 (D5a) | D5a, D6b |

The reader's staging host was `staging.laminapdf.com`. It becomes `staging-app.laminapdf.com` so that its name says which product it stages, following `staging-downloads.laminapdf.com`. A first-level name also stays inside the zone's universal certificate, which covers the apex and `*.laminapdf.com` but no deeper names. Nothing uses `staging.laminapdf.com` after the move.

The website deploys from `main` like the docs site, because it holds pages, not the reader's code. Pages that make a promise (privacy, pricing) still need the owner's approval in their PR before they merge.

## 3. Why the reader gets its own origin

Separate origins keep the reader's local data apart from the website. Browsers scope storage, service workers and permissions by origin, so the split brings three gains:

- **The reader's data stays with the reader.** The OPFS cache, the recent list and the settings ([ADR 0011 §3](../decisions/0011-cloudflare-topology-and-privacy.md#3-the-v01-privacy-guarantees-testable) item 9) live under `app.laminapdf.com`, where no website page can read or clear them.
- **The offline service worker covers only the reader.** C7b's worker caches the reader's shell. At the apex it would also capture every website page, and a stale cache would serve old pricing or privacy text.
- **Each origin gets the headers it needs.** The reader needs `'wasm-unsafe-eval'` to compile its WebAssembly and `connect-src https:` to fetch `#url=` documents. The website needs neither, so its policy can be stricter.

Remote documents open from `https://app.laminapdf.com/#url=…`. The fragment rule of ADR 0011 §2 holds unchanged on the new host. The website forwards no `#url=` fragments: the apex never served the reader, so no link expects it to.

## 4. The website

`apps/site` is a static Astro site on Workers static assets, built and deployed like `apps/docs`. It has no Worker code, so every request is answered by the asset server.

- **Pages.** The home page comes from W1c; the other pages come from the todos that already plan them, retargeted from `apps/web` or the docs site to `apps/site` by W1a:

  | Path | Page | Todo |
  | --- | --- | --- |
  | `/` | home: what Lamina PDF is, and for now the roadmap with a dashboard of what's coming next ([below](#the-home-page-for-now-the-roadmap)) | [W1c](../todos/ready/W1c-home-page.md), [W1e](../todos/ready/W1e-progress-snapshot.md) |
  | `/download` | desktop downloads, detecting the visitor's system | [D7b](../todos/ready/D7b-settings-renderer-switch-and-download-page.md) |
  | `/privacy` | the privacy page, every statement tied to ADR 0011 | [C7c](../todos/ready/C7c-performance-gates-and-launch.md) |
  | `/pricing` | plans, prices and the fair-pricing promises | [U5a](../todos/ready/U5a-pricing-and-promises-page.md) |
  | `/performance` | the reader benchmark | [U5b](../todos/ready/U5b-reader-benchmark.md) |
  | `/desktop/releases/<version>` | desktop release notes, which the updater links to | [D5a](../todos/ready/D5a-update-worker-and-adr-0014.md), [D7c](../todos/ready/D7c-desktop-beta-release.md) |
  | `/keys/lamina-release.asc` | the public key that signs the Linux packages | [D6a](../todos/ready/D6a-signing-runbook-and-owner-setup.md) |

- **Look.** Paper, through `@laminapdf/tokens` and the self-hosted fonts of `@laminapdf/fonts` ([U6b](../todos/ready/U6b-self-hosted-fonts.md)), the one copy that the reader, the desktop app and both sites share. The header is §7.39's: the wordmark, the links Download · Docs · Pricing · Performance · Privacy, and "Open the web app". [W1d](../todos/ready/W1d-one-site-header.md) gives the website and the docs site one header, built from one list of links.
- **Privacy.** The website keeps the reader's promises that apply to a page of text. It sets no cookies, loads nothing from a third party and sends no analytics. ADR 0011's amendment states these rules and the website's headers, and W1b checks every built page against them.
- **Before the reader launches.** The website can go live first, with the roadmap, the SDK, the library and the docs. "Open the web app" appears once C7c deploys the reader to production, behind one build setting that C7c turns on.
- **HSTS preload.** ADR 0011 §4 held the preload submission until the reader served the apex. After the move, the website serves the apex, so the preload follows W1b's first production deploy, once every subdomain is known to be HTTPS-only. Then a one-line PR adds `preload` to the website's `Strict-Transport-Security` header, which the preload list requires, and the owner submits the domain.

### The home page, for now: the roadmap

Until the reader launches, the home page is mostly the roadmap. The page follows the build log the project keeps as a separate page today: where the work stands, what is being built and what comes next. Under a headline and one paragraph on what Lamina PDF is, the page shows:

- **The milestones** of the [roadmap](roadmap.md#milestones), M0 to M10, each with what it ships and whether its rounds are done, under way or not started.
- **Progress by round:** one bar per round of the todo board, split into done, in progress and ready.
- **In progress:** the todos an open pull request has claimed.
- **Next up:** the todos whose dependencies are all done, the board's "Can start now" list.
- **Recently merged:** the last ten pull requests merged into `main`, by title.
- **A first PDF in code:** a panel with three tabs. The TypeScript and Rust tabs show the same small program from the tested examples; the command-line tab shows the CLI's own example commands.
- **The packages:** every published package, its registry and what it holds (the docs site's install table), the two install lines (`npm install @laminapdf/sdk@alpha`, `cargo add laminapdf@0.2.0-alpha`), and the `laminapdf` command-line tool with its MCP server, marked unreleased until its binaries ship ([L4d](../todos/ready/L4d-cli-distribution.md)).
- **Where each part lives:** the docs, the packages and the Rust crate. The reader and the desktop app point at their milestones until they ship.
- **The snapshot's date,** so a visitor can tell how fresh the numbers are.

The dashboard reads one committed file, `apps/site/src/data/progress.json`. `cargo xtask progress` writes it from the todo files (with the board's own parser), the roadmap's milestone table, `git log` on `main` and the open pull requests. The build reads only that file, so it never needs the network or GitHub, and a preview shows exactly what production will.

Someone refreshes the dashboard from time to time: they run the command and merge the new file as an exempt PR, like a change to the todo board ([git workflow §2](../guidelines/git-workflow.md#pick-work-from-the-todo-board) gains that case). A scheduled workflow could open that PR every week, but it needs the owner to commit a workflow file, so it stays optional.

**The design** (chosen by the owner on 2026-11-05 from three samples): the whole page is one document page resting on the reader's paper ground, as a PDF sits in the reader, with Newsreader headings and Source Sans 3 text. The milestones run down a vertical timeline, the dashboard follows, and the packages sit in a table. The opening section carries the code panel. The site header stays §7.39's, the same on both sites.

The page shows titles and counts only. It names no person and links nothing in the repository, which is private. Rounds get plain names ("Core library", "Web reader") from one table in the site, and todo titles appear as the board writes them.

[W1e](../todos/ready/W1e-progress-snapshot.md) builds the command and the first snapshot; [W1c](../todos/ready/W1c-home-page.md) designs and builds the page. When the reader launches, the product leads the home page and the roadmap moves to a page of its own ([§7](#7-open-questions), question 3).

## 5. Risks

- **Plans and todos name the old hosts.** ADR 0011, the roadmap, the web reader plan, C4c and C4d say the reader is at `laminapdf.com` or `staging.laminapdf.com`, and C4c's privacy tests and C4d's deploy config would carry those names into code. W1a changes every one of them before C4b's app exists. Done todos keep their history.
- **Two Astro sites drift apart.** Both sites share the tokens, the fonts and W1d's header. Anything else they both need moves to a shared workspace package the first time a second copy would appear, per the [maintainability guideline](../guidelines/maintainability.md).
- **Home page copy makes promises.** "No upload" and "works offline" are claims a reader can test. Each claim on the page ties to the ADR rule or test behind it, and the owner approves the wording: W1c's in its PR, and C7c's privacy promises when it adds them with `/privacy`.
- **The dashboard goes stale.** A snapshot nobody refreshes tells visitors the project has stopped. The page always shows the snapshot's date, and a refresh is one command and one exempt PR.

## 6. Todos

| ID | Todo | Depends on |
| --- | --- | --- |
| [W1a](../todos/done/W1a-domain-map.md) | The domain map: amend ADR 0011, move the reader to `app.laminapdf.com` in every plan, todo and design doc | — |
| [W1b](../todos/ready/W1b-site-scaffold-and-deploys.md) | `apps/site`: the Astro site in Paper, its headers, a preview per PR and production at `laminapdf.com` | W1a, U6b |
| [W1c](../todos/ready/W1c-home-page.md) | The home page for now: what Lamina PDF is, the roadmap and the dashboard; design, copy approved by the owner, metadata and share images | W1b, W1e |
| [W1d](../todos/ready/W1d-one-site-header.md) | One header for the website and the docs site | W1b |
| [W1e](../todos/ready/W1e-progress-snapshot.md) | The progress snapshot: `cargo xtask progress` writes the dashboard's data from the board, the roadmap and `git log` | W1b |

C4c and C4d depend on W1a, because they turn the hosts into code. C7c, D7b, U5a and U5b depend on W1b, because their pages move to the website. W1b depends on U6b for the fonts.

## 7. Open questions

1. **Launch the website before the reader?** Recommendation: yes. It explains the SDK and links the docs today, and "Open the web app" waits for C7c (§4).
2. **The home page's words.** The owner writes or approves the headline, every promise and what the dashboard shows in W1c's PR.
3. **When does the roadmap leave the home page?** Recommendation: at the reader's launch (C7c). The home page then leads with the reader and the SDK, and the roadmap and its dashboard move to `/roadmap`, linked from the home page.

## 8. To-dos

- [x] Amend ADR 0011 and move the reader to `app.laminapdf.com` everywhere ([W1a](../todos/done/W1a-domain-map.md))
- [ ] Serve `apps/site` at `laminapdf.com`, with previews per PR ([W1b](../todos/ready/W1b-site-scaffold-and-deploys.md))
- [ ] Publish the home page with the roadmap and the dashboard ([W1c](../todos/ready/W1c-home-page.md))
- [ ] Share one header between the website and the docs site ([W1d](../todos/ready/W1d-one-site-header.md))
- [ ] Write the dashboard's data with one command ([W1e](../todos/ready/W1e-progress-snapshot.md))

With these five todos done, `laminapdf.com` tells a visitor what Lamina PDF is, where the work stands and what comes next, and the reader launches at `app.laminapdf.com` on its own origin.
