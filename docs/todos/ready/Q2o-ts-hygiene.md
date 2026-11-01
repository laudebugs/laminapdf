# Q2o — TypeScript hygiene: test hooks out of shipped code, shared helpers, viewer surface

| Field | Value |
| --- | --- |
| ID | `Q2o` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | None |
| ADRs | None |
| Branch | `fix/q2o-ts-hygiene` |
| PR title | `fix(viewer,sdk): no test hooks in shipped code; shared helpers; smaller viewer surface` |
| Size | M |
| PR | — |

## Goal

The viewer's worker keeps `renderLog` (grows on every render, never trimmed: a leak in the shipped worker) and the public method types include `__busy`, `__renderLog`, `__crash`, `__budget`, `__stats`. Small helpers are written several times (`whole()` 2×, listener fan-out 2×, "drop undefined keys" 4× plus 23 inline spreads), the viewer has its own byte-source type next to sdk's `RangeSource`, and it exports plumbing without TSDoc.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `packages/viewer/src/{worker/render.ts:11, protocol.ts:58-63, client.ts:161-183,227-235, source.ts, index.ts:18-20}`, `packages/sdk/src/worker/{server.ts:139,167,268-278, client.ts:78,116,491}`, `packages/sdk/src/builder.ts:244`, `apps/docs` (`tsconfig` without `checkJs`; codes.json parsed twice; repo URL in two places), `tsconfig.base.json` (`ESNext.Disposable` repeated in 10 tsconfigs), the three near-identical sdk entries.

## Scope

**In scope**

- Test methods registered only from test worker entries; `renderLog` removed.
- `whole`, `Emitter<E>` and `defined()` once in `@laminapdf/sdk/worker` (or sdk-internal); viewer uses `RangeSource`/`callbackSource`; viewer exports trimmed; missing TSDoc added.
- Docs app: `checkJs` for `plugins/*.mjs`, `src/lib/codes.ts`, one repo-URL constant.
- `ESNext.Disposable` in the base tsconfig; a test that the three sdk entries differ only in their `locate` import and TSDoc phrase.

**Out of scope** (and which todo covers it)

- Error shapes ([Q2k](./Q2k-boundary-errors-ts.md)); complexity splits ([Q2p](./Q2p-ts-complexity-splits.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] One change at a time with its tests.

## Acceptance criteria

- [ ] No `__`-prefixed method in any package `src/` (grep in `check ts`).
- [ ] knip (from [Q2b](./Q2b-maintainability-checks-ts.md)) reports no unused export in the viewer.

## Post-merge checks

None.

## Tests

Existing suites; the entry-diff test; a test that the worker keeps no per-render state.

## Docs to update

Plan §9.

## Risks and open questions

Viewer API trim is a breaking change while private: fine; note in a changeset.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
