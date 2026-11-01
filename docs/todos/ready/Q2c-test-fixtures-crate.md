# Q2c — `lamina-test-fixtures`: one home for Rust test helpers

| Field | Value |
| --- | --- |
| ID | `Q2c` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | [Q2d](./Q2d-test-writer-on-save-helpers.md) |
| ADRs | None |
| Branch | `refactor/q2c-test-fixtures` |
| PR title | `refactor(tests): lamina-test-fixtures crate for shared test helpers` |
| Size | M |
| PR | — |

## Goal

Rust test helpers are copied across crates (`fixtures_dir` 11×, `fixture` ~9×, `corpus` 4×, the classic-xref builder 5×, `Rng` 2×, ~15 `ByteSource` fakes) and linked by 35 `#[path = "../../other-crate/…"]` includes. Create a dev-only, unpublished `crates/lamina-test-fixtures` with **no lamina dependencies** (so `lamina-cos`'s own unit tests can use it), move the shared helpers there, and remove every cross-crate `#[path]`.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse), [maintainability guideline](../../guidelines/maintainability.md) rule 6; `code-organization.md` §11 (its "no lamina-common for later" rule: this crate has present users and a name saying what it does; amend the guideline to say so).
- Locations: `tests/common/mod.rs` of lamina-cos, -model, -save, -text, -ops, -build, -content, -source, -render-pdfium; `lamina-cos/tests/common/classic.rs` (and its header explaining the `#[path]` workaround); `lamina/tests/facade.rs:16`, `lamina-render/tests/hayro.rs:22`, `lamina-text/tests/extract_fonts.rs:23`; `ByteSource` fakes in `lamina-cos/tests/{window_bounds,review_fixes,source}.rs` and `lamina-cos/src/source.rs`.

## Scope

**In scope**

- New crate (`publish = false`, `layer = "dev"` or the closest existing layer with a note): `fixtures_dir`, `fixture`, `corpus`, `classic`/`stream_body`, `Rng`, a byte-slice `read_at` helper for fakes, the codes-table loader now shared via `#[path]`.
- Migrate every crate; delete the copies and the `#[path]` includes; add the ban to `check maintainability` (from [Q2a](./Q2a-maintainability-checks-rust.md), or a grep step here if Q2a hasn't landed).
- Amend code-organization.md §11.

**Out of scope** (and which todo covers it)

- The legacy `Writer` ([Q2d](./Q2d-test-writer-on-save-helpers.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Create the crate; move helpers one at a time; run each crate's tests after each move.

## Acceptance criteria

- [ ] No `#[path = "../../` outside the crate's own tree; no second definition of the moved helpers.
- [ ] Every test passes; no test changed what it asserts.

## Post-merge checks

None.

## Tests

Every crate's existing tests.

## Docs to update

code-organization.md §11, CLAUDE.md layout, plan §9.

## Risks and open questions

Cyclic dev-dependencies: the crate depends on no lamina crate, so any crate can dev-depend on it.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
