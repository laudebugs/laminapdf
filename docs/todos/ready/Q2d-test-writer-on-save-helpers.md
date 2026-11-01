# Q2d — One PDF serialiser: the test `Writer` built on lamina-save's own xref and object-stream code

| Field | Value |
| --- | --- |
| ID | `Q2d` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2c](./Q2c-test-fixtures-crate.md) |
| Blocks | None |
| ADRs | None |
| Branch | `refactor/q2d-one-serialiser` |
| PR title | `refactor(save): build the test Writer on the save writers' helpers and move it out of the public API` |
| Size | M |
| PR | — |

## Goal

`lamina-save/src/legacy.rs` (the spike-era `Writer` and `incremental_update`) re-implements the classic xref, xref streams, object-stream packing, object framing and the header that `xref.rs` and `full.rs` already have, and it sits in lamina-save's public API although only tests and doc examples use it. Rebuild it on `xref::Rows`, `classic_table`, `stream_table` and `object_stream`, and move it into `lamina-test-fixtures` (or a `test-support` feature) so lamina-save's public surface is the product API only.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `lamina-save/src/legacy.rs:62–481`, `full.rs`, `xref.rs`, `lib.rs` (`pub use legacy::…`); users: `lamina-ops/src/{tx,record}.rs` tests, docs of lamina-ops, lamina-model, lamina-content.

## Scope

**In scope**

- Rebuild on the shared helpers; keep the frozen goldens it produces byte-identical (`hello.pdf`, `imaging-demo.pdf` and the test files); move it; update the dev-dependencies and doc examples.

**Out of scope** (and which todo covers it)

- Other test helpers ([Q2c](./Q2c-test-fixtures-crate.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Refactor with the existing tests as the guard; then move.

## Acceptance criteria

- [ ] Every test and golden unchanged; `lamina-save`'s public API no longer exports `Writer`/`incremental_update`.
- [ ] No second implementation of xref/object-stream writing in the workspace.

## Post-merge checks

None.

## Tests

Existing tests and goldens.

## Docs to update

CLAUDE.md saving paragraph ("the spike-era Writer"), lamina-save README, plan §9.

## Risks and open questions

If byte layout differs (EOL, padding), keep the legacy layout behind options rather than changing goldens.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
