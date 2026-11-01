# Q2f — One owner for page attributes and rotation (fixes `/Rotate 90.0`)

| Field | Value |
| --- | --- |
| ID | `Q2f` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | None |
| ADRs | None |
| Branch | `fix/q2f-page-attributes` |
| PR title | `fix(model,ops,render): one reader of page attributes and one rotation type` |
| Size | M |
| PR | — |

## Goal

`/Rotate` and `/MediaBox` are parsed in both lamina-cos (`Page.rotate: i64`, integers only) and lamina-model (accepts integral reals); lamina-ops reads the cos copy, so `/Rotate 90.0` is treated as unrotated by ops while the model says 90°. lamina-render also has `ViewRotation`, a copy of `lamina_model::Rotation`, with degree parsing written again in lamina-render-wasm and both adapters. Make lamina-model the only interpreter of page attributes and keep one rotation type.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `lamina-cos/src/reader/mod.rs:63-82,1359-1393`, `lamina-model/src/page.rs:17,242-360`, `lamina-ops/src/rotate.rs:137`, `lamina-render/src/request.rs:76`, `lamina-render-wasm/src/lib.rs:216`, `lamina-render/src/hayro.rs:207`, `lamina-render-pdfium/src/lib.rs:217`.

## Scope

**In scope**

- cos keeps raw inherited values only; model adds a no-warning rotation reader for ops; ops switches to it; test for `/Rotate 90.0`.
- One `Rotation` with `from_degrees` in the lowest crate that needs it (lamina-syntax or lamina-model, whichever lamina-render already depends on); `ViewRotation` becomes an alias or is removed; the hand parsers go.

**Out of scope** (and which todo covers it)

- Matrix/graphics types ([Q2g](./Q2g-one-matrix.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Change, migrate callers, test.

## Acceptance criteria

- [ ] `/Rotate 90.0` rotates correctly in ops (new test).
- [ ] One rotation type; no hand-written degree parser left.

## Post-merge checks

None.

## Tests

New tests in lamina-ops and lamina-render; existing goldens.

## Docs to update

Crate READMEs (who owns page attributes), CLAUDE.md if the render API changes, plan §9.

## Risks and open questions

API break in lamina-cos and lamina-render while alpha: add a changeset.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
