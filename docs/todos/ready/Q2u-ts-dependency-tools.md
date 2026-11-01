# Q2u — TypeScript dependency tools: cycles, cross-package imports, duplication, unused exports

| Field | Value |
| --- | --- |
| ID | `Q2u` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | [Q2o](./Q2o-ts-hygiene.md) |
| ADRs | None |
| Branch | `ci/q2u-ts-dependency-tools` |
| PR title | `ci(ts): dependency-cruiser, jscpd and knip in check ts` |
| Size | M |
| PR | — |

## Goal

This todo adds three dev tools to `cargo xtask check ts`, after a licence check: dependency-cruiser for import cycles and imports that reach into another package, jscpd for duplicated blocks, and knip for unused exports and files.

## Context

- The [maintainability guideline](../../guidelines/maintainability.md) ("Public surface and tests", "How it's checked"); ADR 0019 and `cargo xtask npm-licenses` (dependency-cruiser MIT, jscpd MIT, knip ISC).
- Today's cross-package imports: `packages/render-{hayro,pdfium}/test/*` and `vitest.browser.config.ts` import `../render/test/…` (fixed by [Q2m](./Q2m-ts-test-utils.md)).

## Scope

**In scope**

- dependency-cruiser: no cycles; no relative import into another package; today's render-adapter test imports in a baseline.
- jscpd over `packages/*/src` (at least 10 lines, threshold 0, tests excluded at first).
- knip over the workspace, entry points taken from each package's `exports`.
- All three as root dev dependencies, pinned, through `npm-licenses`.

**Out of scope** (and which todo covers it)

- Biome and TypeDoc rules ([Q2b](./Q2b-ts-lint-rules.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Add the three tools and run `cargo xtask npm-licenses`.
- [ ] Write each configuration; run each; record today's findings.
- [ ] Baseline what other todos fix (Q2m's imports, Q2o's unused exports), fix the rest, and wire all three into `check ts`.

## Acceptance criteria

- [ ] `cargo xtask check ts` runs all three and passes on `main` with the recorded baselines.
- [ ] Licences pass `npm-licenses`.

## Post-merge checks

None.

## Tests

The tools themselves; a scratch-branch run showing each failing (in the PR description).

## Docs to update

typescript.md, the commands list in CLAUDE.md, plan §9.

## Risks and open questions

Tool noise: start each with the narrowest scope that catches real problems, and widen later.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
