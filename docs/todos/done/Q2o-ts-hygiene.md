# Q2o — TypeScript hygiene: no test hooks in shipped code

| Field | Value |
| --- | --- |
| ID | `Q2o` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2u](./Q2u-ts-dependency-tools.md) |
| Blocks | None |
| ADRs | None |
| Branch | `fix/q2o-ts-hygiene` |
| PR title | `fix(viewer,sdk): no test hooks in shipped code; shared helpers; a smaller viewer surface` |
| Size | M |
| PR | [#166](https://github.com/laudebugs/lamina/pull/166) |

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

- Error shapes ([Q2k](../done/Q2k-boundary-errors-ts.md)); complexity splits ([Q2p](../ready/Q2p-ts-complexity-splits.md)).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Add test worker entries in sdk and viewer that register the `__` methods; remove them from `src/`; remove `renderLog`.
- [x] Extract the shared helpers; replace each copy.
- [x] Switch the viewer to `RangeSource`; trim and document its exports. (knip has no baseline: [Q2u](../done/Q2u-ts-dependency-tools.md) found nothing to list, because knip counts an export as used when a test uses it and never reports a package entry's own exports.)
- [x] Fix the docs app and the base config; write the entry-diff test.
- [x] Run every suite.

## Acceptance criteria

- [x] No `__`-prefixed method in any package's `src/`.
- [x] Every viewer export is part of its documented API (a review check: knip can't see test-only exports or an entry's own exports, so it passes either way).

## Outcome

No package's `src/` has a test hook now, and the viewer exports only its documented API. A changeset records the viewer's smaller surface and the SDK's new worker options. Five choices go beyond the plan.

- **Test hooks:** the SDK's and the viewer's browser tests start their own workers (`test/browser/test-worker.ts`), which register the `__` methods on top of what ships.
  - The viewer's methods moved into `src/worker/methods.ts` (`viewerMethods`), so its test worker serves the same methods and wraps `render` to log scales. `renderLog`, the list that grew with every render, is gone, and `test/worker-state.test.ts` checks that the render module keeps no collection at module level.
  - Two of the SDK's hooks reached into the server, so they became small public pieces of `@laminapdf/sdk/worker/server`. `serve({ saveBudget })` sets the work units per save step: the cancellation test starts a worker named `budget-50`, and its test worker passes that on. `workerStats()` reports calls started, handles held and calls queued; a growing handle count finds a `RemoteDocument` an app forgot to close. `__crash` lives in the test workers only.
  - The viewer client's `@internal` getters went too. The cancellation test counts `ImageBitmap.prototype.close` calls instead of `discardedBitmaps`. The password test drops its `reopenState` checks. The crash test still shows the client keeps no password in memory: after a crash it must ask for it again (`password-required`), which it wouldn't if it held one.
- **Shared helpers:** `whole` is in `packages/sdk/src/worker/transfer.ts` and `defined()` in `packages/sdk/src/defined.ts`, used by the worker client, the worker server and the builder (`rest()`). `Emitter<E>` is in `src/worker/emitter.ts`, exported from `@laminapdf/sdk/worker`: the SDK's `WorkerClient` and the viewer's `ViewerClient` both use it. The viewer's option spreads stay, because they are expressions, not helpers.
- **The viewer's surface:** its sources are `@laminapdf/sdk`'s `RangeSource` and `callbackSource`, and `ByteSource`, `bufferSource`, `blobSource` and `readAll` are no longer exported. The index keeps `ViewerClient`, `createViewer`, `Viewer`, their option, event and state types, the protocol types `client.call` uses, and `MIN_ZOOM`/`MAX_ZOOM`, which the README now lists. The cache, pool, layout, tile and zoom maths stay internal, and knip found four constants and a function that only their own modules use, now private.
- **The docs app:** `tsconfig.json` sets `checkJs`. That covers `plugins/*.mjs`, `scripts/check-api-docs.mjs` and `tools/ts/package-entries.mjs`, which now carry JSDoc types. `src/lib/codes.ts` reads `codes.json` once for the code table and the code pages. `plugins/repository.mjs` holds the repository URL and the `LAMINA_DOCS_GITHUB` flag.
- **Config and entries:** `tsconfig.base.json` carries `ESNext.Disposable`, and nine package configs no longer repeat it. The viewer's build config keeps it, because its own `lib` adds `WebWorker.ImportScripts` and replaces the base list. `test/entries.test.ts` checks that the three SDK entries match apart from their header, their `locate` import and the phrase saying where the module comes from.

## Post-merge checks

None.

## Tests

Existing suites, the entry-diff test, and a test that the worker keeps no per-render state.

## Docs to update

Plan §9.

## Risks and open questions

Trimming the viewer's exports breaks callers while it is private: add a changeset.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
