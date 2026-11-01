# Q2k — One error wire shape in TypeScript (fixes lost render ranges)

| Field | Value |
| --- | --- |
| ID | `Q2k` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2j](./Q2j-boundary-errors-rust.md) |
| Blocks | None |
| ADRs | None |
| Branch | `fix/q2k-ts-error-shape` |
| PR title | `fix(sdk,render,viewer): one error wire shape; render data-pending keeps its ranges across the worker` |
| Size | M |
| PR | — |

## Goal

The worker turns a render error's `ranges` into `{offset, length}` objects, but `LaminaError.ranges` only accepts `[offset, length]` tuples (what Rust emits), so a render `data-pending` crossing the worker arrives with no ranges. Render errors also have an unused `toJSON`/`reviveRenderError` pair, the worker duck-types instead, and the viewer throws plain objects. Use one shape (tuples) and one path.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `packages/sdk/src/worker/server.ts:116-136,123`, `packages/sdk/src/errors.ts:97`, `packages/render/src/index.ts:199-222`, `packages/viewer/src/worker/document.ts:39,93,118`; TypeScript guideline ("never throw plain objects").

## Scope

**In scope**

- `RenderError.toDiagnostic()` returning the ADR 0026 object; the worker's `diagnostic()` calls it; delete `reviveRenderError`/`SerializedRenderError`.
- Ranges as tuples everywhere.
- Viewer throws `RenderError("cancelled" | "disposed")` (and maps its bare `Error`).
- Test: a render `data-pending` crossing the worker keeps its ranges.

**Out of scope** (and which todo covers it)

- Generated render codes ([Q2l](./Q2l-boundary-lists-generated.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Fix, delete, test in Node and browsers.

## Acceptance criteria

- [ ] The new crossing test passes; no plain-object throws in `packages/*/src` (Biome `useThrowOnlyError` from [Q2b](./Q2b-maintainability-checks-ts.md), or a grep here).

## Post-merge checks

None.

## Tests

New worker test; existing contract suites.

## Docs to update

Plan §9.

## Risks and open questions

None.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
