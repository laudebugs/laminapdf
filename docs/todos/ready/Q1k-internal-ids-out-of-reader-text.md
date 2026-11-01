# Q1k — Internal IDs out of error pages, package READMEs and the repo README

| Field | Value |
| --- | --- |
| ID | `Q1k` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q1b](./Q1b-docs-prose-check.md) |
| Blocks | [Q1l](./Q1l-docs-checks-strict.md) |
| ADRs | None |
| Branch | `docs/q1k-reader-text-no-internal-ids` |
| PR title | `docs: keep ADR and todo IDs out of reader-facing text` |
| Size | M |
| PR | — |

## Goal

45 of 117 error-code pages and the SDK README (both rendered on the docs site) mention ADR numbers or todo IDs, and the repository README never links the live docs. Rewrite them in reader terms, and make the error-page plugin stop rendering contributor-only notes.

## Context

- [Quality plan §2.1](../../plans/quality.md#21-docs-prose-and-navigation); `docs/errors/*.md`, `apps/docs/plugins/error-pages.mjs`, `packages/sdk/README.md`, `README.md`, `apps/docs/src/content/docs/reference/rust.mdx` ("fast lane").

## Scope

**In scope**

- Rewrite each flagged `docs/errors/*.md` line in reader terms (keep the decision links in an HTML comment or a "For contributors" section the plugin drops).
- SDK README: lead with what it is and a five-line example; no ADR/P1/P2.
- Repo README: one-sentence intro, docs.laminapdf.com link, short example, remove the stale workers.dev line; crate list moves below.
- `reference/rust.mdx`: how to read the Rust docs, nothing about CI.

**Out of scope** (and which todo covers it)

- Guide rewrites ([Q1i](./Q1i-guides-plain-english.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Run `docs-prose` for the internal-ID rule; fix every hit; adjust the plugin.

## Acceptance criteria

- [ ] `docs-prose --strict` reports no internal-ID hit anywhere it checks.
- [ ] `cargo xtask gen codes --check` still passes (error pages are inputs to generated docs tables).

## Post-merge checks

None.

## Tests

`docs-prose`, `gen codes --check`, the docs build.

## Docs to update

`docs/errors/README.md` (the page template), plan §9.

## Risks and open questions

Contributors use those IDs: keep them reachable (comments or a contributor section), just not in reader text.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
