# P1 — Reserve names, ADR 0017 publishing part, OIDC trusted publishing and the release workflow

| Field | Value |
| --- | --- |
| ID | `P1` |
| Status | `ready` — **blocked on the owner's decision to publish** (2026-10-28: crates and npm packages stay local until then) |
| Plan | [architecture: Publishing (later)](../../plans/architecture.md#publishing-later), [pdf-library §6](../../plans/pdf-library.md#6-docs-examples-and-publishing) |
| Round / milestone | Publishing (deferred) |
| Depends on | [B7b](../ongoing/B7b-versioning-and-0-1-0-alpha.md) |
| Blocks | [L4d](./L4d-cli-distribution.md), [P2](./P2-first-registry-prerelease.md) |
| ADRs | Completes **0017** (publishing part: OIDC trusted publishing, provenance, attestations); follows [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (SBOM), [0002](../../decisions/0002-binding-strategy.md) (`@lamina/wasm` packaging) |
| Branch | `build/p1-trusted-publishing` |
| PR title | `build: reserve names and set up OIDC trusted publishing with provenance and attestations` |
| Size | M (≈ 300–600 lines: ADR section, workflow, manifests) |
| PR | — |

## Goal

Prepare publishing so the first release is safe and needs no long-lived tokens: reserve `lamina` and the `lamina-*` crate names on crates.io and the `@lamina` scope on npm (architecture §8 Q3), record the publishing policy in ADR 0017, and add a release workflow (Changesets version PR → publish job) that publishes with OIDC trusted publishing, npm provenance and GitHub artifact attestations for the `.wasm` files — without actually publishing a release yet (that is [P2](./P2-first-registry-prerelease.md)).

## Context

- [Repo and packaging notes §5](../../research/architecture/notes/repo-and-packaging.md#5-versioning-and-release-changesets-release-plz-cargo-release-semver-checks-provenance-trusted-publishing): trusted publishing on npm and crates.io, provenance, attestations; crates.io's first publish must be manual.
- [Report: repository and release](../../research/architecture/app-and-package-architecture.md#the-repository-is-cargo-pnpm-and-a-task-runner-released-through-oidc-and-gated-by-a-pdf-corpus).
- ADR 0017 versioning half from [B7b](../ongoing/B7b-versioning-and-0-1-0-alpha.md).

## Scope

**In scope**

- Name reservation (owner action, documented step by step): crates.io placeholder `0.0.0` publishes for `lamina` and each planned `lamina-*` crate (manual, owner's account, first publish), npm `@lamina` org/scope.
- Switch each crate to trusted-publishing-only on crates.io; configure npm trusted publishers for `@lamina/wasm` and `@lamina/pdf` (GitHub Actions OIDC, repo + workflow file + environment).
- `.github/workflows/release.yml`: on `main`, Changesets action opens/updates the version PR; on merge of the version PR, a `publish` job in a protected `release` environment (required reviewer: owner) runs the release lane suites, builds, `pnpm publish --provenance`, `cargo publish` per crate in dependency order (via OIDC token exchange), uploads `.wasm` and SBOMs with `actions/attest-build-provenance`.
- SBOM generation (CycloneDX for Cargo and npm) attached to GitHub releases.
- ADR 0017 publishing section; `publish = false` removed only for crates meant to be public (keep xtask, test kits, `lamina-wasm` crate private).
- Dry-run mode (`workflow_dispatch` with `dry-run: true` → `cargo publish --dry-run`, `pnpm publish --dry-run`).

**Out of scope** (and which todo covers it)

- Publishing the first pre-release, public stability policy, cargo-semver-checks: [P2](./P2-first-registry-prerelease.md).
- Desktop app signing and release: desktop D6.

## Tasks

- [ ] Get the owner's decision and the final names (architecture §8 Q3)
- [ ] Owner: reserve names (crates.io manual first publish, npm scope); record in the PR
- [ ] Write ADR 0017's publishing part
- [ ] `release.yml` with environments, OIDC permissions (`id-token: write`), attestations, SBOMs; dry-run path
- [ ] Configure trusted publishers on both registries (owner)
- [ ] Update `deny.toml`/manifests (`publish` flags, `repository`, `homepage`, `documentation`, `readme`, `keywords`, `categories`)
- [ ] Run the dry-run workflow on a branch

## Acceptance criteria

- [ ] Names are reserved and owned by the owner's accounts
- [ ] The release workflow's dry run succeeds for every public crate and both npm packages, without any registry token secret in the repository
- [ ] ADR 0017 Accepted in full
- [ ] Attestation and SBOM steps produce artefacts in the dry run

## Post-merge checks

None.

## Tests

Workflow dry run; `cargo publish --dry-run` and `pnpm publish --dry-run` for each package.

## Docs to update

- [ ] [architecture: Publishing (later)](../../plans/architecture.md#publishing-later): tick names, ADR 0017 publishing part, release workflow
- [ ] [architecture §7 Round A](../../plans/architecture.md#round-a--foundations): the struck-through "Reserve package names" item
- [ ] `docs/decisions/README.md`, `CLAUDE.md` (release process), contributing guide

## Risks and open questions

- **Blocked on the owner**; do not start without it.
- crates.io trusted publishing requires the first version to be published manually; placeholders must not mislead users (README: "reserved, not yet released").
- Name availability (`lamina` may be taken): fallback names are an owner decision.

## Definition of done

- [ ] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
