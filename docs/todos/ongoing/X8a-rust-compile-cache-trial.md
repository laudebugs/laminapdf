# X8a — sccache on Cloudflare R2: the setup action and a trial on two Rust legs

| Field | Value |
| --- | --- |
| ID | `X8a` |
| Status | `ongoing` |
| Plan | [CI caching §4–§5](../../plans/ci-caching.md#4-design) |
| Round / milestone | Round X (cross-cutting): CI |
| Depends on | None |
| ADRs | Amends [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes); follows [ADR 0019](../../decisions/0019-supply-chain-and-licence-hygiene.md) (tool pins, licences) |
| Branch | `ci/rust-compile-cache` |
| PR title | `ci: share compiled Rust between runs through sccache on R2, on two legs first` |
| Size | S |
| PR | — |

## Goal

Every PR recompiles all of the workspace's own crates on six Rust legs, and compiling takes two to five times as long as running the tests. This todo adds the pieces that let a job read compiled crates from a bucket on Cloudflare R2 through sccache, and turns them on for two legs, Rust on Linux x64 and on macOS x64, so X8b can decide from measurements whether to spread or remove them.

## Context

- [CI caching plan](../../plans/ci-caching.md): the measurements, the choices and the design this todo builds.
- [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes): the lanes this changes.
- [ADR 0015 §5](../../decisions/0015-monorepo-layout-and-tooling.md#decision): why not Turborepo or Nx.
- `.github/actions/cloudflare-secrets`: the pattern for secrets passed to a composite action, and for skipping when they are missing.
- [sccache's S3 docs](https://github.com/mozilla/sccache/blob/main/docs/S3.md): the R2 settings and `SCCACHE_S3_RW_MODE`.

## Scope

**In scope**

- `.github/actions/rust-compile-cache/action.yml`: install, configure, skip without credentials, read-only off `main`.
- The sccache pin in `tools/xtask/src/doctor.rs` (optional locally).
- Workflow checks in `tools/xtask/src/workflows.rs`: the action's pin is doctor's, and no publishing or deploying workflow uses the action.
- ADR 0018's amendment, and the owner's R2 setup steps.
- The patch to `ci.yml` for the two legs, for the owner to commit.

**Out of scope** (and which todo covers it)

- Every other Rust job, and dropping the Rust cache: [X8b](../ready/X8b-rust-compile-cache-everywhere.md).
- The runner waits: the plan's §8, not yet todos.

## Tasks

- [ ] Pin sccache in `doctor.rs` as an optional tool.
- [ ] Write the action, with the stats step as a second action or input.
- [ ] Add the two workflow checks.
- [ ] Amend ADR 0018 §2 and write the setup steps in the action's README.
- [ ] Write `ci.yml`'s patch; the owner creates the bucket, the two tokens, the secrets and the `rust-compile-cache` environment, then commits the patch.

## Acceptance criteria

- [ ] `cargo xtask check` passes, with the new workflow checks.
- [ ] With no credentials, the action prints a notice and the job builds as before.

## Post-merge checks

- [ ] After the owner commits the patch: five PR runs and five pushes to `main` on the two legs, with `sccache --show-stats` in each job summary, recorded in X8b.
- [ ] A PR run shows `READ_ONLY` and no writes; a push to `main` writes.

## Tests

`tools/xtask/src/workflows.rs`: the pin matches `doctor.rs`, and `release.yml`, `cli-release.yml`, `release-train.yml`, `web-deploy.yml`, `web-release-train.yml`, `docs.yml` and `site.yml` don't use the action.

## Docs to update

The plan's to-dos, ADR 0018, `CLAUDE.md` (the layout's `.github/` line and the CI paragraph), `docs/plans/README.md`, `docs/plans/architecture.md`'s CI items.

## Risks and open questions

- The environment chosen by an expression that is empty off `main` is not confirmed for GitHub; the first PR run shows it. The fallback is a second environment, `rust-compile-cache-read`, with no branch rule.
- Whether sccache caches `clippy-driver` calls: the stats show it.

## Definition of done

- [ ] Claimed: the work PR, opened from a fresh worktree, moved this file `ready/` → `ongoing/` in its first commit (Status `ongoing`, **PR** field filled in, links updated, board regenerated) before the work
- [ ] The change follows the guidelines that apply to it: [maintainability](../../guidelines/maintainability.md) for code, [docs writing](../../guidelines/docs-writing.md) and [examples](../../guidelines/examples.md) for what readers see
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings), and `/writing-review` on the PR's prose ([git workflow §7](../../guidelines/git-workflow.md#7-review-and-merge-process))
- [ ] CI green
- [ ] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated
