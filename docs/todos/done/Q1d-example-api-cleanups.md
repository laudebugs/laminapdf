# Q1d — API fixes the examples need

| Field | Value |
| --- | --- |
| ID | `Q1d` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | [Q1f](./Q1f-examples-readable.md) |
| ADRs | Follows 0028; may amend it for `save` |
| Branch | `feat/q1d-example-api-cleanups` |
| PR title | `feat(api): small API fixes that make the examples read cleanly` |
| Size | S |
| PR | [#192](https://github.com/laudebugs/laminapdf/pull/192) |

## Goal

Some of the noise in the examples comes from the API, not the examples. This todo fixes it at the source so the example rewrite is clean: no misleading `#[must_use]` on `TextRun`, a `RectPt::from_points`, a TypeScript `save()` that returns `Uint8Array<ArrayBuffer>`, and façade calls that read naturally (`save` without an explicit `CancelToken`, `Default` and `new()` on options, `SetInfo::patch`).

## Context

- [Quality plan §2.2](../../plans/quality.md#22-code-examples): 12 `let _ = s.text(…)?` lines, three hand-written `rect()` helpers, the Cloudflare Worker's cast, 10 `CancelToken::new()` calls in examples.
- `crates/laminapdf-build/src/surface/text.rs` (`TextRun`'s `#[must_use]`), `crates/laminapdf-model/src/geometry.rs` (`PointPt::from_points`), `packages/sdk/src/document.ts` (`save`), `crates/laminapdf/src/document.rs` (`save`), `crates/laminapdf-ops/src/op.rs` (`SetInfo::new(patch, None)`).
- ADR 0028 (façade rules), which governs the save and cancel shape.

## Scope

**In scope**

- Remove `#[must_use]` from `TextRun` (the text is drawn either way) and the `let _ =` it forced in examples, docs and tests.
- `RectPt::from_points(x, y, w, h)` next to `PointPt::from_points`; the examples use it instead of their own `rect()`.
- TypeScript `Document.save()` and `saveWithReport()` return `Uint8Array<ArrayBuffer>`; the Cloudflare Worker example loses its cast.
- Façade: `Document::save(&SaveOptions)` plus `save_with_cancel(&SaveOptions, &CancelToken)`, or the shape ADR 0028 allows; `Default` and `new()` on every options type; `SetInfo::patch(p)`.
- A changeset for the API change.

**Out of scope** (and which todo covers it)

- Readability rewrites of the examples ([Q1f](./Q1f-examples-readable.md)).
- Façade renames ([Q2s](../ready/Q2s-facade-names.md)).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Remove the attribute and every `let _ =` it forced; build the workspace.
- [x] Add `RectPt::from_points` with tests; replace the examples' helpers.
- [x] Change the TypeScript return types; remove the cast; run the TS suites. (The Worker example keeps its now-redundant cast: removing it would make that file strict before [Q1g](../ready/Q1g-worker-examples.md) clears it.)
- [x] Add the façade `save` overload, the option constructors and `SetInfo::patch`, with tests; update rustdoc examples.
- [x] Run both example reference suites to prove the output bytes are unchanged; add the changeset.

## Acceptance criteria

- [x] Every reference PDF is byte-identical, except `invoice.pdf`, which Q1f changed on purpose (see Q1f's Outcome).
- [x] No `let _ =` remains in `crates/laminapdf/examples/` or the façade docs.
- [x] A changeset is added; ADR 0028 is amended if the save shape needs it.

## Post-merge checks

None.

## Tests

Unit tests for `RectPt::from_points`, the new `save` and `SetInfo::patch`; the existing reference suites.

## Docs to update

Façade rustdoc, `@laminapdf/sdk` TSDoc, the façade paragraph in CLAUDE.md if signatures change, a changeset.

## Risks and open questions

The save signature is a façade rule: amend ADR 0028 in this PR if it needs to change.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Outcome

- `Document::save(&SaveOptions)` runs to completion. `save_with(&SaveOptions, &CancelToken)` is the F10 form another thread can stop, named like `save_job_with`. ADR 0028 is not amended: F10 still holds through `save_with`. 31 `CancelToken::new()` arguments are gone.
- `TextRun` has no `#[must_use]`. Its impl explains, in an `expect`, why its builder methods may be ignored. 79 `let _ =` and `drop(…)` wrappers are gone.
- `RectPt::from_points` and `SetInfo::patch` are added, and `new()` on `BuildOptions`, `OpenOptions`, `PngOptions`, `RetryOptions` and `TextBoxOptions`. `LayoutOptions::new` already takes the box width and the text size.
- TypeScript `save()`, `saveWithReport()` and `SaveJob.step()` return `Uint8Array<ArrayBuffer>`, from the WASM binding's type up.
- It landed with [Q1f](./Q1f-examples-readable.md) in [#192](https://github.com/laudebugs/laminapdf/pull/192). Editing an example makes the examples checks strict on it, so this todo couldn't pass on its own.
