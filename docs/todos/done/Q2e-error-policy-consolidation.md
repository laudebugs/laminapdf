# Q2e — One error-propagation policy per kind (fixes swallowed I/O errors)

| Field | Value |
| --- | --- |
| ID | `Q2e` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2a](./Q2a-maintainability-checks-rust.md) |
| Blocks | [Q2q](./Q2q-split-cos-reader.md) |
| ADRs | Follows 0003 |
| Branch | `fix/q2e-error-policy-consolidation` |
| PR title | `fix(cos,text): one set of error-propagation policies; extraction propagates source errors` |
| Size | S |
| PR | [#180](https://github.com/laudebugs/laminapdf/pull/180) |

## Goal

Text extraction decides which errors propagate with its own `fatal()`, which leaves out `Error::Source`, so an I/O failure from a `FileSource` reads as "no font" (against ADR 0003). laminapdf-cos has its own scattered set: `tolerant` and its duplicate `or_repair`, a `must_propagate` wrapper, `absent_if_malformed`, and inline matches for ending the page walk and security refusals. This todo puts every policy in `laminapdf_cos::error` and makes extraction use one.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); ADR 0003; `laminapdf-cos/src/error.rs:246,267`, `laminapdf-cos/src/reader/mod.rs:274,280,290,298,408`, `laminapdf-text/src/extract/font.rs:137`, `laminapdf-ops/src/error.rs:150`.

## Scope

**In scope**

- Public, documented policies in `error.rs`: `tolerant`, `absent_if_malformed`, `ends_page_walk`, `is_security_refusal`; `or_repair` and the free `must_propagate` deleted.
- laminapdf-text's `fatal` replaced; a regression test that a `Source` error from a fake source propagates out of font loading.
- A ban in `check maintainability`: no `matches!` on `LimitExceeded` or `DataPending` outside `laminapdf_cos::error`.

**Out of scope** (and which todo covers it)

- Splitting the reader module ([Q2q](./Q2q-split-cos-reader.md)).

## Tasks

- [x] Claim by opening the work PR from a fresh worktree off the updated `main` (see Definition of done).
- [x] Move each policy into `error.rs` with a doc comment saying when to use it.
- [x] Replace `or_repair` and `must_propagate` at their call sites; delete them.
- [x] Replace `fatal`; write the regression test first and watch it fail.
- [x] Decide whether extraction propagates `LimitExceeded` (cos does for limits) and say so in the PR.
- [x] Add the ban to `check maintainability`.

## Acceptance criteria

- [x] The regression test passes: a `Source` error propagates from extraction.
- [x] Each policy is defined once, and the ban passes.

## Post-merge checks

None.

## Tests

The new regression test; existing tolerance tests and the differential.

## Docs to update

ADR 0003's implementation note if it names policies, plan §9.

## Risks and open questions

Behaviour change for limits in extraction: decide it explicitly.

**Decided (#180):** extraction keeps failing on hard limits. It reads fonts with `absent_if_malformed`, the policy for reads outside ADR 0003's tolerant paths. So it now also fails on `Error::Source`, the bug this todo fixes, and on reference cycles, which ADR 0003 rejects. A malformed entry still reads as absent, with `font-invalid`. The page's resource lookup in `laminapdf-content` stays tolerant, so a cycle there reads as a missing resource, with `content-resource-missing`. A cycle inside the font's own entries fails the page.

## Definition of done

- [x] Claimed: the work PR's first commit moved this file `ready/` → `ongoing/` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
