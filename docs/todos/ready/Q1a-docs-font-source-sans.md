# Q1a — The docs site in one sans-serif font (Source Sans 3)

| Field | Value |
| --- | --- |
| ID | `Q1a` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | None |
| ADRs | None |
| Branch | `feat/q1a-docs-font` |
| PR title | `feat(docs): one sans-serif font for the docs site (Source Sans 3)` |
| Size | S |
| PR | — |

## Goal

The owner asked (2026-10-31) for one sans-serif font across the docs site. Today headings use Newsreader (serif) and text uses Source Sans 3. Switch every heading, the site title and the hero to Source Sans 3, carry the hierarchy with weight and size, and drop the four Newsreader files (about 461 KB) from `apps/docs`.

## Context

- [Quality plan §5](../../plans/quality.md#5-the-font): the decision and why Source Sans 3 (already self-hosted, OFL-1.1, variable, the app's UI face).
- `apps/docs/src/styles/paper.css` (the `@font-face` blocks, `--paper-font-serif`, the heading rules), `apps/docs/src/fonts/` and its README, `apps/docs/README.md` ("Look").
- [Design language §3](../../design/paper/design-language.md#3-typography) ("Site display" row) and §7.39 (the site wordmark): both name Newsreader for docs-site pages.
- [U6g](../done/U6g-docs-site-paper.md): the interim Paper theme this changes; [U6b](./U6b-self-hosted-fonts.md): the fonts package that will replace the copies.

## Scope

**In scope**

- Delete the Newsreader `@font-face` rules and `--paper-font-serif`; headings, `.site-title` and the hero use `--sl-font` with weights 600–700 and slight negative letter-spacing; consider `--sl-text-base: 1.0625rem` (Source Sans 3 has a small x-height).
- Delete `newsreader-*.woff2` and `OFL-Newsreader.txt` from `apps/docs/src/fonts/`; update its README and the docs README.
- Design language: the docs site is sans-only (§3 "Site display", §7.39 wordmark); the app keeps Newsreader unless the owner decides otherwise (plan §8 question 1). Note it in U6b.

**Out of scope** (and which todo covers it)

- The app's fonts ([U6b](./U6b-self-hosted-fonts.md)).
- Any copy changes ([Q1h](./Q1h-docs-story.md), [Q1i](./Q1i-guides-plain-english.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Edit `paper.css`; delete the font files; update both READMEs.
- [ ] Build the site (`pnpm --filter @laminapdf/docs build`) and look at the home page, a guide, the code table and a code page in light and dark at desktop and phone widths.
- [ ] Amend design-language §3 and §7.39; add a line to U6b's context.

## Acceptance criteria

- [ ] No `Newsreader` anywhere under `apps/docs/` (grep), and no request for a Newsreader file in the built site.
- [ ] Headings are visibly distinct from body text by weight and size in both themes; CSP `font-src 'self'` unchanged.
- [ ] The `check ts` docs build passes.

## Post-merge checks

None.

## Tests

None beyond the docs build in `cargo xtask check ts`; screenshots of four pages in the PR description.

## Docs to update

`apps/docs/README.md`, `apps/docs/src/fonts/README.md`, design-language §3 and §7.39, U6b context, plan §9 to-do.

## Risks and open questions

Pure CSS and assets; low risk. If headings look too close to body text, raise the weight before adding a second family.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
