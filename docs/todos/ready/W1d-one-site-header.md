# W1d — One header for the website and the docs site

| Field | Value |
| --- | --- |
| ID | `W1d` |
| Status | `ready` |
| Plan | [Website §4 The website](../../plans/website.md#4-the-website) |
| Round / milestone | Round W (website) — ships before M2 Web reader v0.1 |
| Depends on | [W1b](./W1b-site-scaffold-and-deploys.md) |
| Blocks | None |
| ADRs | — |
| Branch | `feat/w1d-site-header` |
| PR title | `feat(site): one header for the website and the docs site` |
| Size | S (100–300 lines) |
| PR | — |

## Goal

A visitor moving between `laminapdf.com` and `docs.laminapdf.com` sees the same header: the wordmark, Download · Docs · Pricing · Performance · Privacy, and "Open the web app". One list of links feeds both sites, so a new page is one edit.

## Context

- [Design language §7.39](../../design/paper/design-language.md#739-site-and-product-pages-proposed): the header's sizes, links and button.
- The docs site uses Starlight's own header today: `apps/docs/astro.config.mjs` overrides no component, and `apps/docs/src/components/` holds only `CodeTable.astro` and `Snippet.astro`. The website's header comes from W1b.

## Scope

**In scope**

- One header component and one list of links in a shared workspace package, used by `apps/site`'s layout and by a new Starlight header override in the docs site (Starlight's `components: { Header }` option), which keeps Starlight's search, theme switch and mobile menu. Each link names its host, so the docs site links to the website's pages by full URL and the website to the docs.
- A link appears only once its page exists, and "Open the web app" only with the build setting from the [plan](../../plans/website.md#4-the-website).
- The docs site's wordmark links to `https://laminapdf.com`.

**Out of scope**

- The pages behind the links (W1a names their todos).

## Tasks

- [ ] Claim: the first commit moves this file to `ongoing/`.
- [ ] Move the header and its links into the shared package; use it in both sites.
- [ ] Browser tests: keyboard order, the current page's underline, phone width, light and dark, axe.

## Acceptance criteria

- [ ] Both sites' previews show the same header, and a link added to the list appears on both.
- [ ] The docs site's search and theme switch still work beside the new header.

## Tests

The shared package's browser tests; both sites' post-build link checks.

## Docs to update

The shared package's README, `CLAUDE.md`'s layout, the [website plan §8](../../plans/website.md#8-to-dos).

## Definition of done

- [ ] Claimed: this file moved `ready/` → `ongoing/` in the work PR's first commit (Status `ongoing`, links updated)
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the package README
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
