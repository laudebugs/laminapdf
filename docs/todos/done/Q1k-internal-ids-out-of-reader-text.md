# Q1k — Internal IDs out of error pages and READMEs

| Field | Value |
| --- | --- |
| ID | `Q1k` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q1b](./Q1b-docs-prose-check.md) |
| Blocks | [Q1l](../ready/Q1l-docs-checks-strict.md) |
| ADRs | None |
| Branch | `docs/q1k-internal-ids-out-of-reader-text` |
| PR title | `docs: keep ADR and todo IDs out of reader-facing text` |
| Size | M |
| PR | [#149](https://github.com/laudebugs/lamina/pull/149) |

## Goal

45 of the 117 error-code pages and the SDK README, all rendered on the docs site, mention ADR numbers or todo IDs, and the repository README never links the live docs. This todo rewrites them in reader terms and makes the error-page plugin drop contributor-only notes.

## Context

- [Quality plan §2.1](../../plans/quality.md#21-docs-prose-and-navigation); `docs/errors/*.md`, `apps/docs/plugins/error-pages.mjs`, `packages/sdk/README.md`, `README.md`, `apps/docs/src/content/docs/reference/rust.mdx` ("fast lane").

## Scope

**In scope**

- Each flagged `docs/errors/*.md` line rewritten in reader terms; the decision links kept in a "For contributors" section that the plugin drops.
- SDK README: what it is and a five-line example first; no ADR, P1 or P2.
- Repository README: a one-sentence intro, a link to docs.laminapdf.com, a short example; the stale workers.dev line removed; the crate list moved lower.
- `reference/rust.mdx`: how to read the Rust docs, nothing about CI.

**Out of scope** (and which todo covers it)

- Guide rewrites ([Q1i](../done/Q1i-guides-create.md), [Q1m](../ready/Q1m-guides-read-and-edit.md)).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Run `docs-prose` for its internal-ID reports; fix each.
- [x] Teach `error-pages.mjs` to drop the contributor section; check a rendered error page (on the PR's preview).
- [x] Rewrite the two READMEs and the Rust reference page. Deviation: the viewer, render and wasm READMEs and the site pages had internal IDs too, so they were rewritten as well (acceptance covers everywhere the check looks); the SDK's test matrix moved to `packages/sdk/CONTRIBUTING.md`.
- [x] Run `cargo xtask gen codes --check` (the error pages feed generated tables).

## Acceptance criteria

- [x] `docs-prose --all-strict` reports no internal ID anywhere it checks.
- [x] `cargo xtask gen codes --check` passes.

## Post-merge checks

None.

## Tests

`docs-prose`, `gen codes --check`, the docs build.

## Docs to update

`docs/errors/README.md` (the page template), plan §9.

## Risks and open questions

Contributors use those IDs: keep them reachable in the contributor section, just not in reader text. The Rust and TypeScript `explain` text still carries the whole page, contributor section included (the Rust side `include_str!`s the file); only the docs site leaves it out.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
