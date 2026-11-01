# Q2s — Façade names that don't collide

| Field | Value |
| --- | --- |
| ID | `Q2s` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2a](./Q2a-maintainability-checks-rust.md) |
| Blocks | None |
| ADRs | None |
| Branch | `refactor/q2s-facade-names` |
| PR title | `refactor(lamina): façade names without collisions; one public path per item` |
| Size | M |
| PR | — |

## Goal

Readers of the docs meet `lamina::TextRun` (drawn text) next to `lamina::extract::TextRun` (extracted text), `Font` next to `FontHandle`, and `Tag` meaning both an OpenType tag and a structure element in the crates below; `ops()` hands out a second `Document` type; and lamina-text exposes `extract` both as a module and at its root. This todo renames, hides and leaves one public path per item.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `lamina/src/{lib.rs:125-163, extract.rs:17, document.rs:390-398}`, `lamina-text/src/lib.rs:93,98`, lamina-build's `TextRun`, `Font` and `Tag`; the façade collision check from [Q2a](./Q2a-maintainability-checks-rust.md).

## Scope

**In scope**

- Renames such as `ExtractedRun` and `OpenTypeTag`, and one of `Font` or `FontHandle` renamed so the two read differently, with `@laminapdf/sdk`'s names kept in step.
- lamina-text modules private, re-exported at the root only; `MAX_USECMAP_DEPTH` private.
- `ops()` and `into_ops()` `#[doc(hidden)]` or behind a feature.
- Examples, guides and CLAUDE.md updated; a changeset.

**Out of scope** (and which todo covers it)

- `TextRun`'s `#[must_use]` ([Q1d](./Q1d-example-api-cleanups.md)); the error-wrapping decision ([Q2y](./Q2y-facade-error-wrapping.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] List every collision the façade check reports; choose names with the TypeScript SDK in view.
- [ ] Rename at the source crate where the name is wrong there too, else at the façade.
- [ ] Make lamina-text's modules private; hide `ops()`.
- [ ] Update the examples, guides, rustdoc and CLAUDE.md; add the changeset; run everything.

## Acceptance criteria

- [ ] The façade collision check passes with no allow-list entry for these names.

## Post-merge checks

None.

## Tests

Existing suites, doc tests and reference files.

## Docs to update

Guides, CLAUDE.md's façade paragraph, a changeset, plan §9.

## Risks and open questions

Renames ripple into the docs: land this before Q1i and Q1m if possible, or update those pages here.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
