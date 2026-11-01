# Q2a — Maintainability checks for Rust: module budget, lint additions and the `check maintainability` step

| Field | Value |
| --- | --- |
| ID | `Q2a` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | None |
| ADRs | None |
| Branch | `ci/q2a-maintainability-checks` |
| PR title | `ci(xtask): check maintainability (module budget, expect counter, duplicate names) and new workspace lints` |
| Size | M |
| PR | — |

## Goal

Make the [maintainability guideline](../../guidelines/maintainability.md) checkable for Rust. Add `cargo xtask check maintainability` to `cargo xtask check`, with a module-size budget (800 lines of non-test code, a baseline that only goes down), a counter for `#[expect]` on size lints, bans on cross-crate `#[path]` includes (baseline until [Q2c](./Q2c-test-fixtures-crate.md)), duplicate public type names across crates (allow-list) and two-path re-exports in the façade; switch on `clippy::cognitive_complexity` and `unreachable_pub`.

## Context

- [Maintainability guideline](../../guidelines/maintainability.md), [Quality plan §4.3](../../plans/quality.md#4-standards).
- Workspace `Cargo.toml` `[workspace.lints]` (pedantic already enforced, so `too_many_lines` at 100 passes everywhere; 3 `#[expect(clippy::too_many_lines)]` today), `clippy.toml`, `tools/xtask/`.

## Scope

**In scope**

- `check maintainability`: module budget with `tools/xtask/maintainability-baseline.toml` (today's over-budget files: `lamina-cos/src/reader/mod.rs`, `lamina-content/src/interp.rs`, `lamina-text/src/extract/font.rs`, `lamina-build/src/surface/{mod,png}.rs`, `lamina-wasm/src/{builder,build_dto}.rs`, with their counts); the `#[expect]` counter; the cross-crate `#[path]` ban with baseline; `pub struct|enum` names defined in two crates (allow-list file with reasons); the façade re-export collision check.
- Lints: `cognitive_complexity = "warn"` (nursery; comment), `unreachable_pub = "warn"`, explicit `too-many-lines-threshold = 100` in `clippy.toml`; fix the `unreachable_pub` hits.
- A hook other todos extend: Q2c, Q2e, Q2i and Q2t add their bans to the same step.

**Out of scope** (and which todo covers it)

- TypeScript checks ([Q2b](./Q2b-maintainability-checks-ts.md)).
- Fixing the over-budget modules (Q2q, Q2r) and the duplicates (Q2c–Q2s).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Write the step with unit tests; generate the baseline; add the lints and fix their hits; wire into `check` and CI's fast lane.

## Acceptance criteria

- [ ] `cargo xtask check maintainability` passes on `main` with the baseline; adding 1 line to a baselined file or a new 801-line module fails.
- [ ] The new lints are on and clean.

## Post-merge checks

None.

## Tests

xtask unit tests with tiny source trees.

## Docs to update

Rust guideline (size limits section), maintainability guideline, CLAUDE.md commands, plan §9.

## Risks and open questions

Counting non-test lines: exclude `#[cfg(test)] mod tests { … }` blocks and comments/blank lines; document the method in the guideline.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
