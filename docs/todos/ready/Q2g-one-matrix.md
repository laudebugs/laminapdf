# Q2g — One `Matrix`, `FillRule`, `Dash` and `Color` shared by building and reading

| Field | Value |
| --- | --- |
| ID | `Q2g` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | None |
| ADRs | None |
| Branch | `refactor/q2g-one-matrix` |
| PR title | `refactor(model): one Matrix, FillRule, Dash and Color for build and content` |
| Size | M |
| PR | — |

## Goal

`Matrix` exists in lamina-build (`new([f64;6])`, `after`) and lamina-content (6-argument `new`, `multiply`) with opposite composition conventions, and raw `[f64; 6]` matrices appear in lamina-model, lamina-render and lamina-wasm. `FillRule`, `Dash` and `Color` are also duplicated. Put one of each in lamina-model (both crates already depend on it) with clearly named composition helpers; the others re-export it.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `lamina-build/src/surface/{path.rs:12,174, style.rs:34, color.rs:22, mod.rs:748}`, `lamina-content/src/{state.rs:10,129,166, visitor.rs:136}`, `lamina-model/src/geometry.rs:378`, `lamina-render/src/request.rs:342,510`, `lamina-wasm/src/build_dto.rs:281`.

## Scope

**In scope**

- One type each in lamina-model; names that say the composition order (`then`/`pre_concat` or similar); migrate both crates and the raw arrays; keep public names stable via re-exports where the façade shows them.

**Out of scope** (and which todo covers it)

- Façade renames ([Q2s](./Q2s-facade-names-and-errors.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Introduce, migrate crate by crate, run all goldens.

## Acceptance criteria

- [ ] One definition of each type (the `check maintainability` duplicate-name check drops them from its allow-list).
- [ ] All goldens unchanged.

## Post-merge checks

None.

## Tests

Unit tests for composition order (both conventions named); existing goldens.

## Docs to update

Crate docs, plan §9.

## Risks and open questions

Mixing up composition order changes output: the goldens catch it.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
