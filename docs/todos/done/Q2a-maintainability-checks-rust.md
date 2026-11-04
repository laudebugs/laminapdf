# Q2a — `check maintainability` for Rust: module budget and new lints

| Field | Value |
| --- | --- |
| ID | `Q2a` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | [Q2c](./Q2c-test-fixtures-crate.md), [Q2e](./Q2e-error-policy-consolidation.md), [Q2g](../ready/Q2g-one-matrix.md), [Q2i](../ready/Q2i-one-lexer-one-png-filter.md), [Q2q](../ready/Q2q-split-cos-reader.md), [Q2r](../ready/Q2r-split-surface-modules.md), [Q2s](../ready/Q2s-facade-names.md), [Q2t](../ready/Q2t-history-out-of-names.md), [Q2w](./Q2w-split-content-interpreter.md) |
| ADRs | None |
| Branch | `ci/q2a-maintainability-checks-rust` |
| PR title | `ci(xtask): check maintainability (module budget, expect count, duplicate names) and new workspace lints` |
| Size | M |
| PR | [#168](https://github.com/laudebugs/lamina/pull/168) |

## Goal

This todo adds `cargo xtask check maintainability` to `cargo xtask check`, the home for every Rust rule in the [maintainability guideline](../../guidelines/maintainability.md) that a tool can check. It starts with the module budget (800 lines of non-test code, against a baseline that only shrinks), a count of `#[expect]` on size lints, duplicate public type names across crates and colliding façade re-exports. It also switches on `clippy::cognitive_complexity` and `unreachable_pub`. Later todos add their bans to the same step.

## Context

- The [maintainability guideline](../../guidelines/maintainability.md) ("Size", "Public surface and tests", "How it's checked"); [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse).
- Workspace `[workspace.lints]` (pedantic already enforced, so `too_many_lines` at 100 passes everywhere; three `#[expect(clippy::too_many_lines)]` today), `clippy.toml`, `tools/xtask/`.

## Scope

**In scope**

- `check maintainability` with:
  - the module budget and `tools/xtask/maintainability-baseline.toml` (today's files over budget and their non-test counts);
  - the `#[expect]` counter for size lints;
  - `pub struct` and `pub enum` names defined in two crates, against an allow-list with reasons;
  - two re-exports with one name in the `lamina` façade.
- A small registry of named checks so later todos add theirs (Q2c, Q2e, Q2i, Q2t).
- Lints: `cognitive_complexity = "warn"` (a nursery lint; say so in a comment), `unreachable_pub = "warn"`, and `too-many-lines-threshold = 100` in `clippy.toml`; fix the hits.

**Out of scope** (and which todo covers it)

- TypeScript checks ([Q2b](../done/Q2b-ts-lint-rules.md), [Q2u](../done/Q2u-ts-dependency-tools.md)).
- Shrinking the baseline (Q2q, Q2w, Q2r) and removing the duplicates (the other Q2 todos).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Write the non-test line counter (skip comments, blank lines and `#[cfg(test)]` blocks) with unit tests.
- [x] Generate the baseline from today's code and commit it.
- [x] Write the `#[expect]` counter, the duplicate-name check (with the allow-list file listing today's duplicates and why) and the façade collision check.
- [x] Add the registry so a check is a function plus a name.
- [x] Add the two lints and the `clippy.toml` threshold; fix every `unreachable_pub` hit.
- [x] Wire the step into `cargo xtask check` and CI's fast lane.

## Acceptance criteria

- [x] `cargo xtask check maintainability` passes on `main`.
- [x] One more line in a baselined file, a new 801-line module, or a new duplicate name fails it (unit tests).
- [x] The two lints are on and clean.

## Post-merge checks

None.

## Tests

Unit tests in `tools/xtask` with small source trees for every check.

## Docs to update

rust.md (size limits), the maintainability guideline if anything differs, the commands list in CLAUDE.md, plan §9.

## Risks and open questions

The non-test line count must be stable and documented: describe the method in the guideline.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
