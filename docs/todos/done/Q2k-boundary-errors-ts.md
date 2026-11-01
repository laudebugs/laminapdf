# Q2k — One error wire shape in TypeScript (fixes lost render ranges)

| Field | Value |
| --- | --- |
| ID | `Q2k` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2j](./Q2j-boundary-errors-rust.md) |
| Blocks | None |
| ADRs | None |
| Branch | `fix/q2k-boundary-errors-ts` |
| PR title | `fix(sdk,render,viewer): one error wire shape; render data-pending keeps its ranges across the worker` |
| Size | M |
| PR | [#153](https://github.com/laudebugs/lamina/pull/153) |

## Goal

The worker turns a render error's `ranges` into `{offset, length}` objects, but `LaminaError.ranges` only accepts `[offset, length]` tuples (what Rust emits), so a render `data-pending` that crosses the worker arrives with no ranges. Render errors also have an unused `toJSON`/`reviveRenderError` pair, the worker duck-types instead, and the viewer throws plain objects. This todo leaves one shape (tuples) and one path.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `packages/sdk/src/worker/server.ts:116-136`, `packages/sdk/src/errors.ts:97`, `packages/render/src/index.ts:199-222`, `packages/viewer/src/worker/document.ts:39,93,118`; typescript.md ("never throw plain objects").

## Scope

**In scope**

- `RenderError.toDiagnostic()` returning the ADR 0026 object; the worker's `diagnostic()` calls it; `reviveRenderError` and `SerializedRenderError` deleted.
- Ranges as tuples everywhere.
- The viewer throws `RenderError("cancelled" | "disposed")` and maps its bare `Error`.

**Out of scope** (and which todo covers it)

- Generated render codes ([Q2l](../ready/Q2l-boundary-lists-generated.md)).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Write a test where a render `data-pending` crosses the worker; watch it lose its ranges.
- [x] Add `toDiagnostic`; switch the worker; delete the unused pair.
- [x] Change the viewer's throws; remove its entries from [Q2b](../done/Q2b-ts-lint-rules.md)'s `useThrowOnlyError` override.
- [x] Run the Node and browser suites.

## Acceptance criteria

- [x] The crossing test passes.
- [x] No plain-object throw in `packages/*/src`.

## Outcome

A render `data-pending` thrown in the worker now reaches the page with its ranges. The new browser test failed before the fix: the ranges arrived as `[]`. A changeset records the fix and the removal of `toJSON()`, `reviveRenderError` and `SerializedRenderError`. Four choices go beyond the plan.

- **How the worker finds the object:** `@laminapdf/sdk` can't import `@laminapdf/render`, so the worker's `diagnostic()` can't check for `RenderError` by class. Instead it calls any thrown value's `toDiagnostic()` method and posts the result if it is a §1 object. Any error class can cross the worker this way. The TypeScript guideline now says so.
- **The object:** `RenderError.toDiagnostic()` returns `code`, `kind`, `message`, `docsUrl` and, for `data-pending`, `data.ranges` as `[offset, length]` pairs (`ByteRange`). It has no hint, because `@laminapdf/render` has no hint table. The `docsUrl` base is hand-copied like the kinds, and [Q2l](../ready/Q2l-boundary-lists-generated.md), which generates the render tables, now lists it. A new test checks the `docsUrl` of every render code against `fixtures/codes.json`. The worker file shrank from 15,702 to 15,667 bytes gzipped.
- **The viewer:** its worker throws `RenderError("cancelled")` and `RenderError("disposed")`, and the viewer's `useThrowOnlyError` override is gone. The bare `Error` for a missing backend stays: only a viewer bug reaches it, since the client loads the backend before anything else, and the worker reports it as `worker-failed`, kind `internal`, which is the right kind. The render codes all mean something else: `render-backend-unavailable`, for one, is a library that failed to load.
- **Ranges on the page:** `LaminaError.ranges` still returns `{ offset, length }` objects, since that is its public API and `Document.ensure` reads it. What crosses the worker is pairs everywhere.

## Post-merge checks

None.

## Tests

The new worker test; existing contract suites.

## Docs to update

Plan §9.

## Risks and open questions

None.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
