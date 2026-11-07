# P6 — The daily release: the release bot, its version commit and pull request

| Field | Value |
| --- | --- |
| ID | `P6` |
| Status | `done` |
| Plan | [releases §3](../../plans/releases.md#3-the-daily-release) |
| Round / milestone | Publishing |
| Depends on | [P4](./P4-docs-match-the-release.md), [P5](./P5-npm-trusted-publishing-and-version-only-releases.md) |
| ADRs | Implements [0017 §5](../../decisions/0017-versioning-and-release.md#5-tags) (amended 2026-11-05) |
| Branch | `build/p6-daily-release` |
| PR title | `build: release once a day after the nightly passes` |
| Size | M (≈ 400–600 lines: xtask, a workflow for the owner, tests, docs) |
| PR | [#271](https://github.com/laudebugs/laminapdf/pull/271) |

## Goal

Release on its own once a day. When a full nightly passes on `main` and changesets are waiting, a workflow cuts the version on the nightly's commit and tags it with the release bot's token, which starts the release lane. Once the lane has published, the version pull request merges the bump into `main`.

## Context

- [Releases plan §3](../../plans/releases.md#3-the-daily-release) and §5's risks; ADR 0017 §5 as amended.
- [P5](../done/P5-npm-trusted-publishing-and-version-only-releases.md)'s version-only guard, which lets the lane skip its nightly run on the version commit; [P4](../done/P4-docs-match-the-release.md)'s docs deploy, which the lane calls after publishing.
- The owner's setup (2026-11-05): a GitHub App with contents and pull-request write on this repository, its ID and key as the secrets `RELEASE_APP_ID` and `RELEASE_APP_PRIVATE_KEY`, and auto-merge allowed.

## Scope

**In scope**

- `cargo xtask release prepare`: the pending changesets on a commit, the next version (Changesets' pre mode), a refusal while a version pull request is open or the tag exists; then `pnpm changeset version` and `version sync`, and the version-only check on the result. Tests.
- `.github/workflows/release-train.yml` (for the owner): on the nightly lane's completion, only for a full run on `main` that succeeded and only while `RELEASE_TRAIN` isn't `off`; the bot's token from `actions/create-github-app-token` (pinned); the branch `release/v<version>`, the commit, the tag.
- The release lane's last job: open the version pull request and turn on its auto-merge with the bot's token, once publishing and the docs deploy succeeded.
- `docs/guidelines/contributing.md`'s release steps, `CLAUDE.md`, and the owner's setup in a README.

**Out of scope**

- A release per merge (the [releases plan §3](../../plans/releases.md#3-the-daily-release) explains why not).

## Tasks

- [x] `cargo xtask release prepare` and its tests
- [x] `release-train.yml` and the release lane's last job, given to the owner as files; xtask's workflow tests cover them
- [x] Docs: contributing, `CLAUDE.md`, the owner's setup

## Acceptance criteria

- [x] With no pending changeset, a passing nightly releases nothing
- [x] A failed or partial nightly, an open version pull request or an existing tag releases nothing; `RELEASE_TRAIN=off` releases nothing

## Post-merge checks

- [ ] With one, a passing nightly ends with the version on crates.io and npm, its docs on `docs.laminapdf.com`, and the bump merged into `main`, with no human step
- [ ] The first daily release publishes and merges its version pull request

## Tests

xtask unit tests (`release prepare` over fixture changesets, the refusals, the workflow's triggers, conditions and permissions).

## Docs to update

- [x] `docs/guidelines/contributing.md`, `CLAUDE.md`, [releases plan §7](../../plans/releases.md#7-to-dos), [architecture: Publishing (later)](../../plans/architecture.md#publishing-later)

## Risks and open questions

- The bot's key is a repository secret ([releases plan §5](../../plans/releases.md#5-risks)).
- A version pull request that conflicts with later merges needs a merge of `main`; the next release waits for it.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process)) — not met: #271 had a manual review only (findings and fixes in its squash message); the `/code-review` and `/writing-review` skills weren't available in that session
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Implementation notes

The implementation and workflow files are in PR #271. The owner installed `release-train.yml` on the PR branch and `release.yml` on `main` (8aa9371); the PR was rebased onto that update. Both installed files match `tools/release/`, and all 811 xtask tests and the docs checks pass locally. The todo stays ongoing pending CI. The first real daily release remains a post-merge check.
