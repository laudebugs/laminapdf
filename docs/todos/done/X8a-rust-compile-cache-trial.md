# X8a — sccache on Cloudflare R2: the setup action and a trial on two Rust legs

| Field | Value |
| --- | --- |
| ID | `X8a` |
| Status | `done` |
| Plan | [CI caching §4–§5](../../plans/ci-caching.md#4-design) |
| Round / milestone | Round X (cross-cutting): CI |
| Depends on | None |
| ADRs | Amends [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes); follows [ADR 0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (tool pins, licences) |
| Branch | `ci/x8a-rust-compile-cache-trial` |
| PR title | `ci: share compiled Rust between runs through sccache on R2, on two legs first` |
| Size | S |
| PR | [#507](https://github.com/laudebugs/laminapdf/pull/507) |

## Goal

Every PR recompiles all 24 of the workspace's own crates on six Rust legs, and compiling takes two to five times as long as running the tests. This todo adds the pieces that let a job read compiled crates from a compile cache on Cloudflare R2 through sccache. It turns them on for two legs, Rust on Linux x64 and on macOS x64, so X8b can decide from measurements whether to spread or remove them.

## Context

- [CI caching plan](../../plans/ci-caching.md): the measurements, the choices, the design and the risks this todo builds on. §4.1's order of steps and its handling of the credentials matter most.
- [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes): the lanes this changes.
- `.github/actions/cloudflare-secrets`: the pattern for secrets passed to a composite action, and for skipping when they are missing.
- `.github/actions/install-tools` and `tools/xtask/src/doctor.rs`: how tools are installed and pinned.
- [sccache's S3 docs](https://github.com/mozilla/sccache/blob/main/docs/S3.md): the R2 settings and `SCCACHE_S3_RW_MODE`.

## Scope

**In scope**

- `.github/actions/rust-compile-cache/action.yml` and its README.
- The sccache pin in `tools/xtask/src/doctor.rs`, optional locally.
- Workflow checks in `tools/xtask/src/workflows.rs`.
- ADR 0018's amendment.
- The `ci.yml` change for the two legs (a patch for the owner, if workflow files can't be pushed).

**Out of scope** (and which todo covers it)

- The other Rust jobs, and dropping the Rust cache: [X8b](../ready/X8b-rust-compile-cache-everywhere.md).
- The runner waits: the plan's §8, not yet todos.

## Tasks

- [x] Pin sccache 0.19.0 in `doctor.rs` as an optional tool, with `optional_for` naming CI's compile cache.
- [x] Write the action as the plan's §4.1 lays it out:
  - install through `install-tools`;
  - start the server in one step, with the credentials in that step's `env` only (never `$GITHUB_ENV`);
  - set `READ_WRITE` on a push to `main` and `READ_ONLY` otherwise;
  - export only `RUSTC_WRAPPER` and `SCCACHE_IGNORE_SERVER_IO_ERROR=1`;
  - print a notice and export nothing without credentials;
  - add a `stats: true` input that writes `sccache --show-stats` into the job summary.
- [x] Write the action's README: what it does and the owner's setup.
  - Cloudflare: the bucket, with a lifecycle rule deleting objects 30 days after upload; a read-only and a read-write R2 API token for that bucket alone.
  - GitHub secrets: the read token in repository and Dependabot secrets.
  - GitHub environment: the `rust-compile-cache` environment, limited to `main`, holding the write token.
  - The names the action reads, which the owner creates (not yet set up on 2026-11-12):
    - `SCCACHE_R2_ENDPOINT` and `SCCACHE_R2_BUCKET`: repository variables;
    - `SCCACHE_R2_ACCESS_KEY_ID` and `SCCACHE_R2_SECRET_ACCESS_KEY`: the read token in repository and Dependabot secrets, and the write token under the same names in the `rust-compile-cache` environment, whose secrets take precedence there. A job therefore reads one pair of names and gets the write token only inside the environment.
- [x] Add workflow checks:
  - the action's sccache pin is `doctor.rs`'s;
  - in every job that uses the action, its setup step comes after the toolchain step;
  - the workflows in the plan's §4.3 don't use the action, following each `uses: ./.github/workflows/*.yml` they call.
- [x] Amend ADR 0018 §2: the compile cache, who writes it, and what never uses it.
- [x] Write the `ci.yml` patch for the Linux x64 and macOS x64 Rust legs. It needs the environment expression, the action after the toolchain step, and the stats step at the end. Then hand it to the owner. (The push worked, so it is in the PR.)
- [x] Record the five PR runs and five pushes to `main` before the patch as the baseline: the compile and test minutes of `check rust` on both legs.

## Acceptance criteria

- [x] `cargo xtask check` passes, with the new workflow checks.
- [x] The workflow checks fail on a fixture that uses the action in `nightly.yml`, and on one that sets it up before the toolchain.
- [x] With no credentials, the action prints a notice, and the job's Rust-cache key is unchanged (by design: the action runs after the toolchain step and exports nothing; the logs confirm it after merge, below).

## Post-merge checks

- [x] The bucket, its 30-day rule, both tokens, the variables and the environment set up through Cloudflare's API and `gh` on 2026-11-12.
- [x] The six `SCCACHE_R2_*` secrets written by the owner on 2026-11-12.
- [ ] After that: five PR runs and five pushes to `main` on the two legs, with each job summary's stats, recorded in X8b.
- [ ] A PR run reads in `READ_ONLY` and writes nothing; a push to `main` writes.
- [ ] The two legs' Rust-cache key, in the toolchain step's log, is the same as before the patch for the same `Cargo.lock`, so the compile cache, which comes after that step, leaves it alone.
- [ ] A PR job runs with the empty environment expression (plan §6); if GitHub rejects it, switch to the `rust-compile-cache-read` fallback.

## Tests

`tools/xtask/src/workflows.rs`: the three checks above, each with a failing fixture.

## Docs to update

The plan's to-dos, ADR 0018, `CLAUDE.md` (the layout's `.github/` line and the CI paragraph), `docs/plans/architecture.md`'s CI item.

## Risks and open questions

- GitHub's docs don't say what an `environment:` expression that evaluates to empty does. The first PR run shows it.
- Whether sccache caches `clippy-driver` calls: the stats show it.
- A server that restarts partway through a job runs without credentials (plan §6): check the job summaries.

## Definition of done

- [x] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [x] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Notes

- **Workflow file.** The `ci.yml` change is in the PR itself: pushing over SSH accepted the workflow file. It marks the two trial legs with `compile-cache: true` in the matrix, which turns on the setup step, the stats step and the `rust-compile-cache` environment on pushes to `main`.
- **Baseline.** In the plan's [§2.1](../../plans/ci-caching.md#21-the-trials-baseline-x8a): compiling in `check rust` on macOS x64 PR runs with a Rust-cache hit took a median 12.9 min, so §5's bar is 8.6 min.
- **Credentials.** sccache's server starts `rustc` with the client's environment (`env_clear()` in its `src/compiler/rust.rs`), so build scripts and proc-macros don't get the keys. A same-user process could read the server's environment from the OS; on a pull request that is the read-only token.
- **The never-cached check** follows every local `uses:` from the release, deploy and nightly workflows and fails on any non-comment mention of sccache or the action on the way, which also catches a hand-set `RUSTC_WRAPPER: sccache`. It doesn't check step order inside composite actions.
- **install-action** has sccache 0.19.0 for every runner we use, `aarch64_windows` included (X8b's Windows on Arm64 check).
