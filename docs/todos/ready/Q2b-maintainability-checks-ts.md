# Q2b — Maintainability checks for TypeScript: complexity, cycles, duplication, unused exports, TSDoc

| Field | Value |
| --- | --- |
| ID | `Q2b` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | [Q2p](./Q2p-ts-complexity-splits.md) |
| ADRs | None |
| Branch | `ci/q2b-maintainability-checks-ts` |
| PR title | `ci(ts): complexity, cycle, duplication, unused-export and TSDoc checks` |
| Size | M |
| PR | — |

## Goal

Make the [maintainability guideline](../../guidelines/maintainability.md) checkable for TypeScript in `cargo xtask check ts`: Biome complexity limits with today's offenders listed, `useThrowOnlyError`, dependency-cruiser for cycles and cross-package relative imports, jscpd for duplicated blocks, knip for unused exports and files, and TypeDoc's `notDocumented` for undocumented exports.

## Context

- [Maintainability guideline](../../guidelines/maintainability.md), [Quality plan §2.3, §4.3](../../plans/quality.md#23-code-quality-and-reuse); root `biome.json`, package `biome.json`s, ADR 0019 (licence check for new dev dependencies: dependency-cruiser MIT, jscpd MIT, knip ISC).
- Today's Biome offenders (complexity > 15): PDFium `render` (39), `server.ts execute` (25), `planTile` (22), `clientCall` (21), two in `source/http.ts`, `PdfiumDocument.open`, hayro `fill`.

## Scope

**In scope**

- Root Biome: `noExcessiveCognitiveComplexity` (15), `noExcessiveLinesPerFunction` (60), `useThrowOnlyError`, at `error` with an override list naming today's offenders.
- dependency-cruiser config: no cycles; no relative import into another package (`packages/*` → `../<other>`); baseline the render adapters' test imports until [Q2m](./Q2m-ts-test-utils.md).
- jscpd over `packages/*/src` (min 10 lines, threshold 0, tests excluded at first).
- knip over the workspace (entry points from `exports`).
- TypeDoc `validation.notDocumented` + `treatWarningsAsErrors` per package entry.
- All new dev dependencies through `cargo xtask npm-licenses`.

**Out of scope** (and which todo covers it)

- Fixing offenders ([Q2p](./Q2p-ts-complexity-splits.md), [Q2o](./Q2o-ts-hygiene.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Add each tool with its config; run; record findings; list or fix them; wire into `check ts`.

## Acceptance criteria

- [ ] `cargo xtask check ts` runs every tool and passes on `main` with the listed exceptions.
- [ ] Licences pass `npm-licenses`.

## Post-merge checks

None.

## Tests

The tools themselves; a scratch-branch demonstration of each failing in the PR description.

## Docs to update

TypeScript guideline, maintainability guideline, CLAUDE.md commands, plan §9.

## Risks and open questions

Tool noise: start each with the narrowest scope that catches real problems; widen later.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
