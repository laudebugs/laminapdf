# Q1d — API fixes the examples need (Rust and TypeScript)

| Field | Value |
| --- | --- |
| ID | `Q1d` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | [Q1f](./Q1f-examples-readable.md) |
| ADRs | Follows 0028 (façade); may amend it for `save` |
| Branch | `feat/q1d-example-api-cleanups` |
| PR title | `feat(api): small API fixes that make the examples read cleanly` |
| Size | S |
| PR | — |

## Goal

Some noise in the examples comes from the API, not the examples. Fix it at the source so Q1f's rewrite is clean: drop the misleading `#[must_use]` on `TextRun`, add `RectPt::from_points`, make `Document.save()` return `Uint8Array<ArrayBuffer>`, and smooth the façade's most-shown calls (`save` without an explicit `CancelToken`, `Default` and `new()` parity on options, `SetInfo::patch`).

## Context

- [Quality plan §2.2](../../plans/quality.md#22-code-examples) (12 `let _ = s.text(…)?` lines; three hand-written `rect()` helpers; the Worker's cast; 10 `CancelToken::new()` in examples).
- `crates/lamina-build/src/surface/text.rs` (`TextRun`'s `#[must_use]`), `crates/lamina-model/src/geometry.rs` (`PointPt::from_points`), `packages/sdk/src/document.ts` (`save`), `crates/lamina/src/document.rs` (`save`), `crates/lamina-ops/src/op.rs` (`SetInfo::new(patch, None)`).
- ADR 0028 (façade rules) for the save/cancel shape.

## Scope

**In scope**

- Remove `#[must_use]` from `TextRun` (text is drawn either way); remove the `let _ =` it forced in examples, docs and tests.
- `RectPt::from_points(x, y, w, h)` beside `PointPt::from_points`; use it in the examples instead of local `rect()` helpers.
- TS `Document.save()`/`saveWithReport()` return `Uint8Array<ArrayBuffer>`; drop the Worker example's cast.
- Façade: `Document::save(&SaveOptions)` plus `save_with_cancel(&SaveOptions, &CancelToken)` (or the ADR 0028-consistent equivalent); `Default` and `new()` on every options type; `SetInfo::patch(p)`.
- Update rustdoc examples and CHANGELOG via a changeset (API change while alpha).

**Out of scope** (and which todo covers it)

- Readability refactors of the examples themselves ([Q1f](./Q1f-examples-readable.md)).
- Façade renames ([Q2s](./Q2s-facade-names-and-errors.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Make each change with tests; run the example goldens (`crates/lamina/tests/examples.rs`, the TS suite) to prove output bytes are unchanged.

## Acceptance criteria

- [ ] The goldens are byte-identical (no output change).
- [ ] No `let _ =` remains in `crates/lamina/examples/` or the façade docs.
- [ ] Changeset added.

## Post-merge checks

None.

## Tests

Unit tests for `RectPt::from_points`, the new `save` overload and `SetInfo::patch`; the existing example goldens.

## Docs to update

Façade rustdoc, `packages/sdk` TSDoc, CLAUDE.md façade paragraph if signatures change, a changeset.

## Risks and open questions

The save signature is a façade rule; if ADR 0028 needs an amendment, write it in this PR.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
