# W1c — The home page: what Lamina PDF is, in Paper, with the owner's words

| Field | Value |
| --- | --- |
| ID | `W1c` |
| Status | `ready` |
| Plan | [Website §4 The website](../../plans/website.md#4-the-website) |
| Round / milestone | Round W (website) — ships before M2 Web reader v0.1 |
| Depends on | [W1b](./W1b-site-scaffold-and-deploys.md) |
| Blocks | None |
| ADRs | Follows 0011 (each privacy claim on the page ties to its §3 guarantee) |
| Branch | `feat/w1c-home-page` |
| PR title | `feat(site): the home page` |
| Size | M (design, copy and 200–500 lines of site) |
| PR | — |

## Goal

A visitor to `laminapdf.com` learns in one screen what Lamina PDF is, then finds the part they came for: the SDK and the docs, the Rust library, the reader once it launches, and the desktop app once it ships. Every claim on the page is one a visitor can check, and the owner approves its words.

## Context

- [Website plan §4 and §7](../../plans/website.md#4-the-website): the pages, "Open the web app" waiting for C7c, the owner approving the copy.
- [Paper design plan](../../plans/paper-design.md), [design language §7.39](../../design/paper/design-language.md#739-site-and-product-pages-proposed) (the header, the large primary link), [screen inventory](../../design/paper/screens.md) (no home page yet).
- The docs site's [first-PDF quick start](../../../apps/docs/src/content/docs/start/first-pdf.mdx) for the install snippet; [docs-writing guideline](../../guidelines/docs-writing.md) for the copy.

## Scope

**In scope**

- The home page's design: a new screen in the inventory with its artboard, in Paper, light and dark, at phone and desktop widths.
- The page:
  - a headline and one paragraph on what Lamina PDF is;
  - the parts: the SDK (`npm install @laminapdf/sdk` and a five-line example from the tested examples), the Rust crate, the reader ("coming soon" until C7c), the desktop app ("coming soon" until D7c);
  - the privacy promises in plain words, each linking to `/privacy` once C7c publishes it;
  - links to the docs and, behind the build setting from the plan, "Open the web app".
- Share metadata: Open Graph and X card tags, a 1200 × 630 share image in Paper, a favicon set, `sitemap.xml` and `robots.txt` (production allows indexing, previews don't).

**Out of scope**

- The other pages (W1a names their todos); the shared header → [W1d](./W1d-one-site-header.md).

## Tasks

- [ ] Claim: the first commit moves this file to `ongoing/`.
- [ ] Design the screen; add it to `screens.md` and the design docs.
- [ ] Draft the copy with `/writing`; mark each promise `[OWNER TO APPROVE]` until the owner approves it in the PR.
- [ ] Build the page, the metadata and the share image; the install snippet comes from a tested example file, never typed by hand.
- [ ] `/writing-review` on the copy.

## Acceptance criteria

- [ ] The owner has approved the headline and every promise in the PR.
- [ ] Each privacy claim names its ADR 0011 guarantee in the PR description.
- [ ] Lighthouse on the preview, phone and desktop: 100 for accessibility, best practices and SEO; axe finds nothing in light and dark.
- [ ] The share image renders in the X and LinkedIn card validators against the preview.

## Tests

The site's post-build checks (W1b); a test that the install snippet matches its example file.

## Docs to update

`screens.md` and the design doc the screen joins; the [website plan §8](../../plans/website.md#8-to-dos).

## Risks and open questions

- **Claims age.** A promise that stops being true is worse than none, so every claim links to the page or test that keeps it true.

## Definition of done

- [ ] Claimed: this file moved `ready/` → `ongoing/` in the work PR's first commit (Status `ongoing`, links updated)
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` and `/writing-review` followed (findings and fixes recorded in commit messages)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
