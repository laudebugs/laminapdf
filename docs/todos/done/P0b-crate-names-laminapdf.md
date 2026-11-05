# P0b — Rename the crates `laminapdf-*` and the façade `laminapdf`

| Field | Value |
| --- | --- |
| ID | `P0b` |
| Status | `done` |
| Plan | [architecture: Publishing (later)](../../plans/architecture.md#publishing-later), [§8 Q3](../../plans/architecture.md#8-open-questions-for-the-team) |
| Round / milestone | Publishing (deferred) |
| Depends on | [P0](./P0-npm-scope-laminapdf.md) |
| ADRs | Amends [0015](../../decisions/0015-monorepo-layout-and-tooling.md) (crate names) |
| Branch | `refactor/p0b-rename-crates-laminapdf` |
| PR title | `refactor: rename the crates laminapdf-* and the façade laminapdf` |
| Size | XL in lines, but a mechanical rename (≈ 750 files, almost every change a name or a path) |
| PR | [#178](https://github.com/laudebugs/laminapdf/pull/178) |

## Goal

Give every Rust crate a name we can publish. `lamina` on crates.io belongs to an unrelated compiler project, so the façade becomes `laminapdf` and each core and edge crate `laminapdf-<name>`, matching the `@laminapdf` npm scope and the `laminapdf.com` domain. Publishing ([P1](../done/P1-reserve-names-and-trusted-publishing.md)) then reserves names that are free.

## Context

- [P0](../done/P0-npm-scope-laminapdf.md) moved the npm packages to `@laminapdf` and kept the crate names; this todo finishes the rename on the Rust side.
- [ADR 0015](../../decisions/0015-monorepo-layout-and-tooling.md): the directory name is the crate name.
- crates.io on 2026-11-01: `lamina` is taken (0.1.0, an IR compiler backend); `laminapdf` and every `laminapdf-*` name used here are free.

## Scope

**In scope**

- The fourteen workspace crates and their folders: `lamina` → `laminapdf`, `lamina-<name>` → `laminapdf-<name>`, and their Rust paths (`lamina_cos::…` → `laminapdf_cos::…`).
- The fuzz workspace's crate (`lamina-fuzz` → `laminapdf-fuzz`) and every manifest, lockfile, workflow, xtask argument, doc link and doc mention of those crates.
- Crates the plans and todos name but that don't exist yet (`laminapdf-extract`, `laminapdf-cli`, `laminapdf-sign`, `laminapdf-desktop`, `laminapdf-test-fixtures`, …).
- `cargo xtask check maintainability`'s baseline comparison follows git renames, so a module moved since the merge base keeps its entry.

**Out of scope**

- The product name (lamina), the repository, `LAMINA_*` environment variables and `[package.metadata.lamina]`.
- The `@laminapdf/wasm` module files (`lamina_base`, `lamina_text`, `lamina_extract`, `lamina_render_hayro`): npm file names, not crates.
- Text written into PDFs and fixtures (producers, creators, document IDs such as `lamina-build-B5a`), so every golden stays byte-identical, and checksummed inputs such as `tools/std14-gen/data/latin-encodings.txt`, whose comment still names `lamina_syntax`.
- The frozen spikes' own crate names (`lamina-ffi`, `lamina-pdf-basics`, …); their dependencies on the workspace crates are updated.
- Generated changelogs (`CHANGELOG.md`).
- Done todos and their board titles: they keep the names they were written under, as in [P0](P0-npm-scope-laminapdf.md), and only their links follow the rename.

## Tasks

- [x] Rename the crate folders and every reference (a token-exact script; the façade by its Rust and Cargo forms only)
- [x] Keep producer, creator and ID strings and the wasm module names
- [x] Regenerate what is generated (`cargo xtask gen codes`) and format
- [x] Follow renames in the maintainability baseline comparison, with a unit test
- [x] Let the ratcheting checks (docs-prose, docs-snippets, clippy-examples, biome-examples) skip a file whose only change is the rename, so the examples' existing problems stay warnings until Q1d and Q1f (the owner's choice)
- [x] Record the decision in ADR 0015, architecture §8 Q3, P0 and P1

## Acceptance criteria

- [x] `cargo metadata` lists only `laminapdf` and `laminapdf-*` crates under `crates/`, and `git grep` finds no `lamina-<crate>` or `lamina_<crate>` outside the out-of-scope list
- [x] Every PDF golden and every checksummed input is unchanged byte for byte; a text fixture changes only where it names a renamed path (the reading-order corpus's `method`, which names its generator)
- [x] CI green

## Post-merge checks

- [x] Once no open branch predates the rename, remove `RENAMED` and `renamed_only` from `tools/xtask/src/reports.rs`, so the ratchet treats every change alike again. Done after P0c, on 2026-11-01, when no branch was open.

## Tests

The existing suites, unchanged; they prove the rename kept behaviour and output. A unit test for the rename-following baseline comparison.

## Docs to update

- [x] ADR 0015 (amendment), [architecture](../../plans/architecture.md) (Publishing, §8 Q3), `CLAUDE.md`, READMEs and guidelines (by the rename)

## Risks and open questions

- Every open Rust branch conflicts with the rename; the other agents hold new Rust work until it lands.
- The examples checks (`docs-snippets`, `clippy-examples`, `biome-examples`) are strict on changed files, and the rename changes every example. Their existing problems belong to [Q1d](../done/Q1d-example-api-cleanups.md) and [Q1f](../done/Q1f-examples-readable.md), so the ratchet skips rename-only files until the post-merge check removes that exemption.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, added this file in `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
