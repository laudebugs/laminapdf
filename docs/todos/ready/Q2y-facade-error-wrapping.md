# Q2y — Decide and implement how the façade wraps crate errors

| Field | Value |
| --- | --- |
| ID | `Q2y` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | None |
| ADRs | Amends 0028 |
| Branch | `docs/q2y-facade-error-wrapping` |
| PR title | `docs(adr): decide how the lamina façade wraps the errors of the crates below it` |
| Size | S |
| PR | — |

## Goal

The façade re-exports every wrapped crate's error type (`CosError`, `OpsError`, `SaveError`, `SinkError`, `TextError`, `BuildError`), against rust.md's "SHOULD NOT expose another crate's error type", so every inner crate's semver becomes the façade's. This todo records the decision in ADR 0028: keep wrapping and amend rust.md, or make the variants opaque behind `code()` and `ErrorCode`. It then implements the choice.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `lamina/src/error.rs:36-52`, `lamina/src/lib.rs:125-132`; rust.md (errors); ADR 0026 and ADR 0028.

## Scope

**In scope**

- An ADR 0028 amendment with both options, their costs and the choice; the code change it implies; rust.md amended if the rule changes; a changeset if the API changes.

**Out of scope** (and which todo covers it)

- Façade renames ([Q2s](./Q2s-facade-names.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Draft the amendment with `/writing`; list every caller that matches on an inner error type.
- [ ] If the choice changes the public API in a way users would notice, ask the owner before implementing it.
- [ ] Implement the choice; run every suite.
- [ ] Run `/writing-review` on the amendment after opening the PR.

## Acceptance criteria

- [ ] ADR 0028 records the decision, and the code and rust.md agree with it.

## Post-merge checks

None.

## Tests

Existing suites and the error contract test.

## Docs to update

ADR 0028, rust.md (if amended), CLAUDE.md's façade paragraph, plan §9.

## Risks and open questions

An API break while alpha: say so in the changeset.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
