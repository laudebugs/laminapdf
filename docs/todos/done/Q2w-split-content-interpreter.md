# Q2w — Split the content interpreter's `execute`

| Field | Value |
| --- | --- |
| ID | `Q2w` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2a](./Q2a-maintainability-checks-rust.md) |
| Blocks | None |
| ADRs | None |
| Branch | `claude/working-session-9g3nst` |
| PR title | `refactor(content): dispatch operators to per-group handlers` |
| Size | S |
| PR | [#210](https://github.com/laudebugs/laminapdf/pull/210) |

## Goal

`laminapdf-content/src/interp.rs`'s `execute` is 318 lines behind an `#[expect(too_many_lines)]` whose reason is "one match arm per operator". This todo makes the match a table of one-line arms that dispatch to per-group handlers, removes the `#[expect]` and brings the file under budget.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `laminapdf-content/src/interp.rs:383`.

## Scope

**In scope**

- Handlers per operator group: path, text, graphics state, colour, marked content, XObjects; the `#[expect]` removed; the baseline entry removed.

**Out of scope** (and which todo covers it)

- The COS reader ([Q2q](../ready/Q2q-split-cos-reader.md)).

## Tasks

- [x] Claim by opening the work PR (#210) off an up-to-date `main`. The cloud session that did the work had a plain clone and an assigned branch, so it worked on that branch instead of a worktree.
- [x] Extract one group per commit, running the content tests and the operator fixture after each. After `interp.rs` moved to `interp/mod.rs`, the six groups went in this order: graphics state, path, text, colour, XObjects and inline images, marked content. Each commit also passed clippy and `cargo xtask check maintainability`.
- [x] Remove the `#[expect]` and the baseline entry; run the differential and a short `content` fuzz run. Moving the first group left `mod.rs` at 751 lines, so the baseline entry went then; the `#[expect]` went with the fifth group. `cargo xtask differential`, `cargo xtask differential --text` and `cargo xtask fuzz content --time 120` pass.

## Acceptance criteria

- [x] No behaviour change; no `#[expect(clippy::too_many_lines)]` in `interp.rs` (now `interp/mod.rs`); the file is under budget: 325 lines of non-test code, down from 823.

## Post-merge checks

None.

## Tests

Existing suites, the operator fixture, a short fuzz run.

## Docs to update

Plan §9.

## Risks and open questions

None.

## Definition of done

- [x] Claimed: the work PR moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] Work PR opened; **PR** field filled in. It came from the session's assigned branch `claude/working-session-9g3nst` in a plain clone, not from a worktree or a `refactor/…` branch (see Tasks)
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
