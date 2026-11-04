# Q2d — One PDF serialiser: the test `Writer` built on laminapdf-save's helpers

| Field | Value |
| --- | --- |
| ID | `Q2d` |
| Status | `ongoing` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2c](../done/Q2c-test-fixtures-crate.md) |
| Blocks | None |
| ADRs | None |
| Branch | `refactor/q2d-one-serialiser` |
| PR title | `refactor(save): build the test Writer on the save writers' helpers and take it out of the public API` |
| Size | M |
| PR | [#213](https://github.com/laudebugs/laminapdf/pull/213) |

## Goal

`laminapdf-save/src/legacy.rs`, the spike-era `Writer` and `incremental_update`, re-implements the classic xref, xref streams, object-stream packing, object framing and the header that `xref.rs` and `full.rs` already have. It also sits in laminapdf-save's public API, although only tests and doc examples use it. This todo rebuilds it on the shared helpers and moves it into `laminapdf-test-fixtures`.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `laminapdf-save/src/legacy.rs:62–481`, `full.rs`, `xref.rs`, `lib.rs` (`pub use legacy::…`); users in `laminapdf-ops/src/{tx,record}.rs` tests and in the docs of laminapdf-ops, laminapdf-model and laminapdf-content.

## Scope

**In scope**

- `Writer` and `incremental_update` rebuilt on `xref::Rows`, `classic_table`, `stream_table` and `object_stream`; every byte they produce unchanged.
- Moved into `laminapdf-test-fixtures` (which then depends on laminapdf-save), or behind a `test-support` feature if a dependency cycle forces it; `laminapdf-save`'s public API keeps only the product writers.
- `XrefEol` stays in `options.rs`.

**Out of scope** (and which todo covers it)

- Other test helpers ([Q2c](../done/Q2c-test-fixtures-crate.md), [Q2x](../ready/Q2x-test-fixtures-migration.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Rewrite each part of `legacy.rs` on the shared helper, running the tests and the frozen reference files (`hello.pdf`, `imaging-demo.pdf`) after each.
- [ ] Decide the home (fixtures crate or feature) by checking the dependency graph; move it; update dev-dependencies.
- [ ] Update the doc examples that use it; remove the `pub use`.

## Acceptance criteria

- [ ] Every test and reference file is unchanged.
- [ ] No second implementation of xref or object-stream writing in the workspace.

## Post-merge checks

None.

## Tests

Existing tests and reference files.

## Docs to update

The saving paragraph in CLAUDE.md, the laminapdf-save README, plan §9.

## Risks and open questions

If the layout differs (end-of-line markers, padding), keep the old layout behind options rather than changing reference files.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
