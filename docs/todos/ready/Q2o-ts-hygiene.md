# Q2o — TypeScript hygiene: no test hooks in shipped code

| Field | Value |
| --- | --- |
| ID | `Q2o` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2u](../done/Q2u-ts-dependency-tools.md) |
| Blocks | None |
| ADRs | None |
| Branch | `fix/q2o-ts-hygiene` |
| PR title | `fix(viewer,sdk): no test hooks in shipped code; shared helpers; a smaller viewer surface` |
| Size | M |
| PR | — |

## Goal

The viewer's worker keeps `renderLog`, which grows on every render and is never trimmed: a leak in the shipped worker. The public method types include the test hooks `__busy`, `__renderLog`, `__crash`, `__budget` and `__stats`. Small helpers are written several times, the viewer has its own byte-source type next to sdk's `RangeSource`, and it exports plumbing without TSDoc. This todo cleans all of that up.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `packages/viewer/src/{worker/render.ts:11, protocol.ts:58-63, client.ts:161-183,227-235, source.ts, index.ts:18-20}`, `packages/sdk/src/worker/{server.ts:139,167,268-278, client.ts:78,116,491}`, `packages/sdk/src/builder.ts:244`.
- The docs app: `tsconfig` without `checkJs`; `codes.json` parsed twice; the repository URL in two places. `tsconfig.base.json`: `ESNext.Disposable` repeated in 10 package configs. The three near-identical sdk entry files.

## Scope

**In scope**

- Test methods registered only from test worker entries; `renderLog` removed.
- `whole`, `Emitter<E>` and `defined()` written once in `@laminapdf/sdk/worker` (or sdk-internal); the viewer uses `RangeSource` and `callbackSource`; viewer exports trimmed and documented.
- Docs app: `checkJs` for `plugins/*.mjs`, `src/lib/codes.ts`, one repository-URL constant.
- `ESNext.Disposable` in the base config; a test that the three sdk entries differ only in their `locate` import and one TSDoc phrase.

**Out of scope** (and which todo covers it)

- Error shapes ([Q2k](../ongoing/Q2k-boundary-errors-ts.md)); complexity splits ([Q2p](./Q2p-ts-complexity-splits.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Add test worker entries in sdk and viewer that register the `__` methods; remove them from `src/`; remove `renderLog`.
- [ ] Extract the shared helpers; replace each copy.
- [ ] Switch the viewer to `RangeSource`; trim and document its exports. (knip has no baseline: [Q2u](../done/Q2u-ts-dependency-tools.md) found nothing to list, because knip counts an export as used when a test uses it and never reports a package entry's own exports.)
- [ ] Fix the docs app and the base config; write the entry-diff test.
- [ ] Run every suite.

## Acceptance criteria

- [ ] No `__`-prefixed method in any package's `src/`.
- [ ] Every viewer export is part of its documented API (a review check: knip can't see test-only exports or an entry's own exports, so it passes either way).

## Post-merge checks

None.

## Tests

Existing suites, the entry-diff test, and a test that the worker keeps no per-render state.

## Docs to update

Plan §9.

## Risks and open questions

Trimming the viewer's exports breaks callers while it is private: add a changeset.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
