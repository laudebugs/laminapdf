# Q2v — One did-you-mean rule, pinned in both languages

| Field | Value |
| --- | --- |
| ID | `Q2v` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | None |
| ADRs | None |
| Branch | `claude/working-session-9g3nst` |
| PR title | `fix(wasm,sdk): one did-you-mean tie rule, pinned by a shared fixture` |
| Size | S |
| PR | [#214](https://github.com/laudebugs/laminapdf/pull/214) |

## Goal

Did-you-mean suggestions are written in both Rust (`laminapdf-wasm`) and TypeScript (`@laminapdf/sdk`), and they break ties differently: Rust sorts by distance and then name, TypeScript keeps input order. The same typo gets different suggestions depending on which layer raised the error. This todo picks one rule and pins it with a shared fixture.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); the [maintainability guideline](../../guidelines/maintainability.md) ("One home per concept"); `laminapdf-wasm/src/error.rs:128-158`, `packages/sdk/src/errors.ts:150-169`.

## Scope

**In scope**

- The rule: sort by distance, then by name.
- `fixtures/did-you-mean.json` (input, candidates, expected suggestions), run by a Rust test and a Vitest test.

**Out of scope** (and which todo covers it)

- Error shapes ([Q2j](../done/Q2j-boundary-errors-rust.md), [Q2k](../done/Q2k-boundary-errors-ts.md)).

## Tasks

- [x] Claim by opening the work PR (#214) off an up-to-date `main`. The cloud session that did the work had a plain clone and an assigned branch, so it worked on that branch instead of a worktree.
- [x] Write the fixture with tie cases; write both tests; watch the TypeScript one fail. `fixtures/did-you-mean.json` has eight cases; the Rust test replaced the two hand-written ones it covers. Before the fix, TypeScript failed the three tie cases.
- [x] Fix the TypeScript sort: distance, then code point, as `laminapdf-wasm` sorts.

## Acceptance criteria

- [x] Both languages give the same suggestions for every fixture case.

## Post-merge checks

None.

## Tests

The two new tests.

## Docs to update

Plan §9.

## Risks and open questions

None.

## Definition of done

- [x] Claimed: the work PR moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] Work PR opened; **PR** field filled in. It came from the session's assigned branch `claude/working-session-9g3nst` in a plain clone, not from a worktree or a `fix/…` branch (see Tasks)
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
