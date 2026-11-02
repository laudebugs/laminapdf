# Q2n — Shared code for the render adapters

| Field | Value |
| --- | --- |
| ID | `Q2n` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2m](./Q2m-ts-test-utils.md) |
| Blocks | None |
| ADRs | None |
| Branch | `refactor/q2n-render-adapter-shared` |
| PR title | `refactor(render): shared adapter helpers, one WasmSource type, memory() on RenderBackend` |
| Size | M |
| PR | [#185](https://github.com/laudebugs/laminapdf/pull/185) |

## Goal

The hayro and PDFium adapters repeat the page-index check, the "bytes could not be read" handling, the input-size limit and the disposed flag, and only PDFium merges pending ranges. Five packages define their own WASM-source type, and PDFium hand-writes an instantiate that the glue already provides. This todo moves the shared pieces into `@laminapdf/render` so both adapters behave the same.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `packages/render-hayro/src/index.ts:91,119-142,185,193`, `packages/render-pdfium/src/index.ts:46-60,101,127,169-196,243,268`, `packages/render/src/index.ts:11-22,244`, `packages/sdk/src/init.ts:20,216`, `packages/sdk/src/worker/client.ts:43`, `packages/viewer/src/protocol.ts:17`.

## Scope

**In scope**

- `checkPageIndex`, `readFailure`, `readRange` (merging pending ranges), the input-limit constant and a disposed guard in `@laminapdf/render`.
- One `WasmSource` type there, used by both adapters and the viewer; `memory()` on `RenderBackend`; PDFium uses the glue's init where it can.

**Out of scope** (and which todo covers it)

- Splitting PDFium's `render` closure ([Q2p](../ongoing/Q2p-ts-complexity-splits.md)).

## Tasks

- [x] Claim by opening the work PR (#185) from a fresh worktree off `main`.
- [x] Add a contract case for merged pending ranges; watch hayro fail it (it reported four touching 64 KiB chunks).
- [x] Move each helper into `@laminapdf/render` (`checkPageIndex`, `checkOpen`, `readRange`, `readFailure`, `mergeRanges`, `pendingError`, `MAX_WASM_INPUT`) and switch both adapters.
- [x] Add `WasmSource` (and `CloneableWasmSource`, what a worker can be sent) and `memory()`; switch the viewer. PDFium hands bytes to the glue's `wasmBinary`.
- [x] Run the contract suite in Node and the three browsers.

## Acceptance criteria

- [x] Both adapters pass the shared contract, including merged ranges.

## Post-merge checks

None.

## Tests

The contract suite with the merged-ranges case.

## Docs to update

Plan §9.

## Risks and open questions

None.

## Definition of done

- [x] Claimed: the work PR (#185), opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
