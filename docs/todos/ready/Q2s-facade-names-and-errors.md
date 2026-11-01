# Q2s — Façade names that don't collide, one public path per item, and the error-wrapping decision

| Field | Value |
| --- | --- |
| ID | `Q2s` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | None |
| ADRs | Amends 0028 |
| Branch | `refactor/q2s-facade-names` |
| PR title | `refactor(lamina): façade names without collisions; one public path per item` |
| Size | M |
| PR | — |

## Goal

Readers of the docs meet `lamina::TextRun` (drawn text) next to `lamina::extract::TextRun` (extracted), `Font` next to `FontHandle`, `Tag` meaning both an OpenType tag and a structure element in the crates below, and `ops()` handing out a second `Document` type. lamina-text exposes `extract` both as a module and at its root. The façade also re-exports every wrapped crate's error type, against rust.md. Rename, hide, and decide the error question in ADR 0028.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `lamina/src/{lib.rs:125-163, extract.rs:17, error.rs:36-52, document.rs:390-398}`, `lamina-text/src/lib.rs:93,98`, `lamina-build`'s `TextRun`/`Font`/`Tag`; rust.md ("SHOULD NOT expose another crate's error type"); ADR 0028.

## Scope

**In scope**

- Renames at the source or the façade (e.g. `ExtractedRun`, `OpenTypeTag`, and one of `Font`/`FontHandle` renamed so the two read differently), with the TS SDK's names kept in step.
- lamina-text modules private, root re-exports only; `MAX_USECMAP_DEPTH` private.
- `ops()`/`into_ops()` `#[doc(hidden)]` or behind a feature.
- ADR 0028 amendment: keep wrapping inner errors (and amend rust.md) or make variants opaque behind `code()`; implement the choice.
- Examples, guides and CLAUDE.md updated; changeset.

**Out of scope** (and which todo covers it)

- `TextRun`'s `#[must_use]` ([Q1d](./Q1d-example-api-cleanups.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Decide the names with the TS SDK in view; rename; update docs; run everything.

## Acceptance criteria

- [ ] The façade's duplicate-name check (from [Q2a](./Q2a-maintainability-checks-rust.md)) passes with no allow-list entry for these.
- [ ] ADR 0028 records the error decision.

## Post-merge checks

None.

## Tests

Existing suites and goldens; doc tests.

## Docs to update

ADR 0028, rust.md (if amended), CLAUDE.md façade paragraph, guides, plan §9.

## Risks and open questions

Renames ripple into the docs: do them before Q1i/Q1j if possible, or update those pages here.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
