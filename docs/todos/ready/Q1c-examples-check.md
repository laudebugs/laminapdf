# Q1c — Examples guideline and the `docs-snippets` check

| Field | Value |
| --- | --- |
| ID | `Q1c` |
| Status | `ready` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | [Q1e](./Q1e-quick-start-apps.md), [Q1f](./Q1f-examples-readable.md), [Q1g](./Q1g-worker-examples.md) |
| ADRs | None |
| Branch | `feat/q1c-examples-check` |
| PR title | `feat(xtask): docs-snippets check and lint config for the examples` |
| Size | M |
| PR | — |

## Goal

Make "examples look like maintainable code" checkable while we write them. Implement the checks of [`docs/guidelines/examples.md`](../../guidelines/examples.md): an xtask step over every file the docs site may show, a stricter Biome override for `packages/sdk/examples/**`, and an examples-only clippy configuration, all at warning level until [Q1l](./Q1l-docs-checks-strict.md).

## Context

- [Examples guideline](../../guidelines/examples.md) and [Quality plan §2.2, §4.2](../../plans/quality.md#22-code-examples).
- `apps/docs/src/components/Snippet.astro` (the glob of files a page may show), `packages/sdk/examples/`, `crates/lamina/examples/`, `packages/sdk/biome.json` (already restricts Node modules in examples).
- Biome 2.6.3 (`pnpm-workspace.yaml` catalog): `noRestrictedImports`, `noNonNullAssertion`, `noExplicitAny`, `noExcessiveCognitiveComplexity`, `noExcessiveLinesPerFunction` (confirm with `biome explain`).

## Scope

**In scope**

- `cargo xtask check docs-snippets` (part of `check docs`): every file in Snippet's glob opens with a header comment free of ADR/todo IDs, `fixtures/`, `golden`, `twin`; no `test/` path in the glob; every example is used by some `<Snippet file>` or listed in `examples/UNUSED.md` with a reason; TS/Rust pairs share a name (pair table); Rust examples don't `use lamina_<crate>` or `include_bytes!("../../..`; no fixed-determinism scaffolding (`lamina-ex-`, `D:20`); file length ≤ 100 lines.
- Biome override for `packages/sdk/examples/**`: `noRestrictedImports` (`../*` outside the example folder, `**/src/**`, `**/test/**`, `**/fixtures/**`), `noNonNullAssertion` and `noExplicitAny` as errors, cognitive complexity ≤ 10, lines per function ≤ 40.
- `cargo xtask clippy-examples`: `cargo clippy -p lamina --examples` with `CLIPPY_CONF_DIR=tools/xtask/clippy-examples` (`too-many-lines-threshold = 40`) and `-D clippy::too_many_lines -D clippy::let_underscore_must_use`; part of `check rust`.

**Out of scope** (and which todo covers it)

- Rewriting the examples ([Q1e](./Q1e-quick-start-apps.md), [Q1f](./Q1f-examples-readable.md), [Q1g](./Q1g-worker-examples.md)).
- Prose checks ([Q1b](./Q1b-docs-prose-check.md)).

## Tasks

- [ ] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [ ] Write the xtask step with unit tests; read the glob from `Snippet.astro` and the uses from the `.mdx` files.
- [ ] Add the Biome override and the clippy config; run both and list today's violations in the PR description.
- [ ] Wire every part at warning level (Biome `warn`, clippy step reports but doesn't fail, xtask prints).

## Acceptance criteria

- [ ] Each rule reports file and line; `--strict` fails on any.
- [ ] Today's violations are listed in the PR (they are Q1e–Q1g's work list).
- [ ] `cargo xtask check` passes on `main`.

## Post-merge checks

None.

## Tests

Unit tests in `tools/xtask` with small example trees under `tools/xtask/tests/` (one violation per rule).

## Docs to update

The examples guideline ("How it's checked"), CLAUDE.md commands, plan §9.

## Risks and open questions

Biome rule names may differ in 2.6.3; check with `biome explain` and adjust the guideline if a rule doesn't exist.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
