# W1f — Code colouring on the home page

| Field | Value |
| --- | --- |
| ID | `W1f` |
| Status | `done` |
| Plan | [website §4](../../plans/website.md#4-the-website) |
| Round / milestone | Round W (website) |
| Depends on | [W1c](./W1c-home-page.md) |
| Blocks | None |
| ADRs | Follows [0011 §4](../../decisions/0011-cloudflare-topology-and-privacy.md#4-security-headers-appswebpublic_headers-for-) (the website's `style-src 'self'`) and [0024 §7](../../decisions/0024-app-shell-ux.md) (`--paper-` tokens) |
| Branch | `feat/w1f-code-colouring` |
| PR title | `feat(site): colour the home page's code` |
| Size | S (≈ 150–300 lines: the code panel, a style sheet, tokens, tests) |
| PR | [#264](https://github.com/laudebugs/laminapdf/pull/264) |

## Goal

Colour the code on the home page. The code panel's TypeScript, Rust and command-line tabs show plain text today; they get syntax colours at build time, with no script, in the site's light and dark themes.

## Context

- Asked for by the owner on 2026-11-05.
- `apps/site/src/components/home/CodePanel.astro` and `src/data/sources.ts` (the code comes from the tested examples).
- Astro ships Shiki. Shiki colours each token with an inline `style`, which the website's `style-src 'self'` blocks and `@laminapdf/site-kit`'s checks refuse; `@shikijs/transformers`' `transformerStyleToClass` turns them into classes whose rules go in a style sheet.
- The docs site colours code with Expressive Code's default themes; the website's colours come from the Paper tokens instead.

## Scope

**In scope**

- Build-time highlighting of the three panes with classes, not inline styles; the generated rules in a style sheet of the site's own.
- Token colours from `--paper-*` tokens in both themes (new code-token tokens in `@laminapdf/tokens` if the palette lacks them), each at 4.5:1 or more against the panel's background.
- Tests: the built page has coloured tokens in every pane and still passes the site's checks; the style sheet uses tokens only; contrast pairs.

**Out of scope**

- The docs site's code blocks (Expressive Code stays).

## Tasks

- [x] Highlight the panes with Shiki, classified by TextMate scope (`transformerStyleToClass` wasn't needed: the panel renders its own classed spans)
- [x] The colours from tokens, with contrast pairs
- [x] Tests and the site README

## Acceptance criteria

- [x] Every pane's code is coloured in light and dark themes, with no inline style and no script added
- [x] Every token colour meets 4.5:1 against the panel's background in both themes
- [x] The website's build and checks pass

## Post-merge checks

- [ ] The production home page shows coloured code

## Tests

`apps/site/test/` (the built panel, the style sheet), `@laminapdf/tokens`' contrast pairs if new tokens are added.

## Docs to update

- [x] `apps/site/README.md` (the home page), [website §8](../../plans/website.md#8-to-dos)

## Risks and open questions

- Shiki's languages add build time, not page weight; only TypeScript, Rust and shell are loaded.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
