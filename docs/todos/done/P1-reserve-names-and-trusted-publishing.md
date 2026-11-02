# P1 — Reserve names, ADR 0017 publishing part, OIDC trusted publishing and the release workflow

| Field | Value |
| --- | --- |
| ID | `P1` |
| Status | `done` |
| Plan | [architecture: Publishing (later)](../../plans/architecture.md#publishing-later), [pdf-library §6](../../plans/pdf-library.md#6-docs-examples-and-publishing) |
| Round / milestone | Publishing (deferred) |
| Depends on | [B7b](./B7b-versioning-and-0-1-0-alpha.md), [P0](./P0-npm-scope-laminapdf.md), [P0b](./P0b-crate-names-laminapdf.md), [P0c](./P0c-lamina-pdf-names.md) |
| Blocks | [L4d](../ready/L4d-cli-distribution.md), [P2](./P2-first-registry-prerelease.md) |
| ADRs | Completes **0017** (publishing part: OIDC trusted publishing, provenance, attestations); follows [0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (SBOM), [0002](../../decisions/0002-binding-strategy.md) (`@laminapdf/wasm` packaging) |
| Branch | `build/p1-trusted-publishing` |
| PR title | `build: reserve names and set up OIDC trusted publishing with provenance and attestations` |
| Size | M (≈ 300–600 lines: ADR section, workflow, manifests) |
| PR | [#181](https://github.com/laudebugs/laminapdf/pull/181) |

## Goal

Set up publishing so a `v<version>` tag publishes the library safely, with no long-lived crates.io token. That means reserving `laminapdf` and the `laminapdf-*` crate names on crates.io and the `@laminapdf` scope on npm (chosen 2026-10-31, architecture §8 Q3, [P0](../done/P0-npm-scope-laminapdf.md)), recording the publishing policy in ADR 0017 and adding the release lane's publish jobs. No release is published yet: that is [P2](../done/P2-first-registry-prerelease.md). The owner decided on 2026-11-01 to publish while the repository stays private, which rules out npm provenance, GitHub artifact attestations and a required reviewer for now, and npm's trusted publishing doesn't accept the self-hosted runners.

## Context

- [Repo and packaging notes §5](../../research/architecture/notes/repo-and-packaging.md#5-versioning-and-release-changesets-release-plz-cargo-release-semver-checks-provenance-trusted-publishing): trusted publishing on npm and crates.io, provenance, attestations; crates.io's first publish must be manual.
- [Report: repository and release](../../research/architecture/app-and-package-architecture.md#the-repository-is-cargo-pnpm-and-a-task-runner-released-through-oidc-and-gated-by-a-pdf-corpus).
- ADR 0017 versioning half from [B7b](../done/B7b-versioning-and-0-1-0-alpha.md).

## Scope

**In scope**

- Name reservation (owner, done 2026-11-01): `0.0.0` placeholders for `laminapdf` and every `laminapdf-*` crate on crates.io, owned by `laudebugs`; the `laminapdf` npm organisation.
- crates.io trusted publishing (owner): each public crate's trusted publisher is `laudebugs/laminapdf`, `release.yml`, no environment.
- npm (owner): a granular, stage-only token limited to `@laminapdf`, as the repository secret `NPM_TOKEN`; npm staged publishing, approved on npmjs.com with 2FA.
- `release.yml`: `publish-crates` (crates.io token by OIDC, the only job with `id-token: write`), `publish-npm` (pack and stage) and `github-release` (SBOMs, the release for the tag), all after the `release` job; real only on a tag push, `--dry-run` on a dispatch (even on a tag); a re-run skips what is already published.
- `cargo xtask publish crates|npm|npm-pack|npm-stage [--dry-run]`, `cargo xtask sbom <dir>` (`cargo-cyclonedx` pinned in `doctor.rs`, `pnpm sbom`), `cargo xtask release-notes <tag> <out>`, with tests; the release-lane test that only the publish jobs publish, and only on a tag.
- Manifests: `publish = true`, `readme`, `homepage`, `keywords`, `categories` on the twelve public crates (not `laminapdf-wasm`, `laminapdf-render-wasm`, xtask, std14-gen); the six npm packages public (`publishConfig.access`, `repository`, `homepage`, `keywords`). Packaging fixes: workspace dev-dependencies by path only; `laminapdf`'s `explain` pages generated as raw strings.
- ADR 0017 §7, Accepted.

**Out of scope** (and which todo covers it)

- Publishing the first pre-release, public stability policy, cargo-semver-checks, the MSRV policy for published crates (rust.md's N-2): [P2](../done/P2-first-registry-prerelease.md).
- npm provenance, artifact attestations and a `release` environment: once the repository is public (ADR 0017 §7).
- A Changesets action that opens the version PR: the version PR stays `pnpm changeset version` and `cargo xtask version sync` by hand (ADR 0017 §2).
- Desktop app signing and release: desktop D6.

## Tasks

- [x] Get the owner's decision and the final names (2026-11-01: publish; private repository; `laminapdf-*`, `@laminapdf`)
- [x] Owner: reserve names (crates.io `0.0.0` placeholders, the npm organisation)
- [x] Write ADR 0017's publishing part
- [x] `release.yml`'s publish jobs, OIDC permission for crates.io, SBOMs, the GitHub release; dry-run path
- [x] Owner: crates.io trusted publishers; the stage-only `NPM_TOKEN` secret
- [x] Manifests (`publish`, `readme`, `homepage`, `keywords`, `categories`, npm `publishConfig`)
- [x] `cargo xtask publish crates --dry-run` and `publish npm --dry-run` locally

## Acceptance criteria

- [x] Names are reserved and owned by the owner's accounts
- [x] `cargo xtask publish crates --dry-run` packages and verifies every public crate, and `cargo xtask publish npm --dry-run` packs and dry-run-stages every npm package, with no crates.io token in the repository
- [x] ADR 0017 Accepted in full
- [x] `cargo xtask sbom` writes an SBOM for every public crate and npm package

## Post-merge checks

- [ ] A dispatch of the release lane on `main` (`gh workflow run release.yml --ref main`) passes, publish jobs included, as a dry run
- [ ] The `CRATES_IO_API_TOKEN` repository secret is deleted and the token revoked (trusted publishing replaces it)

## Tests

Unit tests of the publish arguments, dist-tags, tarball names, release notes and the release-lane rule; `cargo xtask publish crates --dry-run`, `publish npm --dry-run` and `sbom` run locally.

## Docs to update

- [x] [architecture: Publishing (later)](../../plans/architecture.md#publishing-later): tick names, ADR 0017 publishing part, release workflow
- [x] [architecture §7 Round A](../../plans/architecture.md#round-a--foundations): the struck-through "Reserve package names" item
- [x] `docs/decisions/README.md`, `CLAUDE.md` (release process), contributing guide

## Risks and open questions

- The npm token is a secret: limited to `@laminapdf`, stage-only, rotated by hand; each version still needs the owner's 2FA approval.
- The release lane runs every suite on the shared Mac runners, so a full disk or a red nightly blocks a release.

## Definition of done

- [x] Claimed: the work PR (#181), opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (the claim rule since 2026-11-01)
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
