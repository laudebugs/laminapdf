# A1 — Foundation ADRs 0001, 0002, 0015, 0019

| Field | Value |
| --- | --- |
| ID | `A1` |
| Status | `done` |
| Plan | [architecture §5 Round A](../../plans/architecture.md#5-rounds-one-pull-request-each), [§7 Round A to-dos](../../plans/architecture.md#round-a--foundations) |
| Round / milestone | Round A (foundations) — M0 |
| Depends on | None |
| Blocks | [A2](./A2-monorepo-skeleton.md) |
| ADRs | Writes [0001](../../decisions/0001-core-language-and-licence.md), [0002](../../decisions/0002-binding-strategy.md), [0015](../../decisions/0015-monorepo-layout-and-tooling.md), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) |
| Branch | `docs/round-a-adrs` |
| PR title | `docs(decisions): ADRs 0001, 0002, 0015, 0019 and record signing + hosted-service decisions` |
| Size | M (≈ 320 lines of docs) |
| PR | [#6](https://github.com/laudebugs/lamina/pull/6) (squash-merged as `130924a`) |

## Goal

Record the four decisions every later round builds on (a Rust core under `MIT OR Apache-2.0` that is bytes in / bytes out; wasm-bindgen + tsify bindings; the Cargo + pnpm + `cargo xtask` monorepo; supply-chain and licence hygiene) and the owner's 2026-10-28 product decisions (the owner personally code-signs the desktop app; lamina will be a hosted service with opt-in login), so that Round A's code PRs implement accepted ADRs instead of assumptions.

## Context

- [Architecture research report](../../research/architecture/app-and-package-architecture.md) and its [notes](../../research/architecture/notes/): the evidence each ADR cites.
- [Architecture plan §3](../../plans/architecture.md#3-decisions-to-record-adrs): the ADR list and the round each belongs to.
- [Code organization guideline](../../guidelines/code-organization.md): conventions the ADRs had to agree with.

## Scope

**In scope**

- ADR 0001 (core language, licence, bytes in / bytes out, determinism, HashMap/HashSet disallowed, inbound = outbound contributions).
- ADR 0002 (wasm-bindgen + tsify now; UniFFI later; napi-rs and Component Model deferred; committed `@lamina/wasm` manifest with a gitignored `dist/`; pinned binaryen).
- ADR 0015 (Cargo + pnpm + `cargo xtask`, the tools xtask shells out to, `cargo xtask doctor`, Node pinned via `.nvmrc` + `engines` + `engine-strict`).
- ADR 0019 (cargo-deny, npm licence check, Dependabot (owner decision; Renovate as the alternative), SBOM, fixture provenance).
- The decisions index `docs/decisions/README.md`; architecture §3/§4/§6/§7/§8, roadmap, web, desktop and editor plans updated with the owner's decisions.

**Out of scope**

- Implementing any of it: [A2](./A2-monorepo-skeleton.md) (skeleton, CI), [A3](./A3-port-spike-core.md) (crates).
- Round B ADRs: [B1a](./B1a-round-b-adrs.md).

## Tasks

- [x] Write ADRs 0001, 0002, 0015, 0019 from the research report's "ADRs to record" section
- [x] Add them to `docs/decisions/README.md`
- [x] Record the signing-identity and hosted-service decisions (architecture §8 Q2/Q5, roadmap "Decisions made")
- [x] Address the review findings (contribution clause, wasm-opt feature set, doctor, Node pinning, Dependabot)

## Acceptance criteria

- [x] All four ADRs are `Accepted` and indexed
- [x] Architecture §3 table links each ADR; §8 Q2 and Q5 marked answered

## Tests

None (documentation only).

## Docs to update

- [x] `docs/decisions/README.md`, `docs/plans/architecture.md`, `docs/plans/roadmap.md`, web/desktop/editor plans

## Risks and open questions

None left open; signing eligibility (Windows Artifact Signing is US/Canada only for individuals) is tracked in architecture §6 for ADR 0014.

## Definition of done

- [x] PR opened from a fresh worktree; this file moved `ready/` → `ongoing/` in the PR's first commit (written retroactively: the todo board did not exist yet)
- [x] `/code-review` process followed (findings posted, fixed, threads resolved; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the PR (status `done`), plan links updated, plan to-dos ticked
- [x] Squash-merged; dependants' "Depends on" now satisfied
