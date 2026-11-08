# Q2x — `laminapdf-test-fixtures` for the remaining crates

| Field | Value |
| --- | --- |
| ID | `Q2x` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2c](./Q2c-test-fixtures-crate.md) |
| ADRs | None |
| Branch | `refactor/q2x-test-fixtures-migration` |
| PR title | `refactor(tests): move the remaining crates' test helpers to laminapdf-test-fixtures` |
| Size | M |
| PR | [#215](https://github.com/laudebugs/laminapdf/pull/215) |

## Goal

This todo finishes [Q2c](../done/Q2c-test-fixtures-crate.md): it migrates laminapdf-content, laminapdf-text, laminapdf-build, laminapdf-source, laminapdf-render, laminapdf-render-pdfium and the `laminapdf` façade to `laminapdf-test-fixtures`, deletes their copies and `#[path]` includes, and empties the ban's baseline.

## Context

- [Q2c](../done/Q2c-test-fixtures-crate.md) and its baseline; the copies in `tests/common/mod.rs` of each crate, plus `lamina/tests/facade.rs:16`, `lamina/tests/{page_text_snapshot,reading_order}.rs`, `laminapdf-render/tests/hayro.rs:22`, `laminapdf-text/tests/{extract_fonts,regressions}.rs`.

## Scope

**In scope**

- The seven crates migrated; the baseline emptied, so the ban is strict.

**Out of scope** (and which todo covers it)

- New helpers.

## Tasks

- [x] Claim: the work PR's first commit moves this file to `ongoing/` (CLAUDE.md's current rule), from a fresh worktree off `main`.
- [x] Migrate one crate at a time, running its tests after each.
- [x] Delete the copies and `#[path]` includes; empty the baseline.
- [x] Run the whole workspace's tests.

## Acceptance criteria

- [x] The ban's baseline is empty and `check maintainability` passes.
- [x] Every test passes and asserts what it did before.

## Post-merge checks

None.

## Tests

Each migrated crate's tests.

## Docs to update

Plan §9.

## Risks and open questions

None.

## Definition of done

- [x] Claimed: the work PR's first commit moved this file `ready/` → `ongoing/` (Status `ongoing`, links updated)
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
