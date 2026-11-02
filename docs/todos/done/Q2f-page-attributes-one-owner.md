# Q2f — One reader of page attributes and one rotation type (fixes `/Rotate 90.0`)

| Field | Value |
| --- | --- |
| ID | `Q2f` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | None |
| ADRs | None |
| Branch | `fix/q2f-page-attributes-one-owner` |
| PR title | `fix(model,ops,render): one reader of page attributes and one rotation type` |
| Size | M |
| PR | [#186](https://github.com/laudebugs/laminapdf/pull/186) |

## Goal

`/Rotate` and `/MediaBox` are parsed both in laminapdf-cos (`Page.rotate: i64`, integers only) and in laminapdf-model (integral reals too), and laminapdf-ops reads the cos copy, so ops treats `/Rotate 90.0` as unrotated while the model says 90°. laminapdf-render also has `ViewRotation`, a copy of `laminapdf_model::Rotation`, and degree parsing is written again in laminapdf-render-wasm and both adapters. This todo makes laminapdf-model the only reader of page attributes and keeps one rotation type.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `laminapdf-cos/src/reader/mod.rs:63-82,1359-1393`, `laminapdf-model/src/page.rs:17,242-360`, `laminapdf-ops/src/rotate.rs:137`, `laminapdf-render/src/request.rs:76`, `laminapdf-render-wasm/src/lib.rs:216`, `laminapdf-render/src/hayro.rs:207`, `laminapdf-render-pdfium/src/lib.rs:217`.

## Scope

**In scope**

- `cos::Page` keeps the raw inherited values only; laminapdf-model adds a rotation reader that raises no warning, for ops; ops switches to it.
- One `Rotation` with `from_degrees`, in the lowest crate laminapdf-render already depends on; `ViewRotation` removed or aliased; the hand-written degree parsers deleted.
- A changeset for the API change.

**Out of scope** (and which todo covers it)

- Graphics types ([Q2g](../ready/Q2g-one-matrix.md)).

## Tasks

- [x] Claimed by opening the work PR from a fresh worktree off `main`.
- [x] Write a test for `/Rotate 90.0` in ops and watch it fail.
- [x] Add the model reader; switch ops; remove the parsed fields from `cos::Page` and fix callers.
- [x] Move or alias `Rotation`, add `from_degrees`, delete the parsers in render-wasm and both adapters.
- [x] Run all tests and the render reference images; add the changeset.

## Acceptance criteria

- [x] Ops rotates a `/Rotate 90.0` page correctly (the new test).
- [x] One rotation type; no hand-written degree parser left.

## Post-merge checks

None.

## Tests

New tests in laminapdf-ops and laminapdf-render; existing reference images.

## Docs to update

Crate READMEs (laminapdf-model owns page attributes), CLAUDE.md if the render API changes, plan §9.

## Risks and open questions

An API break in laminapdf-cos and laminapdf-render while alpha: the changeset says so.

**Decided (#186):** `Rotation` lives in laminapdf-syntax, the lowest crate both the model and laminapdf-render depend on, with `from_object` as the one reader of a `/Rotate` value. Inheritance and warnings stay in laminapdf-model. The `RotatePage` op's fast path, which follows `/Parent` instead of walking the tree, moved into the model as `Document::page_by_parents`, so ops reads no page attribute itself. The render tests changed only through the rename; the rotation tests live with the type in laminapdf-syntax. `from_canonical_degrees` now checks the values callers pass in (op JSON and both WASM bindings), so the builder binding refuses `-90` and `450` as its type already said.

## Definition of done

- [x] Claimed: the work PR's first commit moved this file `ready/` → `ongoing/` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
