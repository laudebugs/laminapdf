# Q1d — API fixes the examples need

| Field | Value |
| --- | --- |
| ID | `Q1d` |
| Status | `ongoing` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | [Q1f](../ready/Q1f-examples-readable.md) |
| ADRs | Follows 0028; may amend it for `save` |
| Branch | `feat/q1d-example-api-cleanups` |
| PR title | `feat(api): small API fixes that make the examples read cleanly` |
| Size | S |
| PR | — |

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
- Façade renames ([Q2s](./Q2s-facade-names.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Remove the attribute and every `let _ =` it forced; build the workspace.
- [ ] Add `RectPt::from_points` with tests; replace the examples' helpers.
- [ ] Change the TypeScript return types; remove the cast; run the TS suites.
- [ ] Add the façade `save` overload, the option constructors and `SetInfo::patch`, with tests; update rustdoc examples.
- [ ] Run both example reference suites to prove the output bytes are unchanged; add the changeset.

## Acceptance criteria

- [ ] Every reference PDF is byte-identical.
- [ ] No `let _ =` remains in `crates/laminapdf/examples/` or the façade docs.
- [ ] A changeset is added; ADR 0028 is amended if the save shape needs it.

## Post-merge checks

None.

## Tests

Unit tests for `RectPt::from_points`, the new `save` and `SetInfo::patch`; the existing reference suites.

## Docs to update

Façade rustdoc, `@laminapdf/sdk` TSDoc, the façade paragraph in CLAUDE.md if signatures change, a changeset.

## Risks and open questions

The save signature is a façade rule: amend ADR 0028 in this PR if it needs to change.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
