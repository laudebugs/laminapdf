# P2 — First registry pre-release, public stability policy and changelog, cargo-semver-checks

| Field | Value |
| --- | --- |
| ID | `P2` |
| Status | `ready` — **blocked on the owner's decision to publish** |
| Plan | [architecture: Publishing (later)](../../plans/architecture.md#publishing-later), [pdf-library §6](../../plans/pdf-library.md#6-docs-examples-and-publishing) |
| Round / milestone | Publishing (deferred) |
| Depends on | [P1](../ongoing/P1-reserve-names-and-trusted-publishing.md) |
| Blocks | None |
| ADRs | Follows 0017 (complete after [P1](../ongoing/P1-reserve-names-and-trusted-publishing.md)), [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) |
| Branch | `build/p2-first-prerelease` |
| PR title | `build: publish the first pre-release and enable cargo-semver-checks` |
| Size | S–M (≈ 150–400 lines: docs, CI job, changeset) |
| PR | — |

## Goal

Publish lamina for the first time: a `0.x` pre-release of every public crate on crates.io and `@laminapdf/wasm` + `@laminapdf/sdk` on npm through the [P1](../ongoing/P1-reserve-names-and-trusted-publishing.md) workflow, verify installation and the quick start from the registries in each runtime, publish the stability policy and changelog, and from then on check every PR for semver breaks against the published version.

## Context

- [pdf-library §6](../../plans/pdf-library.md#6-docs-examples-and-publishing) and [§8 risks](../../plans/pdf-library.md#8-risks) ("cargo-semver-checks from 0.2").
- `docs/stability-policy.md` and `CHANGELOG.md` drafts from [B7b](../done/B7b-versioning-and-0-1-0-alpha.md).
- [Repo and packaging notes §5](../../research/architecture/notes/repo-and-packaging.md#5-versioning-and-release-changesets-release-plz-cargo-release-semver-checks-provenance-trusted-publishing).

## Scope

**In scope**

- Version PR for the chosen pre-release (likely the current `0.2.0-alpha.N` if [B8b](../done/B8b-conformance-profiles-and-verapdf.md) is merged, else `0.1.0-alpha.N`); publish via the release workflow.
- Registry install verification: extend `cargo xtask verify-install --from-registry` (npm + crates.io) for Node, Deno, Bun, a Vite browser app, workerd and a Rust binary; run in the release lane after publish.
- Publish `docs/stability-policy.md` on the docs site; link the changelog; README badges.
- `cargo-semver-checks` job in the fast lane against the latest published version for public crates (pinned in `doctor.rs`); for TS, `attw` + an API report (`api-extractor` or similar) diffed against the published package.
- Docs site install pages switched from tarballs/path deps to registry installs; docs.rs builds verified.

**Out of scope**

- Python/UniFFI bindings (pdf-library §9 Q5).

## Tasks

- [ ] Owner approval of the version and the stability policy wording
- [ ] Changeset + version PR; merge; release workflow publishes (owner approves the `release` environment)
- [ ] `verify-install --from-registry`; release-lane job
- [ ] Stability policy and changelog on the docs site; README updates
- [ ] cargo-semver-checks and TS API diff jobs

## Acceptance criteria

- [ ] The release workflow's publish steps pass a dry run on the branch (`cargo publish --dry-run` for every crate in dependency order; `pnpm publish --dry-run` for each package), with provenance and attestation settings present
- [ ] `cargo xtask verify-install --from-registry` exists and passes on the branch against local stand-in registries (Verdaccio for npm, a local Cargo alternative registry) populated from this PR's packages, proving the harness before the real publish
- [ ] cargo-semver-checks runs on PRs and fails a deliberate breaking change in a test branch; the TS API diff job fails a deliberate change the same way

## Post-merge checks

- [ ] The release workflow published the version: packages are on crates.io and npm with provenance (npm shows the provenance badge) and attestations on the GitHub release
- [ ] Clean installs from the registries run the quick start in Node 22/24/26, Deno, Bun, Chromium (Vite), workerd and native Rust (`verify-install --from-registry` in the release lane)
- [ ] docs.rs builds the crates
- [ ] The docs site's install pages show registry installs (production deploy)

## Tests

Release-lane install verification; semver-check job.

## Docs to update

- [ ] [architecture: Publishing (later)](../../plans/architecture.md#publishing-later): tick "Publish the first pre-release…" and "Publish the stability policy… cargo-semver-checks"
- [ ] [pdf-library §10 B7](../../plans/pdf-library.md#10-to-dos): tick "Later, when the owner decides: publish…"
- [ ] `README.md`, docs site install pages, `CLAUDE.md`

## Risks and open questions

- **Blocked on the owner.**
- Once published, error/warning codes and op JSON are public contracts (ADR 0003, 0006); the stability policy must say so.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
