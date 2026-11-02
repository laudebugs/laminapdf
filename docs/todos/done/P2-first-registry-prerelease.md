# P2 — First registry pre-release: 0.2.0-alpha.1, the stability policy, MSRV N-2

| Field | Value |
| --- | --- |
| ID | `P2` |
| Status | `done` |
| Plan | [architecture: Publishing (later)](../../plans/architecture.md#publishing-later), [pdf-library §6](../../plans/pdf-library.md#6-docs-examples-and-publishing) |
| Round / milestone | Publishing |
| Depends on | [P1](./P1-reserve-names-and-trusted-publishing.md) |
| Blocks | [P3](../ready/P3-registry-checks.md) |
| ADRs | Follows [0017](../../decisions/0017-versioning-and-release.md) §2 and §7, [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) |
| Branch | `build/p2-first-prerelease` |
| PR title | `build: release 0.2.0-alpha.1, the first registry pre-release` |
| Size | S in hand-written lines (the version PR bumps every manifest and lockfile) |
| PR | [#184](https://github.com/laudebugs/laminapdf/pull/184) |

## Goal

Publish Lamina PDF for the first time: `0.2.0-alpha.1` of the twelve public crates on crates.io and the six `@laminapdf` packages on npm, through the release lane [P1](../done/P1-reserve-names-and-trusted-publishing.md) built. This PR is the version PR (ADR 0017 §2) and makes the promises that come with a registry release: the stability policy is in force and the crates' MSRV follows N-2. After it merges, the owner tags `v0.2.0-alpha.1` and approves the staged npm versions. [P3](../ready/P3-registry-checks.md) then checks what needs a published version: registry installs, semver checks and the docs site's install pages.

## Context

- The owner's decisions of 2026-11-01: publish; version `0.2.0-alpha.1` (the pending changesets in alpha pre mode); the stability policy as drafted.
- [`docs/stability-policy.md`](../../stability-policy.md) and [`CHANGELOG.md`](../../../CHANGELOG.md), drafted in [B7b](../done/B7b-versioning-and-0-1-0-alpha.md).
- [rust.md: MSRV](../../guidelines/rust.md#toolchain-edition-msrv): N-2 before the first public release.

## Scope

**In scope**

- The version PR: `pnpm changeset version` and `cargo xtask version sync` (every manifest, lockfile and changelog at `0.2.0-alpha.1`); the root changelog's preamble no longer says nothing is published, and names `@laminapdf/viewer`.
- `rust-version = "1.97"` (N-2 of the pinned 1.99): the workspace builds on 1.97 and no dependency needs newer (hayro's 1.92 is the highest); rust.md's N-2 becomes a rule.
- The stability policy in force: no longer a draft, the current names, how to install an alpha, the MSRV rule.

**Out of scope** (and which todo covers it)

- `verify-install --from-registry`, cargo-semver-checks and a TypeScript API diff against the published version, the docs site's install pages and stability page, README badges, docs.rs: [P3](../ready/P3-registry-checks.md).
- npm provenance and attestations: once the repository is public (ADR 0017 §7).

## Tasks

- [x] The owner's approval of the version and the stability policy
- [x] `pnpm changeset version`, `cargo xtask version sync`; the changelog preamble
- [x] MSRV 1.97, checked on 1.97 with every feature
- [x] The stability policy in force

## Acceptance criteria

- [x] `cargo xtask version sync --check` passes at `0.2.0-alpha.1`; `cargo xtask publish crates --dry-run` packages every public crate at it, and `publish npm-pack` and `npm-stage --dry-run` pack and dry-run-stage every npm package
- [x] The `msrv-and-spike` job builds the workspace on 1.97
- [x] CI green

## Post-merge checks

- [ ] A scheduled nightly on `main` passes, so the release guard lets the lane run (the 2026-11-01 one failed on the runner)
- [ ] The owner tags `v0.2.0-alpha.1` on the merge commit; the release lane passes, publishes the crates, stages the npm packages and creates the GitHub release
- [ ] The owner approves the six staged npm versions on npmjs.com, dependencies first (`@laminapdf/wasm`, then `sdk`, `render`, `render-hayro`, `render-pdfium`, `viewer`)
- [ ] `cargo add laminapdf@0.2.0-alpha.1` and `npm install @laminapdf/sdk@alpha` work in a clean project

## Tests

The existing suites at the new version; the MSRV job on 1.97; the publish dry runs.

## Docs to update

- [x] [`docs/stability-policy.md`](../../stability-policy.md), [rust.md](../../guidelines/rust.md), the changelog preamble
- [ ] [architecture: Publishing (later)](../../plans/architecture.md#publishing-later): tick "Publish the first pre-release" once it is out (P3)

## Risks and open questions

- The release lane needs a green scheduled nightly and free disk on the runners.
- Once published, error and warning codes and the op JSON are public contracts (ADR 0003, 0006); the stability policy says so.

## Definition of done

- [x] Claimed: the work PR (#184), opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit
- [x] The change follows the guidelines that apply to it
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
