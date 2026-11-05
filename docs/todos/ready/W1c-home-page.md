# W1c — The home page for now: what Lamina PDF is, the roadmap and what's coming next

| Field | Value |
| --- | --- |
| ID | `W1c` |
| Status | `ready` |
| Plan | [Website §4 The home page, for now](../../plans/website.md#the-home-page-for-now-the-roadmap) |
| Round / milestone | Round W (website) — ships before M2 Web reader v0.1 |
| Depends on | [W1b](./W1b-site-scaffold-and-deploys.md), [W1e](./W1e-progress-snapshot.md) |
| Blocks | None |
| ADRs | Follows 0011 (no third parties: the dashboard is static data in the build; each claim on the page ties to the rule behind it) |
| Branch | `feat/w1c-home-page` |
| PR title | `feat(site): the home page, with the roadmap and what's coming next` |
| Size | M (design, copy and 300–500 lines of site) |
| PR | — |

## Goal

A visitor to `laminapdf.com` learns in one screen what Lamina PDF is, then sees where the work stands: the roadmap's milestones, what is being built now, what comes next and what merged lately. They also find the parts that exist today: the docs, the SDK and the Rust crate. The dashboard is a dated snapshot that W1e's command refreshes from time to time, and the owner approves the page's words.

## Context

- [Website plan §4](../../plans/website.md#the-home-page-for-now-the-roadmap): what the home page shows for now, where its data comes from, and what it leaves out (people's names, links into the private repository); §7 for the owner's approval and when the roadmap leaves the home page.
- [W1e](./W1e-progress-snapshot.md): `apps/site/src/data/progress.json` and its TypeScript type, the page's only data.
- The build log the project keeps as a separate page: the model for the dashboard (totals, progress by round, in flight, ready to start, recently merged, the snapshot's date).
- [Roadmap § Milestones](../../plans/roadmap.md#milestones): the milestones the page lists.
- [Paper design plan](../../plans/paper-design.md), [design language §7.39](../../design/paper/design-language.md#739-site-and-product-pages-proposed) (the header, the large primary link), [screen inventory](../../design/paper/screens.md) (no home page yet).
- The docs site's [first-PDF quick start](../../../apps/docs/src/content/docs/start/first-pdf.mdx) for the install snippet; [docs-writing guideline](../../guidelines/docs-writing.md) for the copy.

## Scope

**In scope**

- The home page's design: a new screen in the inventory with its artboard, in Paper, light and dark, at phone and desktop widths.
- The page, built from the snapshot at build time (static HTML; no script is needed to read it):
  - a headline and one paragraph on what Lamina PDF is;
  - the milestones, M0 to M10, each with what it ships and its state;
  - the dashboard: totals (todos done of all, pull requests merged), progress by round as bars split into done, in progress and ready, with the counts in text beside each bar; in progress; next up; recently merged; the snapshot's date;
  - where each part lives: the docs, the SDK (`npm install @laminapdf/sdk` and a five-line example from the tested examples), the Rust crate; the reader and the desktop app link to their milestones;
  - "Open the web app", behind the build setting from the plan.
- The rounds' plain names ("Core library", "Web reader") in one table in the site.
- Share metadata: Open Graph and X card tags, a 1200 × 630 share image in Paper, a favicon set, `sitemap.xml` and `robots.txt` (production allows indexing, previews don't).

**Out of scope**

- The snapshot and its command → [W1e](./W1e-progress-snapshot.md).
- The privacy promises: [C7c](./C7c-performance-gates-and-launch.md) adds them to the home page when it publishes `/privacy`.
- Moving the roadmap to `/roadmap` at the reader's launch ([plan §7](../../plans/website.md#7-open-questions), question 3).
- The other pages (W1a names their todos); the shared header → [W1d](./W1d-one-site-header.md).

## Tasks

- [ ] Claim: the first commit moves this file to `ongoing/`.
- [ ] Design the screen; add it to `screens.md` and the design docs.
- [ ] Draft the copy with `/writing`; mark the headline, the paragraph and each claim `[OWNER TO APPROVE]` until the owner approves them in the PR.
- [ ] Build the page from the snapshot, the metadata and the share image; the install snippet comes from a tested example file, never typed by hand.
- [ ] `/writing-review` on the copy.

## Acceptance criteria

- [ ] The owner has approved the headline, every claim and what the dashboard shows.
- [ ] Every number and list on the page comes from `progress.json`: a test builds the page from a fixture snapshot and finds each count, title and the date.
- [ ] An empty list (nothing in progress, nothing merged) shows a sentence, not an empty box.
- [ ] Lighthouse on the preview, phone and desktop: 100 for accessibility, best practices and SEO; axe finds nothing in light and dark.
- [ ] The share image renders in the X and LinkedIn card validators against the preview.

## Tests

The site's post-build checks (W1b); the fixture-snapshot test; a test that the install snippet matches its example file.

## Docs to update

`screens.md` and the design doc the screen joins; the [website plan §8](../../plans/website.md#8-to-dos).

## Risks and open questions

- **Claims age.** A promise that stops being true is worse than none, so every claim links to the page or test that keeps it true.
- **A stale snapshot.** The date sits beside the numbers; refreshing is W1e's command and an exempt PR.

## Definition of done

- [ ] Claimed: this file moved `ready/` → `ongoing/` in the work PR's first commit (Status `ongoing`, links updated)
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` and `/writing-review` followed (findings and fixes recorded in commit messages)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
