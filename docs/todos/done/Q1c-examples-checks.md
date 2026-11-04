# Q1c — The examples checks: `docs-snippets`, a Biome override and `clippy-examples`

| Field | Value |
| --- | --- |
| ID | `Q1c` |
| Status | `done` |
| Plan | [Quality plan §6](../../plans/quality.md#6-todos) |
| Round / milestone | Round Q (docs and code quality) |
| Depends on | None |
| Blocks | [Q1e](./Q1e-quick-start-apps.md), [Q1f](./Q1f-examples-readable.md), [Q1g](../ongoing/Q1g-worker-examples.md) |
| ADRs | None |
| Branch | `feat/q1c-examples-checks` |
| PR title | `feat(xtask): docs-snippets, the examples Biome override and clippy-examples` |
| Size | M |
| PR | [#155](https://github.com/laudebugs/lamina/pull/155) |

## Goal

The owner asked for a check that keeps examples maintainable while we write them. This todo adds the three checks in the [examples guideline](../../guidelines/examples.md)'s "How it's checked" table. Each is strict on every example a PR adds or changes and warns on the others until the example rewrites land.

## Context

- The [examples guideline](../../guidelines/examples.md): the rules and the table of checks.
- [Quality plan §2.2](../../plans/quality.md#22-code-examples): what the examples look like today.
- `apps/docs/src/components/Snippet.astro` (the glob of files a page may show), `packages/sdk/examples/`, `crates/lamina/examples/`, `packages/sdk/biome.json` (already restricts Node modules in examples).
- Biome 2.6.3 (the `pnpm-workspace.yaml` catalog): `noRestrictedImports`, `noNonNullAssertion`, `noExplicitAny`, `noExcessiveCognitiveComplexity`, `noExcessiveLinesPerFunction`; confirm each with `biome explain`.

## Scope

**In scope**

- `cargo xtask check docs-snippets`, part of `check docs`:
  - every file in Snippet's glob opens with a header comment free of ADR and todo IDs, `fixtures/`, "golden" and "twin";
  - no `test/` path in the glob;
  - every example is used by a `<Snippet file>` or listed in `examples/UNUSED.md`;
  - TS and Rust pairs share a name (a pair table);
  - Rust examples never `use lamina_<crate>` or `include_bytes!("../../..`;
  - no test scaffolding (`lamina-ex-`, fixed `D:20…` dates);
  - files ≤ 100 lines.
- A Biome override for `packages/sdk/examples/**` in `check ts`: `noRestrictedImports` (relative paths out of the example's folder, `**/src/**`, `**/test/**`, `**/fixtures/**`), `noNonNullAssertion` and `noExplicitAny` as errors, cognitive complexity ≤ 10, function length ≤ 40.
- `cargo xtask clippy-examples` in `check rust`: clippy on `crates/lamina/examples/` with `CLIPPY_CONF_DIR=tools/xtask/clippy-examples` (`too-many-lines-threshold = 40`) and `-D clippy::let_underscore_must_use`.
- Strict on examples changed since the merge base with `main`; warnings elsewhere; an `--all-strict` flag for Q1l.

**Out of scope** (and which todo covers it)

- Rewriting the examples ([Q1e](../done/Q1e-quick-start-apps.md), [Q1f](../done/Q1f-examples-readable.md), [Q1g](../ongoing/Q1g-worker-examples.md)).

## Tasks

- [x] Claim via a claim PR (see Definition of done); then a fresh worktree off the updated `main`.
- [x] Write `docs-snippets` with unit tests; read the glob from `Snippet.astro` and the uses from the `.mdx` files.
- [x] Add the Biome override; run it and list today's violations. Deviation: the override's rules are warnings, not errors, so untouched examples pass `biome ci`; `biome-examples` makes them errors on changed examples.
- [x] Add `clippy-examples` and its configuration directory; run it and list today's violations. Deviation: `let_underscore_must_use` is warned and ratcheted like the rest, not denied with `-D`, so untouched examples don't fail.
- [x] Add the changed-files logic (shared with Q1b if it has landed) and `--all-strict` to all three. Q1b's ratchet moved into `tools/xtask/src/reports.rs`, which docs-prose and the three examples checks use; Biome's strict pass is `cargo xtask biome-examples`.
- [x] Wire each check into its lane; put today's violations in the PR description as Q1e–Q1g's work list.

## Acceptance criteria

- [x] Each check reports file and line, fails on a changed example, and warns elsewhere.
- [x] `cargo xtask check` passes on `main`.

## Post-merge checks

None.

## Tests

Unit tests in `tools/xtask` with small example trees under `tools/xtask/tests/` (one violation per rule). Deviation: as in Q1b, the cases are inline in `docs_snippets.rs`, `examples_lints.rs` and `reports.rs` (one passing and one failing case per rule), since xtask has no `tests/` folder.

## Docs to update

The examples guideline if anything differs, the commands list in CLAUDE.md, plan §9.

## Risks and open questions

Biome rule names can differ between versions: confirm each with `biome explain` and correct the guideline if a rule is missing.

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process)) Findings and fixes in #155's commits a11667f..412a250: f284ff5 (writing), b96da47 (code), 412a250 (second round).
- [x] CI green (every check run on #155's last tested commit)
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
