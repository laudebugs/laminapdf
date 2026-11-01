# Q1a — One sans-serif font on the docs site (Source Sans 3)

| Field | Value |
| --- | --- |
| ID | `Q1a` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | None |
| ADRs | None |
| Branch | `feat/q1a-docs-font-source-sans` |
| PR title | `feat(docs): one sans-serif font for the docs site (Source Sans 3)` |
| Size | S |
| PR | — |

## Goal

The owner asked (2026-10-31) for one sans-serif font across the docs site. Headings use Newsreader (serif) today and text uses Source Sans 3. This todo sets every heading, the site title and the hero in Source Sans 3, lets weight and size carry the hierarchy, and removes the four Newsreader files (about 461 KB).

## Context

- [Quality plan §5](../../plans/quality.md#5-the-font): why Source Sans 3 (already self-hosted, OFL-1.1, variable, the app's UI face).
- `apps/docs/src/styles/paper.css`: the Newsreader `@font-face` blocks, `--paper-font-serif` and the heading rules; `apps/docs/src/fonts/` and its README; the "Look" section of `apps/docs/README.md`.
- [Design language §3](../../design/paper/design-language.md#3-typography) ("Site display") and §7.39 (the site wordmark), which name Newsreader for docs-site pages.
- [U6g](../done/U6g-docs-site-paper.md), the interim Paper theme this changes; [U6b](./U6b-self-hosted-fonts.md), the fonts package that later replaces the copies.

## Scope

**In scope**

- `paper.css`: no Newsreader `@font-face` and no `--paper-font-serif`.
- Headings, `.site-title` and the hero use `--sl-font`, with weights 600–700 and slight negative letter-spacing.
- Base text size: set `--sl-text-base: 1.0625rem` if body text looks small beside the old serif (Source Sans 3 has a small x-height); say which in the PR.
- Remove the Newsreader files and licence from `apps/docs/src/fonts/`; update both READMEs.
- Design language §3 and §7.39: the docs site is sans-only; the app keeps Newsreader unless the owner decides otherwise (plan §8).

**Out of scope** (and which todo covers it)

- The app's fonts ([U6b](./U6b-self-hosted-fonts.md)).
- Copy changes ([Q1h](./Q1h-docs-story.md) and later).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Delete the Newsreader `@font-face` rules and `--paper-font-serif` from `paper.css`.
- [ ] Rewrite the heading, site-title and hero rules on `--sl-font` with explicit weights and letter-spacing.
- [ ] Delete `newsreader-*.woff2` and `OFL-Newsreader.txt`; update `apps/docs/src/fonts/README.md` and `apps/docs/README.md`.
- [ ] Build the site (`pnpm --filter @laminapdf/docs build`) and compare the home page, a guide, the code table and an error page in light and dark, at desktop and phone widths; adjust weights until headings stand out clearly.
- [ ] Amend design-language §3 and §7.39, and add a line to U6b's Context.

## Acceptance criteria

- [ ] No `Newsreader` under `apps/docs/` and no Newsreader request in the built site.
- [ ] Headings stand out from body text by weight and size in both themes (screenshots in the PR).
- [ ] The CSP `font-src 'self'` is unchanged; the docs build in `cargo xtask check ts` passes.

## Post-merge checks

None.

## Tests

The docs build in `cargo xtask check ts`; before-and-after screenshots of four pages in the PR description.

## Docs to update

`apps/docs/README.md`, `apps/docs/src/fonts/README.md`, design-language §3 and §7.39, U6b's Context, plan §9.

## Risks and open questions

Pure CSS and assets. If headings look too close to body text, raise the weight before adding a second family.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
