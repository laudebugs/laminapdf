# W1b — `apps/site`: the website in Paper, its headers, a preview per PR and production at `laminapdf.com`

| Field | Value |
| --- | --- |
| ID | `W1b` |
| Status | `done` |
| Plan | [Website §4 The website](../../plans/website.md#4-the-website) |
| Round / milestone | Round W (website) — ships before M2 Web reader v0.1 |
| Depends on | [W1a](./W1a-domain-map.md), [U6a](./U6a-paper-tokens.md), [U6b](./U6b-self-hosted-fonts.md) |
| Blocks | [C7c](../ready/C7c-performance-gates-and-launch.md), [W1c](../ready/W1c-home-page.md), [W1d](./W1d-one-site-header.md), [W1e](./W1e-progress-snapshot.md), [D7b](../ready/D7b-settings-renderer-switch-and-download-page.md), [U5a](../ready/U5a-pricing-and-promises-page.md), [U5b](../ready/U5b-reader-benchmark.md) |
| ADRs | Implements 0011 §1 and §4 as W1a amends them; follows [0015](../../decisions/0015-monorepo-layout-and-tooling.md) (Astro for sites, pnpm workspace) and [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (pinned actions, least-privilege tokens) |
| Branch | `feat/w1b-site` |
| PR title | `feat(site): the website at laminapdf.com, with a preview per pull request` |
| Size | M (300–600 lines of site, config, checks and workflow) |
| PR | [#246](https://github.com/laudebugs/laminapdf/pull/246) |

## Goal

`laminapdf.com` serves a static Astro site in the Paper look, with the strict headers ADR 0011 gives the website, a preview URL on every PR that changes it, and production from `main`. The site starts with a placeholder home page; W1c writes the real one, and the todos of the moved pages add theirs.

## Context

- [Website plan §2–§4](../../plans/website.md#2-the-domain-map) and ADR 0011 as W1a amends it.
- The docs site is the model: `apps/docs/wrangler.toml` (custom domain route, `preview_urls`, no Worker code, `observability` off), `.github/workflows/docs.yml` (preview build without secrets, preview deploy with secrets and no PR code, production on `main`, one PR comment), `public/_headers` with inline-script hashes filled in after the build, `apps/docs/README.md` (the owner's setup).
- `@laminapdf/tokens` (U6a) and `@laminapdf/fonts` (U6b): Newsreader and Source Sans 3, the one shared copy. The docs site's own `apps/docs/src/fonts/` holds only Source Sans 3 and moves to that package too.

## Scope

**In scope**

- `apps/site` (`@laminapdf/site`, private): Astro static output, `@laminapdf/tokens` for colours, type and space, `@laminapdf/fonts` for Newsreader and Source Sans 3, a base layout with the §7.39 header and footer, a placeholder `/` and a 404 page.
- `apps/site/public/_headers`: ADR 0011 §4's headers for the website, written from one template, with `X-Robots-Tag: noindex` on preview builds only (`LAMINA_PDF_ENV`).
- Post-build checks (an Astro integration or `apps/site/scripts/`): every built page loads only same-origin files, sets no cookie, has a `<title>`, a description and a canonical URL on `https://laminapdf.com`, and every internal link resolves.
- `apps/site/wrangler.toml`: Worker `lamina-pdf-site`, custom domain `laminapdf.com`, `preview_urls = true`, `not_found_handling = "404-page"`, no Worker code, observability off.
- `.github/workflows/site.yml`, the docs workflow's pattern: preview per same-repository PR (`pr-<n>`, one comment), production on pushes to `main`, path-filtered inside the jobs. If the implementer's GitHub access can't push workflow files, the PR gives the file for the owner to commit.
- `apps/site/README.md`: the owner's setup (token scopes, the `www` redirect rule, the HSTS preload once production is live).
- `cargo xtask check ts` builds and checks the site. `cargo xtask changes` already classifies `apps/site/` as code, as it does `apps/docs/` (neither is on the docs allowlist), so site PRs run both lanes; leave `is_docs` as it is.

**Out of scope**

- The home page's content → [W1c](../ready/W1c-home-page.md); the shared header with the docs site → [W1d](./W1d-one-site-header.md).
- `/download`, `/privacy`, `/pricing`, `/performance` → their todos (W1a lists them).

## Tasks

- [x] Claim: the first commit moves this file to `ongoing/`.
- [x] Scaffold `apps/site` with the layout and placeholder pages; add it to the workspace and to `check ts`.
- [x] `_headers` template and the post-build checks, each with a test that fails on a bad page (a third-party script, a cookie, a missing title). The checks live in `@laminapdf/site-kit`, with the inline-script hashing moved there from the docs site.
- [ ] `wrangler.toml` and `site.yml`; preview deploy verified on this PR (once the owner commits the workflow). **Not yet:** `wrangler.toml` is in place, and the PR gives `site.yml` for the owner to commit, since this session can't push workflow files. Tick this once a PR shows its preview comment.
- [x] Owner actions in the PR: token scope on the zone, the `www.laminapdf.com` redirect rule, the first production deploy (also in [`apps/site/README.md`](../../../apps/site/README.md#what-the-owner-needs-to-do)).
- [x] `apps/site/README.md`; `CLAUDE.md`'s layout and commands.

## Acceptance criteria

- [ ] This PR gets a preview URL, and its pages carry the website's headers with `noindex`. **Not yet:** waits for the owner to commit `site.yml`. Locally, `wrangler dev` serves the build with the website's headers and `X-Robots-Tag: noindex`, and an unknown path answers 404.
- [x] The post-build checks fail on a page that loads a third-party file or sets a cookie.
- [ ] Lighthouse on the preview: 100 for accessibility and best practices on `/`. **Not yet:** waits for the preview. Locally, axe finds no violations on `/` and the 404 page in Chromium, light and dark, at 1280 and 375 px wide.

## Post-merge checks

- [ ] `https://laminapdf.com` serves the site from `main` without `noindex`; `https://www.laminapdf.com/x` answers 301 to `https://laminapdf.com/x`.
- [ ] Once every subdomain answers over HTTPS only, a one-line PR adds `preload` to the website's `Strict-Transport-Security` header (`max-age=63072000; includeSubDomains; preload`), and after it deploys the owner submits `laminapdf.com` to the HSTS preload list.

## Tests

The post-build checks' own tests (Vitest); the docs site's link check pattern for internal links.

## Docs to update

`apps/site/README.md`, `CLAUDE.md` (layout, commands, deploys), the [website plan §8](../../plans/website.md#8-to-dos).

## Risks and open questions

- **Two Astro sites.** Before copying anything from `apps/docs`, move it to a shared package; W1d does the header.
- **Workflow files.** When the owner has to commit `site.yml`, deploy the PR's preview by hand with the README's commands until it lands.

## Notes from the work

- **The shared build steps live in `@laminapdf/site-kit`.** The docs site's inline-script hashing moved there, and its link check now uses the same `pageExists` rule as the website: a file, a folder's `index.html` or `<path>.html`, the three forms the asset server answers. The docs site still gets `noindex` from `docs.yml`; it can switch to `previewNoindex` once `docs.yml`'s production build sets `LAMINA_PDF_ENV=production`.
- **The checks read markup, not text.** An attribute parser runs over each page with script bodies and comments removed, so a code sample or a JSON-LD string can't trip a rule. Links and loads resolve against the page's own address, as a browser resolves them.
- **A load by full address on the site's own origin fails the build.** It works in production but breaks on a preview, whose `'self'` is a workers.dev host.
- **Production has no workers.dev copy.** `workers_dev = false` with `preview_urls = true`, which Cloudflare's previews page documents. The README tells the owner where to turn on Version URLs if previews don't appear.
- **The header lists §7.39's links but shows only those whose pages exist**, from a page list `astro.config.mjs` reads once per build, so no component imports every page.
- **Lighthouse is unrun.** This environment blocked downloading it, and the preview waits for the owner's workflow commit. axe in Chromium found no violations.

## Definition of done

- [x] Claimed: this file moved `ready/` → `ongoing/` in the work PR's first commit (Status `ongoing`, links updated)
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the README and `CLAUDE.md` text
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
