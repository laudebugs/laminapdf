# P0 — Publish the npm packages under `@laminapdf`, with `@lamina/pdf` as `@laminapdf/sdk`

| Field | Value |
| --- | --- |
| ID | `P0` |
| Status | `done` |
| Plan | [architecture: Publishing (later)](../../plans/architecture.md#publishing-later), [§8 Q3](../../plans/architecture.md#8-open-questions-for-the-team) |
| Round / milestone | Publishing (deferred) |
| Depends on | None |
| Blocks | [P0b](./P0b-crate-names-laminapdf.md), [P1](./P1-reserve-names-and-trusted-publishing.md) |
| ADRs | Amends [0015](../../decisions/0015-monorepo-layout-and-tooling.md) (the `packages/` scope) |
| Branch | `refactor/npm-scope-laminapdf` |
| PR title | `refactor: publish the npm packages under @laminapdf` |
| Size | M (≈ 800 changed lines, almost all a mechanical rename) |
| PR | [#125](https://github.com/laudebugs/lamina/pull/125) |

## Goal

The owner decided (2026-10-31) that lamina's npm packages are published under the **`@laminapdf`** scope instead of `@lamina`, matching the `laminapdf.com` domain. Rename the scope everywhere: package manifests, imports, the xtask tasks and `pnpm --filter` calls, CI, generated files, the docs site and the documentation. The owner asked for this work directly, so it had no claim PR. In the same PR the owner also renamed the TypeScript API package from `@lamina/pdf` to **`@laminapdf/sdk`** (directory `packages/sdk`, subpaths `@laminapdf/sdk/explain` and `@laminapdf/sdk/worker`, the docs site's reference at `/reference/sdk/`).

## Context

- [ADR 0015](../../decisions/0015-monorepo-layout-and-tooling.md) §3 fixed `packages/` as `@lamina/<dir>`; the [code organization guide](../../guidelines/code-organization.md#3-naming) repeats it.
- [Architecture §8 Q3](../../plans/architecture.md#8-open-questions-for-the-team) left the package scope open.
- [P1](../done/P1-reserve-names-and-trusted-publishing.md) reserves the names; publishing stays later.

## Scope

**In scope**

- `@laminapdf/sdk` (was `@lamina/pdf`; `packages/pdf` → `packages/sdk`), `@laminapdf/wasm`, `@laminapdf/docs`, the render packages (manifests, imports in `packages/sdk` src, tests, examples and bench, Vite/Vitest config), `pnpm-lock.yaml` (regenerated with `pnpm install`), `packages/wasm/size-budgets.json`.
- `tools/xtask` (`wasm`, `size`, `pdf-runtimes`, `codegen`, the docs-site build, the slug test), then the files generated from `fixtures/codes.json` (`cargo xtask gen codes`).
- `.github/workflows/*`: `pnpm --filter @laminapdf/docs` in `docs.yml`, job names and comments (committed by the owner: the session that made the PR could not push workflow files).
- `apps/docs` (pages, TypeDoc sidebar label), Rust doc comments that name the packages.
- Documentation: CLAUDE.md, README.md, ADRs, plans, ready and ongoing todos (planned packages too: `@laminapdf/viewer`, `@laminapdf/tokens`, `@laminapdf/fonts`, …), research notes, design docs, error pages, the board; anchors that contained the old name.
- The frozen `spikes/pdf-basics/ts` package (`@laminapdf/pdf-basics-spike`): its CI job still passes.
- Records: §8 Q3 (package scope), a roadmap "Decisions made" entry, an amendment note in ADR 0015, the guide, P1's reservation task.

**Out of scope** (and which todo covers it)

- Reserving the scope and publishing → [P1](../done/P1-reserve-names-and-trusted-publishing.md), [P2](../ready/P2-first-registry-prerelease.md).
- Crate names (`lamina-*`), the Rust façade `lamina` and the product name: unchanged (the crates became `laminapdf-*` later, in [P0b](P0b-crate-names-laminapdf.md)).
- History: done todos keep the name they were written under (links into renamed anchors are fixed), and so do their titles on the generated board. The frozen spikes `pdfium-wasm`, `krilla-fit` and `hayro-parity` keep it too: `pdfium-wasm`'s lockfile pins the old tarballs, and it is re-run only with a rename of its own.

## Tasks

- [x] Rename `@lamina/` to `@laminapdf/` outside history; regenerate the lockfile, the codes and the board
- [x] Fix the anchors whose slug changed (`pdf-library.md#b6-wasm-and-laminapdfpdf`)
- [x] Record the decision (§8 Q3, roadmap, ADR 0015, code organization guide, P1)
- [x] Rename `@laminapdf/pdf` to `@laminapdf/sdk` and `packages/pdf` to `packages/sdk`; amend ADR 0016
- [x] The workflow files' `@lamina/` lines (owner, on the PR branch)

## Acceptance criteria

- `git grep -n "@lamina/"` finds only done todos (and their titles on the board) and the three frozen spikes.
- `cargo xtask ci` and the pdf-basics spike's checks pass.
