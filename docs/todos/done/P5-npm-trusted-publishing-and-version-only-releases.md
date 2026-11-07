# P5 — npm trusted publishing, and a release lane that skips its nightly run for a version-only commit

| Field | Value |
| --- | --- |
| ID | `P5` |
| Status | `done` |
| Plan | [releases §2](../../plans/releases.md#2-what-a-release-needs-today-and-what-changes) |
| Round / milestone | Publishing |
| Depends on | [P2](./P2-first-registry-prerelease.md) |
| ADRs | Implements [0017 §6–§7](../../decisions/0017-versioning-and-release.md#6-the-release-lane) and [0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes)'s Release row (amended 2026-11-05) |
| Branch | `build/p5-npm-trusted-publishing` |
| PR title | `build: publish to npm with trusted publishing and skip the release lane's nightly run for a version-only commit` |
| Size | M (≈ 300–500 lines: xtask, workflow changes for the owner, tests) |
| PR | [#263](https://github.com/laudebugs/laminapdf/pull/263) |

## Goal

Remove the two slow or manual steps from the release lane. npm packages publish with npm's trusted publishing from one short job on a GitHub-hosted runner, with no stored token and no approval on npmjs.com. And the lane skips its own run of the nightly lane when the release commit differs from a commit a full nightly passed on only in the files a version bump writes.

## Context

- [Releases plan §2](../../plans/releases.md#2-what-a-release-needs-today-and-what-changes); ADR 0017 §6–§7 as amended.
- `tools/xtask/src/publish.rs` (`npm-pack`, `npm-stage`), `tools/xtask/src/release.rs` (`release-guard` and the workflow's tests), `.github/workflows/release.yml`.
- npm's trusted publishing needs npm 11.5.1 or later, Node 22.14 or later, `id-token: write`, a cloud-hosted runner and a trusted publisher per package on npmjs.com ([npm's docs](https://docs.npmjs.com/trusted-publishers)).

## Scope

**In scope**

- `cargo xtask publish npm-pack` also writes the publish order (dependencies first) beside the tarballs; they go up as an artifact.
- A `publish-npm` job on `ubuntu-latest` with `id-token: write` that checks out nothing, downloads the artifact, and runs `npm publish <tarball> --access public --tag <pre-release identifier>` in order, leaving out versions already on npm. `npm-stage` and the `NPM_TOKEN` secret go.
- `cargo xtask release-guard --version-only`: passes when the release commit's diff from the commit the nightly passed on touches only version files (the fixed group's `package.json` versions, `Cargo.toml` and `Cargo.lock` versions, `CHANGELOG.md` files, `.changeset/`, `packages/sdk/src/version.ts`); the lane then skips its nightly run. Tests on fixtures of version-only and other diffs.
- `release.rs`' workflow tests updated: two jobs with `id-token: write` (crates and npm), the npm job runs no repository code.

**Out of scope**

- The daily release and the release bot → [P6](../done/P6-daily-release.md).
- npm provenance and artifact attestations: once the repository is public.

## Tasks

- [x] `npm-pack`'s publish order; drop `npm-stage`
- [x] `release-guard --version-only` and its tests
- [x] The `release.yml` changes, given to the owner as a file; `release.rs`' tests pass with it

## Acceptance criteria

- [x] `release-guard --version-only` accepts a version bump on top of a nightly-passed commit and refuses any other change, in tests
- [x] No step that holds a token runs code from the repository

## Post-merge checks

- [ ] A dispatched dry run of the release lane packs the tarballs on the Mac and lists what the GitHub-hosted job would publish, in order
- [ ] The owner adds a trusted publisher to each npm package, then deletes `NPM_TOKEN` after the first release that publishes without it
- [ ] The next release publishes every npm package without an approval on npmjs.com

## Tests

xtask unit tests: the version-only diff rule, the publish order, the workflow's jobs and permissions.

## Docs to update

- [x] `CLAUDE.md` (the release lane paragraph), `docs/guidelines/contributing.md` (releases), [architecture: Publishing (later)](../../plans/architecture.md#publishing-later), [releases plan §7](../../plans/releases.md#7-to-dos)

## Risks and open questions

- The npm job is the project's first on a GitHub-hosted runner; it uses a few minutes a release.
- A trusted publisher configured on npmjs.com but never used may expire; set them up shortly before the first release that needs them.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
