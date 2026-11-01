# Q2h — One ToUnicode writer and one FontDescriptor builder (fixes split surrogates)

| Field | Value |
| --- | --- |
| ID | `Q2h` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | [Q2r](./Q2r-split-surface-modules.md) |
| ADRs | None |
| Branch | `fix/q2h-text-writers-dedup` |
| PR title | `fix(text): one ToUnicode writer and one FontDescriptor builder` |
| Size | S |
| PR | — |

## Goal

lamina-text has two ToUnicode CMap writers that have drifted: the simple-font one caps text with `encode_utf16().take(MAX_DEST_UNITS)` and can split a surrogate pair. FontDescriptor building is written twice too, differing on `CapHeight` without saying why, and two helpers turn numbers into PDF objects with different rounding. This todo leaves one of each.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `lamina-text/src/{simple/tounicode.rs:36, embed/tounicode.rs:42, embed/type0.rs:62-82,206, simple/mod.rs:184-202, embed/subset.rs:278}`, `lamina-build/src/page.rs:334`.

## Scope

**In scope**

- A `tounicode` module with `Codespace::{OneByte, TwoByte}`; a `descriptor(…)` builder with a comment on `CapHeight`; number helpers next to `Object` (an integer when integral, else a real, with an optional precision); callers migrated.

**Out of scope** (and which todo covers it)

- The lexer and PNG filter ([Q2i](./Q2i-one-lexer-one-png-filter.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Write a test for a 257-unit text ending in an astral character and watch the simple-font writer split it.
- [ ] Merge the writers behind `Codespace`; migrate both callers.
- [ ] Merge the descriptor builders; document `CapHeight`.
- [ ] Add the number helpers; migrate lamina-build and lamina-text.
- [ ] Run the text reference PDFs and extraction facts; explain any changed file in the PR.

## Acceptance criteria

- [ ] The surrogate test passes.
- [ ] One writer, one descriptor builder, one number helper.

## Post-merge checks

None.

## Tests

The new regression test; text reference PDFs and extraction facts.

## Docs to update

Plan §9.

## Risks and open questions

A reference PDF may change if it relied on the split: the extraction facts show whether extractors read it better.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
