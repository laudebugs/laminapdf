# Plan: share compiled Rust between CI runs through sccache on Cloudflare R2

- **Status:** Draft
- **Date:** 2026-11-12
- **Asked for by:** the project owner (2026-11-12): "look at caching for our tests, so as to speed up the pipeline", then "cache the rust builds without turborepo", with the cache on Cloudflare R2.
- **Changes:** [ADR 0018 §2](../decisions/0018-test-strategy.md#2-ci-lanes) (a CI compile cache, and what may never use it)
- **Builds on:** the fast and full lanes ([X4](../todos/done/X4-full-lane-gate.md), [X7](../todos/done/X7-fast-lane-gate.md)), the hosted runners ([X6](../todos/done/X6-windows-ci.md)), and the shared setup actions in `.github/actions/`
- **Todos:** [§7](#7-todos)

## 1. Goal

Every pull request recompiles every crate the workspace owns, on six Rust legs, because the Rust cache we restore today holds only third-party crates. Compiling takes two to five times as long as running the 4,221 Rust tests, and a cache miss adds nine minutes to the macOS x64 leg. This plan adds a second cache that sits under the compiler: sccache looks each `rustc` call up by its inputs in a bucket on Cloudflare R2, so a PR compiles only the crates its change reaches. Pushes to `main` fill the bucket, pull requests only read it, and nothing that is published or deployed is ever built from it.

## 2. What we measured (2026-11-12)

Wall-clock time in CI goes to waiting first and compiling second. A survey of recent runs of `ci.yml` and `full.yml` found:

| Where the time goes | Typical cost |
| --- | --- |
| Waiting for a macOS runner | 10–59 min per job |
| Waiting for `main`'s previous CI run to finish | up to 43 min |
| Compiling in `cargo xtask check rust`, warm Rust cache | 1.5 min (Linux x64) to 4.3 min (macOS x64) |
| Compiling in `cargo xtask check rust`, Rust cache missed | 9.2 min (macOS x64) |
| Running the Rust tests | 50 s (Linux x64) to 1.8 min (macOS x64) |
| Saving the Rust cache on `main` | 1–3 min per leg |
| TypeScript typecheck and tests | 2–4 min, never on the critical path |

The waits need their own work (§8). This plan takes the compiling. The Rust cache (`Swatinem/rust-cache` through `actions-rust-lang/setup-rust-toolchain`) saves `target/` with the workspace's own crates removed, keys it on `Cargo.lock`, and is saved only by pushes to `main`. So a PR always recompiles our crates, and after a change to `Cargo.lock` it falls back to a partial match and recompiles some dependencies too.

## 3. Choices

### 3.1 sccache, not Turborepo or Nx

Turborepo and Nx cache whole tasks: they skip a `test` when none of its declared inputs changed. Their Cargo support is experimental, and [ADR 0015 §5](../decisions/0015-monorepo-layout-and-tooling.md#decision) already rejected them at the root until a measured need. Our need is compile time, which a compiler cache addresses directly. sccache ([mozilla/sccache](https://github.com/mozilla/sccache), Apache-2.0) wraps `rustc` through `RUSTC_WRAPPER`, hashes each call's sources, flags, environment and compiler, and returns the stored output on a hit. It needs no change to the crates or to `cargo xtask`.

### 3.2 R2, not the GitHub Actions cache

sccache can also store into the GitHub Actions cache, which needs no account. That cache holds 10 GB per repository, the Rust caches already fill most of it, and GitHub evicts the oldest entries when it overflows, so two caches there would push each other out. R2 has no such limit, charges nothing for downloads, and sits in the Cloudflare account that already hosts the sites ([ADR 0011](../decisions/0011-cloudflare-topology-and-privacy.md)). sccache's S3 backend speaks to R2 with `SCCACHE_ENDPOINT=https://<account>.r2.cloudflarestorage.com` and `SCCACHE_REGION=auto` ([sccache's S3 docs](https://github.com/mozilla/sccache/blob/main/docs/S3.md)).

### 3.3 Keep the Rust cache during the trial

sccache can't cache crates that call the linker: binaries, test binaries, `cdylib`s (the WebAssembly modules), proc-macros and build scripts. The Rust cache keeps those for dependencies, so the trial keeps it and adds sccache beside it. Only the measurements in X8b decide whether the Rust cache can go, which would save its 1–3 minute save step on `main`.

## 4. Design

### 4.1 One setup action

A composite action, `.github/actions/rust-compile-cache`, sets the cache up in any job that builds Rust:

1. It installs sccache through `.github/actions/install-tools`, at the version pinned in `tools/xtask/src/doctor.rs`.
2. It sets `RUSTC_WRAPPER=sccache`, the bucket, the endpoint, `SCCACHE_REGION=auto` and a key prefix (`rust/v1/`, bumped to empty the cache).
3. It sets `SCCACHE_S3_RW_MODE` to `READ_WRITE` on a push or schedule on `main`, and to `READ_ONLY` everywhere else.
4. Without credentials it prints a notice and changes nothing, so fork PRs and a repository without the bucket build as they do today.

Like `.github/actions/cloudflare-secrets`, it takes the secrets as inputs, because a composite action can't read the `secrets` context. A step after the build prints `sccache --show-stats` into the job summary, so each run shows its hits and misses.

### 4.2 Two keys, and only `main` holds the one that writes

The owner creates two R2 API tokens scoped to the one bucket. A token that can only read goes in repository secrets, where every same-repository PR job can use it. The token that can write goes in a GitHub environment, `rust-compile-cache`, whose deployment rule allows only `main`, so no PR job can get it, even one that edits a workflow. A job picks its token with `environment: ${{ <push or schedule on main> && 'rust-compile-cache' || '' }}`. The read-only mode in §4.1 keeps a PR from trying to write, and the read-only token makes sure it can't.

### 4.3 What never uses the cache

Nothing that is published or deployed is built from the cache. A poisoned entry would otherwise reach users, and the release lane's value is that it builds from source. `release.yml`, `cli-release.yml`, `release-train.yml`, `web-deploy.yml`, `web-release-train.yml`, `docs.yml` and `site.yml` keep compiling from scratch, and a test in `tools/xtask/src/workflows.rs` fails if one of them uses the action. The nightly lane may use the cache, because the release lane reruns everything it publishes from source.

### 4.4 Size and cost

The bucket gets a lifecycle rule that deletes objects 30 days after upload. A dropped entry that is still used is uploaded again by the next push to `main`. While the Rust cache stays, sccache sees only the workspace's own crates, about 30 of the 671 in `Cargo.lock`, so a run makes a few thousand requests. Without the Rust cache, each leg would look up every crate in each of its builds (clippy, tests, the cross-target checks). That comes to tens of thousands of reads per run and some tens of millions a month, about $15–20 a month in R2 read charges past the free 10 million. X8b measures the real counts before choosing.

## 5. Rollout

The trial (X8a) runs the cache on the two legs where compiling costs most and least: Rust on macOS x64 and on Linux x64 in `ci.yml`. It records, for five PR runs and five pushes to `main`, the hit rate from `sccache --show-stats` and the minutes of each step against the runs before it. Then X8b decides:

- **Keep and spread** if warm compiling falls by a third or more on macOS x64. The cache then goes to every Rust job in `ci.yml`, `full.yml` and `nightly.yml`, and X8b tries one leg without the Rust cache.
- **Remove** otherwise. X8b then takes the action and the secrets out and records the numbers here, so nobody tries the same thing without new evidence.

Agents can't push changes to `.github/workflows/`, so each todo's PR carries the workflow edits as a patch for the owner to commit. The action, the pin, the tests and the docs go in the PR itself.

## 6. Risks

- **Clippy.** `cargo clippy` runs our crates through `clippy-driver`. sccache's docs don't say whether it caches those calls; the trial's stats will show it.
- **Deployment entries.** A job with an environment records a deployment, so each push to `main` adds a few to the repository's deployments list. If that list becomes noise, X8b moves the writes to one job.
- **A wrong hit.** sccache keys include the compiler, the target, the flags and the `CARGO_*` environment, and the runners build at the same path on every run of an OS. A false hit would fail a test or build that passes from scratch; `rust/v1/` → `rust/v2/` empties the cache in one commit.
- **R2 outage.** sccache falls back to compiling when the bucket doesn't answer, so CI slows down and keeps working.

## 7. Todos

- [ ] The setup action, the sccache pin, the workflow checks, ADR 0018's amendment and a trial on two legs ([X8a](../todos/ongoing/X8a-rust-compile-cache-trial.md))
- [ ] Spread it to every Rust job or remove it, from the trial's numbers ([X8b](../todos/ready/X8b-rust-compile-cache-everywhere.md))

## 8. Not in this plan

The waits in §2 cost more than compiling and need their own todos: Rust on macOS x64 only on `main` and in the nightly, render goldens only when rendering changes, more of `cargo xtask changes`'s path flags to skip whole jobs, and newer pushes to `main` cancelling an older CI run. Turborepo for the TypeScript packages stays as ADR 0015 left it, to revisit once TypeScript time shows on the critical path.
