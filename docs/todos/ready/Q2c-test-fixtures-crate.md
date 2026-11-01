# Q2c — `lamina-test-fixtures` for the core crates' test helpers

| Field | Value |
| --- | --- |
| ID | `Q2c` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2a](../ongoing/Q2a-maintainability-checks-rust.md) |
| Blocks | [Q2d](./Q2d-one-serialiser.md), [Q2x](./Q2x-test-fixtures-migration.md) |
| ADRs | None |
| Branch | `refactor/q2c-test-fixtures-crate` |
| PR title | `refactor(tests): lamina-test-fixtures for shared Rust test helpers (core crates)` |
| Size | M |
| PR | — |

## Goal

Rust test helpers are copied across crates and linked by 35 `#[path = "../../other-crate/…"]` includes. This todo creates an unpublished, dev-only `crates/lamina-test-fixtures` with no lamina dependencies, so `lamina-cos`'s own unit tests can use it too. It moves the shared helpers there and migrates lamina-cos, lamina-model, lamina-save and lamina-ops. [Q2x](./Q2x-test-fixtures-migration.md) migrates the rest.

## Context

- The [maintainability guideline](../../guidelines/maintainability.md) ("Public surface and tests"); [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse).
- Copies: `fixtures_dir` (11), `fixture` (about 9), `corpus` (4), the classic-xref builder (5; canonical at `lamina-cos/tests/common/classic.rs`, whose header explains the `#[path]` workaround), `Rng` (2), about 15 `ByteSource` fakes (`lamina-cos/tests/{window_bounds,review_fixes,source}.rs`, `lamina-cos/src/source.rs`), and the codes-table loader shared through `#[path]`.
- code-organization.md §6 (shows `tests/common/mod.rs` for shared helpers) and §11 ("no lamina-common for later"); `tools/xtask/src/metadata.rs` (layers are `core`, `edge` and `app`).

## Scope

**In scope**

- The crate: `publish = false`; `fixtures_dir`, `fixture`, `corpus`, `classic` and `stream_body`, `Rng`, a byte-slice `read_at` helper for fakes, the codes-table loader.
- A layer for it: extend `metadata.rs` with a `dev` layer, or use `app` with a note.
- lamina-cos, lamina-model, lamina-save and lamina-ops migrated; their copies and `#[path]` includes deleted.
- A ban in `check maintainability`: no cross-crate `#[path]` and no second `fn fixtures_dir` or `fn classic` outside the crate, with today's remaining users (Q2x's crates) in a baseline.
- code-organization §6 and §11 amended: shared test helpers live here; the crate is allowed because it has present users and says what it does.

**Out of scope** (and which todo covers it)

- The other crates ([Q2x](./Q2x-test-fixtures-migration.md)); the test `Writer` ([Q2d](./Q2d-one-serialiser.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Create the crate and its layer; move the helpers with their tests.
- [ ] Migrate lamina-cos (unit and integration tests), then lamina-model, lamina-save and lamina-ops, running each crate's tests after each.
- [ ] Delete the copies and `#[path]` includes in those crates.
- [ ] Add the ban with its baseline to `check maintainability`.
- [ ] Amend code-organization §6 and §11; add the crate to CLAUDE.md's layout.

## Acceptance criteria

- [ ] No `#[path = "../../` and no copied helper in the four crates.
- [ ] Every test passes and asserts what it did before.

## Post-merge checks

None.

## Tests

Each migrated crate's tests.

## Docs to update

code-organization §6 and §11, CLAUDE.md layout, plan §9.

## Risks and open questions

Dev-dependency cycles can't happen: the crate depends on no lamina crate.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
