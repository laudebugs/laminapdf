# Q4i — Code colours by class, and no inline styles in the docs' CSP

| Field | Value |
| --- | --- |
| ID | `Q4i` |
| Status | `done` |
| Plan | [Docs redesign §4](../../plans/docs-redesign.md#4-todos) |
| Round / milestone | Round Q (docs and code quality), Q4 docs redesign |
| Depends on | None |
| ADRs | [ADR 0011](../../decisions/0011-cloudflare-topology-and-privacy.md) (the headers) |
| Branch | `fix/q4i-docs-csp-styles` |
| PR title | `fix(docs): colour code by class and drop 'unsafe-inline' from style-src` |
| Size | M |
| PR | — |

## Goal

The docs site's CSP allows inline styles (`style-src 'self' 'unsafe-inline'`) only because Expressive Code colours code tokens with style attributes. The website already colours its code through classes and allows no inline styles. This todo does the same for the docs, so the docs, like the website, allow no inline styles.

## Context

- [Docs research: recommendation 7](../../research/docs-site/developer-docs.md#recommendations-in-order) and the [frameworks note](../../research/docs-site/developer-docs/docs_frameworks.md).
- `apps/docs/public/_headers` (the comment that explains the exception), `apps/site/src/data/highlight.ts` (classes from Shiki's tokens, coloured by `--paper-code-*`).
- Expressive Code's options in `astro.config.mjs`.

## Scope

**In scope**

- Code tokens coloured by classes and the `--paper-code-*` tokens, in light and dark.
- `'unsafe-inline'` removed from `style-src` on every docs route.
- Any other inline `style` attribute the build emits found and removed.

**Out of scope**

- The runner's own CSP under `/run/*`.

## Tasks

- [x] Find whether Expressive Code can emit classes; if not, colour tokens at build time as `highlight.ts` does and keep Expressive Code's frames, copy button and tabs.
- [x] Scan the built pages for `style=` attributes and remove each source.
- [x] Drop `'unsafe-inline'`; run the e2e tests with CSP violations reported as failures.

## Acceptance criteria

- [x] No built page has a `style` attribute.
- [x] The docs' headers have no `'unsafe-inline'`.
- [x] Code looks the same in light and dark as before.

## Post-merge checks

- [x] The production docs site shows no CSP violation in the browser console on a sample of pages.

## Tests

A test that fails on any `style=` in `dist/`; the e2e CSP check; screenshots of a code block in both themes.

## Docs to update

The `_headers` comment; ADR 0011's notes if it lists the exception.

## Risks and open questions

- Starlight or a plugin may emit inline styles of its own; each needs an override or a fix upstream.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
