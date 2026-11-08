# A2 — Monorepo skeleton, `cargo xtask`, licence checks and CI fast lane

| Field | Value |
| --- | --- |
| ID | `A2` |
| Status | `done` |
| Plan | [architecture §5 Round A](../../plans/architecture.md#5-rounds), [§7 Round A to-dos](../../plans/architecture.md#round-a--foundations) |
| Round / milestone | Round A (foundations) — M0 |
| Depends on | [A1](./A1-foundation-adrs.md) |
| ADRs | Implements [0001](../../decisions/0001-core-language-and-licence.md), [0015](../../decisions/0015-monorepo-layout-and-tooling.md), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) |
| Branch | `build/monorepo-skeleton` |
| PR title | `build: monorepo skeleton, cargo xtask, licence checks and CI fast lane` |
| Size | L (≈ 2,650 lines incl. lockfiles and licence texts) |
| PR | [#7](https://github.com/laudebugs/lamina/pull/7) (squash-merged as `ebe87b3`) |

## Goal

Create the empty but fully checked monorepo that every later PR lands in: one Cargo workspace and one pnpm workspace, one task entry point (`cargo xtask`) that CI runs verbatim, licence and supply-chain gates, and the CI fast lane from [ADR 0018](../../decisions/0018-test-strategy.md#2-ci-lanes) (then still planned).

## Context

- [ADR 0015](../../decisions/0015-monorepo-layout-and-tooling.md): layout and tooling.
- [ADR 0019](../../decisions/0019-supply-chain-and-licence-hygiene.md): `deny.toml`, npm licence check, Dependabot.
- [Code organization §1](../../guidelines/code-organization.md#1-top-level-layout) and [Rust guideline](../../guidelines/rust.md#lints): workspace lints, layers.

## Scope

**In scope**

- Root `Cargo.toml` (virtual manifest, edition 2024, `[workspace.lints]`, `wasm-release` profile), `rust-toolchain.toml` (pinned toolchain + wasm32/iOS/Android targets), `clippy.toml` (disallowed `HashMap`/`HashSet`), `rustfmt.toml`, `.cargo/config.toml` (`cargo xtask` alias with `--locked`).
- pnpm root (`package.json`, `pnpm-workspace.yaml` for `packages/*` and `apps/*`, `tsconfig.base.json`, `biome.json`, `.npmrc`, `.nvmrc`).
- `tools/xtask`: `check [rust|ts]`, `fmt [--check]`, `lint`, `test`, `portability` (core crates: no fs/net/process/thread/clocks, no edge deps transitively, crate-level `forbid(unsafe_code)`, cross-target `cargo check`), `wasm`, `deny`, `npm-licenses`, `ci`, `doctor [--install]` with tool pins in `doctor.rs`.
- `deny.toml` (allow-list without AGPL/GPL, mupdf crates banned, wildcard rules); npm licence check reading the same allow-list.
- `.github/workflows/ci.yml` fast lane (Rust, MSRV, TypeScript, pdf-basics spike jobs) and `dependabot.yml`; `LICENSE-MIT`, `LICENSE-APACHE`.
- A placeholder `lamina-syntax` crate so the checks have something to run on.

**Out of scope**

- Real crates: [A3](./A3-port-spike-core.md). Full and nightly lanes: [X4](./X4-full-lane-gate.md), [X3a](../done/X3a-nightly-lane.md). WASM size budget: [B6a](../done/B6a-lamina-wasm-and-adr-0016.md).

## Tasks

- [x] Workspace manifests, toolchain, lint and format config
- [x] `tools/xtask` with every subcommand above and `doctor` pins
- [x] `deny.toml` and the npm licence check
- [x] CI fast lane and Dependabot
- [x] Review fixes (`--locked` alias, nextest fallback, transitive edge-dep rule, crate-level `forbid`, TOML parsing)

## Acceptance criteria

- [x] `cargo xtask ci` passes locally and in CI on an otherwise empty workspace
- [x] A core crate depending on an edge crate, or missing `forbid(unsafe_code)`, fails `cargo xtask portability`

## Tests

xtask's own unit tests; the CI run itself.

## Docs to update

- [x] `CLAUDE.md` (layout, commands), `README.md`, architecture §7 Round A ticks

## Risks and open questions

None open. (Self-hosted runner selection came later in PR #11, outside the todo system.)

## Definition of done

- [x] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (written retroactively: the todo board did not exist yet)
- [x] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [x] Squash-merged; dependants' "Depends on" now satisfied
