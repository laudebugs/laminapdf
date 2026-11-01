# Q2e — One error-propagation policy per kind (fixes swallowed I/O errors)

| Field | Value |
| --- | --- |
| ID | `Q2e` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2a](../done/Q2a-maintainability-checks-rust.md) |
| Blocks | [Q2q](./Q2q-split-cos-reader.md) |
| ADRs | Follows 0003 |
| Branch | `fix/q2e-error-policy-consolidation` |
| PR title | `fix(cos,text): one set of error-propagation policies; extraction propagates source errors` |
| Size | S |
| PR | — |

## Goal

Text extraction decides which errors propagate with its own `fatal()`, which leaves out `Error::Source`, so an I/O failure from a `FileSource` reads as "no font" (against ADR 0003). lamina-cos has its own scattered set: `tolerant` and its duplicate `or_repair`, a `must_propagate` wrapper, `absent_if_malformed`, and inline matches for ending the page walk and security refusals. This todo puts every policy in `lamina_cos::error` and makes extraction use one.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); ADR 0003; `lamina-cos/src/error.rs:246,267`, `lamina-cos/src/reader/mod.rs:274,280,290,298,408`, `lamina-text/src/extract/font.rs:137`, `lamina-ops/src/error.rs:150`.

## Scope

**In scope**

- Public, documented policies in `error.rs`: `tolerant`, `absent_if_malformed`, `ends_page_walk`, `is_security_refusal`; `or_repair` and the free `must_propagate` deleted.
- lamina-text's `fatal` replaced; a regression test that a `Source` error from a fake source propagates out of font loading.
- A ban in `check maintainability`: no `matches!` on `LimitExceeded` or `DataPending` outside `lamina_cos::error`.

**Out of scope** (and which todo covers it)

- Splitting the reader module ([Q2q](./Q2q-split-cos-reader.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Move each policy into `error.rs` with a doc comment saying when to use it.
- [ ] Replace `or_repair` and `must_propagate` at their call sites; delete them.
- [ ] Replace `fatal`; write the regression test first and watch it fail.
- [ ] Decide whether extraction propagates `LimitExceeded` (cos does for limits) and say so in the PR.
- [ ] Add the ban to `check maintainability`.

## Acceptance criteria

- [ ] The regression test passes: a `Source` error propagates from extraction.
- [ ] Each policy is defined once, and the ban passes.

## Post-merge checks

None.

## Tests

The new regression test; existing tolerance tests and the differential.

## Docs to update

ADR 0003's implementation note if it names policies, plan §9.

## Risks and open questions

Behaviour change for limits in extraction: decide it explicitly.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
