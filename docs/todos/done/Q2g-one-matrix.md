# Q2g — One `Matrix`, `FillRule`, `Dash` and `Color`

| Field | Value |
| --- | --- |
| ID | `Q2g` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | [Q2a](./Q2a-maintainability-checks-rust.md) |
| Blocks | None |
| ADRs | None |
| Branch | `refactor/q2g-one-matrix` |
| PR title | `refactor(model): one Matrix, FillRule and Dash for building and reading` |
| Size | M |
| PR | [#226](https://github.com/laudebugs/laminapdf/pull/226) |

## Goal

`Matrix` exists in laminapdf-build (`new([f64;6])`, `after`) and in laminapdf-content (six-argument `new`, `multiply`), with opposite composition orders, and raw `[f64; 6]` matrices appear in laminapdf-model, laminapdf-render and laminapdf-wasm. `FillRule`, `Dash` and `Color` are also duplicated. This todo puts one of each in laminapdf-model, which both crates already depend on.

## Context

- [Quality plan §2.3](../../plans/quality.md#23-code-quality-and-reuse); `laminapdf-build/src/surface/{path.rs:12,174, style.rs:34, color.rs:22, mod.rs:748}`, `laminapdf-content/src/{state.rs:10,129,166, visitor.rs:136}`, `laminapdf-model/src/geometry.rs:378`, `laminapdf-render/src/request.rs:342,510`, `laminapdf-wasm/src/build_dto.rs:281`.

## Scope

**In scope**

- One `Matrix` with composition methods whose names say the order (for example `then` and `pre_concat`); one `FillRule`, `Dash` and `Color`; re-exports where the façade shows them; the raw arrays replaced.
- The duplicate-name allow-list from [Q2a](../done/Q2a-maintainability-checks-rust.md) loses these entries.

**Out of scope** (and which todo covers it)

- Façade renames ([Q2s](../done/Q2s-facade-names.md)).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Write unit tests that pin each composition order.
- [x] Add the types to laminapdf-model; migrate laminapdf-build, then laminapdf-content, then the raw arrays in render and wasm.
- [x] Remove the allow-list entries; run every reference suite.

## Acceptance criteria

- [x] One definition of each type; the duplicate-name check passes without them.
- [x] Every reference PDF and image is unchanged.

## Outcome

`Matrix`, `FillRule` and `Dash` have one definition each, in `laminapdf-model`, and the other crates re-export it. `Color` stays two types, since the builder's colour and the content reader's current colour (a space, its components and a pattern) are different things. The reader's is renamed `CurrentColor`, so the name has one home.

## Post-merge checks

None.

## Tests

Composition-order unit tests; existing reference suites.

## Docs to update

Crate docs, plan §9.

## Risks and open questions

Mixing up the order changes output: the reference suites catch it.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
