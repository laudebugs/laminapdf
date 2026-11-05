# Plan: the website at laminapdf.com, and the reader at app.laminapdf.com (Round W)

- **Status:** Draft
- **Date:** 2026-11-05
- **Asked for by:** the project owner (2026-11-05): "the app rests on app.laminapdf.com and laminapdf.com is the landing page"
- **Changes:** [ADR 0011 §1–§4](../decisions/0011-cloudflare-topology-and-privacy.md#1-topology), which put the reader at the apex: its hosts (§1), its `#url=` links (§2), the origin its guarantees name (§3), and its headers and the HSTS preload (§4); [W1a](../todos/ready/W1a-domain-map.md) amends them
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
  | `/` | home: what Lamina PDF is, the reader, the SDK, the desktop app, the privacy promises | [W1c](../todos/ready/W1c-home-page.md) |
  | `/download` | desktop downloads, detecting the visitor's system | [D7b](../todos/ready/D7b-settings-renderer-switch-and-download-page.md) |
  | `/privacy` | the privacy page, every statement tied to ADR 0011 | [C7c](../todos/ready/C7c-performance-gates-and-launch.md) |
  | `/pricing` | plans, prices and the fair-pricing promises | [U5a](../todos/ready/U5a-pricing-and-promises-page.md) |
  | `/performance` | the reader benchmark | [U5b](../todos/ready/U5b-reader-benchmark.md) |
  | `/desktop/releases/<version>` | desktop release notes, which the updater links to | [D5a](../todos/ready/D5a-update-worker-and-adr-0014.md), [D7c](../todos/ready/D7c-desktop-beta-release.md) |
  | `/keys/lamina-release.asc` | the public key that signs the Linux packages | [D6a](../todos/ready/D6a-signing-runbook-and-owner-setup.md) |

- **Look.** Paper, through `@laminapdf/tokens` and the self-hosted fonts of `@laminapdf/fonts` ([U6b](../todos/ready/U6b-self-hosted-fonts.md)), the one copy that the reader, the desktop app and both sites share. The header is §7.39's: the wordmark, the links Download · Docs · Pricing · Performance · Privacy, and "Open the web app". [W1d](../todos/ready/W1d-one-site-header.md) gives the website and the docs site one header, built from one list of links.
- **Privacy.** The website keeps the reader's promises that apply to a page of text. It sets no cookies, loads nothing from a third party and sends no analytics. ADR 0011's amendment states these rules and the website's headers, and W1b checks every built page against them.
- **Before the reader launches.** The website can go live first, about the SDK, the library and the docs. "Open the web app" appears once C7c deploys the reader to production, behind one build setting that C7c turns on.
- **HSTS preload.** ADR 0011 §4 holds the preload submission until the reader serves the apex. After the move, the website serves the apex, so the preload follows W1b's first production deploy, once every subdomain is known to be HTTPS-only: a one-line PR adds `preload` to the website's `Strict-Transport-Security` header, which the preload list requires, and the owner then submits the domain.

## 5. Risks

- **Plans and todos name the old hosts.** ADR 0011, the roadmap, the web reader plan, C4c and C4d say the reader is at `laminapdf.com` or `staging.laminapdf.com`, and C4c's privacy tests and C4d's deploy config would carry those names into code. W1a changes every one of them before C4b's app exists. Done todos keep their history.
- **Two Astro sites drift apart.** Both sites share the tokens, the fonts and W1d's header. Anything else they both need moves to a shared workspace package the first time a second copy would appear, per the [maintainability guideline](../guidelines/maintainability.md).
- **Home page copy makes promises.** "No upload" and "works offline" are claims a reader can test. W1c ties each claim on the page to the ADR rule or test behind it, and the owner approves the wording.

## 6. Todos

| ID | Todo | Depends on |
| --- | --- | --- |
| [W1a](../todos/ready/W1a-domain-map.md) | The domain map: amend ADR 0011, move the reader to `app.laminapdf.com` in every plan, todo and design doc | — |
| [W1b](../todos/ready/W1b-site-scaffold-and-deploys.md) | `apps/site`: the Astro site in Paper, its headers, a preview per PR and production at `laminapdf.com` | W1a, U6b |
| [W1c](../todos/ready/W1c-home-page.md) | The home page: design, copy approved by the owner, metadata and share images | W1b |
| [W1d](../todos/ready/W1d-one-site-header.md) | One header for the website and the docs site | W1b |

C4c and C4d depend on W1a, because they turn the hosts into code. C7c, D7b, U5a and U5b depend on W1b, because their pages move to the website. W1b depends on U6b for the fonts.

## 7. Open questions

1. **Launch the website before the reader?** Recommendation: yes. It explains the SDK and links the docs today, and "Open the web app" waits for C7c (§4).
2. **The home page's words.** The owner writes or approves the headline and every promise in W1c's PR.

## 8. To-dos

- [ ] Amend ADR 0011 and move the reader to `app.laminapdf.com` everywhere ([W1a](../todos/ready/W1a-domain-map.md))
- [ ] Serve `apps/site` at `laminapdf.com`, with previews per PR ([W1b](../todos/ready/W1b-site-scaffold-and-deploys.md))
- [ ] Publish the home page ([W1c](../todos/ready/W1c-home-page.md))
- [ ] Share one header between the website and the docs site ([W1d](../todos/ready/W1d-one-site-header.md))

With these four todos done, `laminapdf.com` tells a visitor what Lamina PDF is and where each part lives, and the reader launches at `app.laminapdf.com` on its own origin.
