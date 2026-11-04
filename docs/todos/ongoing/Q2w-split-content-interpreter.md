# Q2w — Split the content interpreter's `execute`

| Field | Value |
| --- | --- |
| ID | `Q2w` |
| Status | `ongoing` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2a](../done/Q2a-maintainability-checks-rust.md) |
| Blocks | None |
| ADRs | None |
| Branch | `claude/working-session-9g3nst` |
| PR title | `refactor(content): dispatch operators to per-group handlers` |
| Size | S |
| PR | — |

## Goal

`laminapdf-content/src/interp.rs`'s `execute` is 318 lines behind an `#[expect(too_many_lines)]` whose reason is "one match arm per operator". This todo makes the match a table of one-line arms that dispatch to per-group handlers, removes the `#[expect]` and brings the file under budget.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `laminapdf-content/src/interp.rs:383`.

## Scope

**In scope**

- Handlers per operator group: path, text, graphics state, colour, marked content, XObjects; the `#[expect]` removed; the baseline entry removed.

**Out of scope** (and which todo covers it)

- The COS reader ([Q2q](./Q2q-split-cos-reader.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Extract one group per commit, running the content tests and the operator fixture after each.
- [ ] Remove the `#[expect]` and the baseline entry; run the differential and a short `content` fuzz run.

## Acceptance criteria

- [ ] No behaviour change; no `#[expect(clippy::too_many_lines)]` in `interp.rs`; the file is under budget.

## Post-merge checks

None.

## Tests

Existing suites, the operator fixture, a short fuzz run.

## Docs to update

Plan §9.

## Risks and open questions

None.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
