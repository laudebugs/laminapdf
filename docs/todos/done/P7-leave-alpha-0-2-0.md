# P7 — End the alphas: release 0.2.0

| Field | Value |
| --- | --- |
| ID | `P7` |
| Status | `done` |
| Plan | [Releases plan](../../plans/releases.md) |
| Round / milestone | Publishing — the first release after the alphas |
| Depends on | [P6](./P6-daily-release.md) |
| ADRs | Amends [0017](../../decisions/0017-versioning-and-release.md) (§3, 2026-11-11) |
| Branch | `chore/leave-alpha` |
| PR title | `chore(release): end the alphas, so the next release is 0.2.0` |
| Size | S |
| PR | [#471](https://github.com/laudebugs/laminapdf/pull/471) |

## Goal

The owner asked on 2026-11-11 for the next release not to be an alpha, and chose a plain `0.2.0` over a beta. So the next daily release cuts `v0.2.0`: what a plain `npm install @laminapdf/sdk` and `cargo add laminapdf` install, a full GitHub release, and from then on SemVer as the stability policy states it (a breaking change raises the minor version).

## Context

- [ADR 0017 §3](../../decisions/0017-versioning-and-release.md): pre-releases come from Changesets' pre mode, and `pnpm changeset pre exit` ends it.
- [P6](./P6-daily-release.md): the daily release train that cuts the version after the nightly passes.
- [Stability policy](../../stability-policy.md) and its page, `apps/docs/src/content/docs/reference/stability.mdx`.

## Scope

**In scope**

- Leave pre mode, and delete the changesets the alphas consumed so 0.2.0's changelog doesn't repeat them.
- Let the semver check count `0.2.0-alpha.6` to `0.2.0` as a minor step, so the release commit passes.
- Mark a CLI GitHub release as a pre-release only when its version has one.
- Say what `0.x` promises in the docs, the website, the READMEs, the contributing guide and ADR 0017.

**Out of scope**

- 1.0, and any later pre-release line: a new one would re-enter pre mode (`pnpm changeset pre enter`).

## Tasks

- [x] `pnpm changeset pre exit` (`.changeset/pre.json` to `"mode": "exit"`); delete `.changeset/pre/*.md`.
- [x] `.changeset/leave-alpha.md`, a patch entry that says what 0.2.0 means.
- [x] `tools/xtask/src/semver.rs`: `version_step` counts leaving a pre-release as minor, with tests.
- [x] `.github/workflows/cli-release.yml`: `--prerelease` only for a pre-release version.
- [x] Docs: install, first PDF, the docs home, the Rust quick start, stability, the changelog guide, choosing a PDF library; install lines use Cargo's caret.
- [x] Website: `alphaNote` becomes `versionNote`; the README, the SDK and render READMEs, `.changeset/README.md`, contributing, CLAUDE.md and ADR 0017.

## Acceptance criteria

- [x] `pnpm exec changeset status` plans `0.2.0` for the lockstep group.
- [x] `version_step("0.2.0-alpha.6", "0.2.0")` is a minor step.
- [x] The site's and the docs app's tests pass; `docs-prose` and `docs-links` pass.
- [x] Reviewed (standards and spec), and CI green.

## Post-merge checks

- [ ] The next daily release is `v0.2.0`: npm's `latest` tag and crates.io's newest version are `0.2.0`, and its GitHub release isn't marked as a pre-release.

## Tests

`tools/xtask/src/semver.rs` (`version_step_should_name_the_first_part_that_moved`); `apps/site/test/sources.test.ts`.
