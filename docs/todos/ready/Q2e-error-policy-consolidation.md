# Q2e — One error-propagation policy per meaning (fixes I/O errors swallowed by text extraction)

| Field | Value |
| --- | --- |
| ID | `Q2e` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | [Q2q](./Q2q-split-large-rust-modules-core.md) |
| ADRs | Follows 0003 |
| Branch | `fix/q2e-error-policies` |
| PR title | `fix(cos,text): one set of error-propagation policies; extraction propagates source errors` |
| Size | S |
| PR | — |

## Goal

Text extraction decides which errors propagate with its own `fatal()` (`lamina-text/src/extract/font.rs:137`), which leaves out `Error::Source`: an I/O failure from a `FileSource` or `MmapSource` reads as "no font", against ADR 0003. lamina-cos itself has `tolerant`, `or_repair` (identical), a `must_propagate` wrapper and `absent_if_malformed`, spread through the reader. Put every policy in `lamina_cos::error` as documented public functions, delete the duplicates, and make extraction use one of them.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); ADR 0003 (tolerance); `lamina-cos/src/error.rs:246,267`, `reader/mod.rs:274,280,290,298,408`, `lamina-text/src/extract/font.rs:137`, `lamina-ops/src/error.rs:150`.

## Scope

**In scope**

- Policies in `error.rs`: `tolerant`, `absent_if_malformed` (public), `ends_page_walk`, `is_security_refusal`; delete `or_repair` and the `must_propagate` free function.
- lamina-text: `fatal` replaced; a test that a `Source` error from a fake source propagates out of font loading.
- The grep ban ("no crate-local `matches!(… LimitExceeded | … DataPending …)`") in `check maintainability`.

**Out of scope** (and which todo covers it)

- Splitting the reader module ([Q2q](./Q2q-split-large-rust-modules-core.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Move, delete, replace, test.

## Acceptance criteria

- [ ] A `Source` error propagates from extraction (new test).
- [ ] One definition per policy; the ban passes.

## Post-merge checks

None.

## Tests

New regression test in lamina-text; existing tolerance tests.

## Docs to update

ADR 0003 implementation note if the policies' names are documented there; plan §9.

## Risks and open questions

Behaviour change for `LimitExceeded` in extraction: decide whether extraction should propagate limits (likely yes, as cos does for limits) and say so in the PR.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
