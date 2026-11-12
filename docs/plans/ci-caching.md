# Plan: share compiled Rust between CI runs through sccache on Cloudflare R2

- **Status:** Draft
- **Date:** 2026-11-12
- **Asked for by:** the project owner (2026-11-12): "look at caching for our tests, so as to speed up the pipeline", then "cache the rust builds without turborepo", with the cache on Cloudflare R2.
- **Changes:** [ADR 0018 §2](../decisions/0018-test-strategy.md#2-ci-lanes) (a CI compile cache, and what may never use it)
- **Builds on:** the fast and full lanes ([X4](../todos/done/X4-full-lane-gate.md), [X7](../todos/done/X7-fast-lane-gate.md)), the hosted runners ([X6](../todos/done/X6-windows-ci.md)), and the shared setup actions in `.github/actions/`
- **Todos:** [§7](#7-todos) (the compile cache) and [§8](#8-the-waits) (the waits)

## 1. Goal

Every pull request recompiles all 24 of the workspace's own crates on six Rust legs, because the Rust cache we restore today holds only third-party crates. Compiling takes two to five times as long as running the 4,221 Rust tests, and a Rust-cache miss adds nine minutes to the macOS x64 leg. This plan adds a compile cache under the compiler: sccache looks each `rustc` call up by its inputs in a bucket on Cloudflare R2, so a PR recompiles only the crates its change reaches. Pushes to `main` fill the compile cache, pull requests only read it, and nothing that is published, deployed or gates a release is ever built from it.

## 2. What we measured (2026-11-12)

Wall-clock time in CI goes to waiting first and compiling second. A survey of recent runs of `ci.yml` and `full.yml` found:

| Where the time goes | Typical cost |
| --- | --- |
| Waiting for a macOS runner | 10–59 min per job |
| Waiting for `main`'s previous CI run (the `concurrency` group queues pushes to `main` one at a time) | up to 43 min |
| Compiling in `cargo xtask check rust`, Rust cache hit | 1.5 min (Linux x64) to 4.3 min (macOS x64) |
| Compiling in `cargo xtask check rust`, Rust cache missed | 9.2 min (macOS x64) |
| Running the Rust tests | 50 s (Linux x64) to 1.8 min (macOS x64) |
| Saving the Rust cache on `main` | 1–3 min per leg |
| TypeScript typecheck and tests | 2–4 min, never on the critical path |

The waits have their own todos (§8). The rest of this plan takes the compiling. The Rust cache (`Swatinem/rust-cache`, through `actions-rust-lang/setup-rust-toolchain`) saves `~/.cargo` and the dependencies in `target/`, never the workspace's crates. Its key hashes `Cargo.lock`, and it falls back to an older lockfile's entry when the lockfile changes, so a PR always recompiles our crates and sometimes some dependencies too.

### 2.1 The trial's baseline (X8a)

Before the compile cache, the first two trial legs of `ci.yml`'s `rust` job took these medians, in minutes, over the five latest PR runs and five latest pushes to `main` in which `cargo xtask check rust` finished on both legs (PR runs 43233720085, 43216833241, 43216820936, 43216814987 and 43216808235; pushes 43220612637, 43213213416, 43189752514, 43184922825 and 43176803902). The step's time comes from the job's step timings. The test time is nextest's summary plus the doc tests' run time. Compiling is the rest, so it includes fmt, the maintainability check and cargo-deny, about half a minute together, and rustdoc compiling the doc tests.

| Leg | Runs | Rust cache | `check rust` | Tests | Compiling | of which clippy | of which test builds |
| --- | --- | --- | --- | --- | --- | --- | --- |
| macOS x64 | 5 PRs | hit (2 exact, 3 an older key) | 15.0 | 2.1 | **12.9** | 3.6 | 5.6 |
| macOS x64 | 5 pushes | missed in 3 | 28.9 | 2.0 | 26.9 | 7.9 | 8.6 |
| Linux x64 | 5 PRs | hit (1 exact, 4 an older key) | 4.7 | 0.9 | 3.9 | 1.3 | 1.4 |
| Linux x64 | 5 pushes | missed in all 5 | 7.8 | 0.7 | 7.1 | 2.8 | 2.4 |

Until 2026-11-12, §5's rule kept the compile cache if this macOS x64 PR median fell to 8.6 minutes or less. The macOS figures are noisy: compiling took 8.4 to 26.7 minutes on PR runs, and four of the five PR runs started within seconds of each other and waited 50 to 120 minutes for a Mac. Pushes to `main` mostly found no Rust cache at all, so their figures are cold builds and don't compare with the PR rows.

The trial's PR measurement moved to macOS Arm64 on 2026-11-12, because the CI waits work in §8 will run Rust on macOS x64 only on pushes to `main` and in the nightly. The macOS Arm64 leg had no compile cache before then. Its medians, in minutes, over PR runs of 2026-11-12 by how they restored the Rust cache, and the five pushes to `main` before the change (runs 43257452966, 43254655568, 43220612637, 43213213416 and 43189752514):

| Leg | Runs | Rust cache | `check rust` | Tests | Compiling | of which clippy | of which test builds |
| --- | --- | --- | --- | --- | --- | --- | --- |
| macOS Arm64 | 5 PRs | hit, exact key | 7.0 | 1.1 | **5.92** | 1.5 | 2.8 |
| macOS Arm64 | 32 PRs | missed | 12.3 | 1.0 | **11.36** | 4.3 | 3.8 |
| macOS Arm64 | 5 pushes | missed in all 5 | 13.4 | 1.0 | 12.6 | 5.2 | 4.3 |

The exact-key PR runs are 43257042096, 43216833241, 43199525601, 43169313504 and 43143748953. Of the 49 PR runs that day, 5 restored the Rust cache by its exact key, 12 by an older key, and 32 found none. The repository's Actions cache sits at its 10 GB limit, and each push to `main` saves about 1 GB of Rust cache per leg, so GitHub evicts entries before the next run reads them. A missed Rust cache is the common case on this leg, and it is where the compile cache can do most, since it also holds the third-party crates.

## 3. Choices

### 3.1 sccache, not Turborepo or Nx

Turborepo and Nx cache whole tasks: they skip a `test` when none of its declared inputs changed. Their Cargo support is experimental, and [ADR 0015 §5](../decisions/0015-monorepo-layout-and-tooling.md#decision) already rejected them at the root until a measured need. Our need is compile time, which a compiler cache addresses directly. sccache ([mozilla/sccache](https://github.com/mozilla/sccache), Apache-2.0, 0.19.0 on 2026-10-24) wraps `rustc` through `RUSTC_WRAPPER`, hashes each call's sources, flags, dependencies, environment and compiler, and returns the stored output on a hit. It needs no change to the crates or to `cargo xtask`.

### 3.2 R2, not the GitHub Actions cache

sccache can also store into the GitHub Actions cache, which needs no account. That cache holds 10 GB per repository, shared with the Rust caches of every leg, and GitHub evicts the least recently used entries when it overflows, so two caches there would push each other out. R2 has no such limit, charges nothing for downloads, and sits in the Cloudflare account that already hosts the sites ([ADR 0011](../decisions/0011-cloudflare-topology-and-privacy.md)). sccache's S3 backend reaches R2 with `SCCACHE_ENDPOINT=https://<account>.r2.cloudflarestorage.com` and `SCCACHE_REGION=auto` ([sccache's S3 docs](https://github.com/mozilla/sccache/blob/main/docs/S3.md)).

### 3.3 Keep the Rust cache during the trial

sccache can't cache any crate that calls the linker ([sccache's README](https://github.com/mozilla/sccache/blob/main/README.md)): binaries, test harnesses (`--test`), integration tests, examples, `cdylib`s such as the WebAssembly modules, proc-macros and build scripts. Doc tests aren't cached either. The Rust cache keeps those for dependencies, so the trial keeps it and adds sccache beside it. Only X8b's measurements decide whether the Rust cache can go, which would save its 1–3 minute save step on `main`.

### 3.4 What to expect

The gain is smaller than "compiling stops". sccache reaches the 22 library crates in each build of `check rust`: clippy (if it caches `clippy-driver`, §6), the cross-target and wasm32 checks, and the libraries that tests link. It never reaches the test harnesses, which are a large share of the compile minutes. Its keys include each dependency's compiled output, so a PR that changes `laminapdf-syntax` or `laminapdf-cos` misses on nearly every crate above them. The trial measures the result; this plan sets no number in advance beyond the rule in §5.

## 4. Design

### 4.1 One setup action

A composite action, `.github/actions/rust-compile-cache`, sets the compile cache up in a job that builds Rust. It runs after the toolchain step, because the Rust cache hashes every `RUST*` variable into its key: a `RUSTC_WRAPPER` set earlier would change the key, and runs without the compile cache (fork and Dependabot PRs) would stop matching `main`'s saves.

1. It installs sccache through `.github/actions/install-tools`, at the version pinned in `tools/xtask/src/doctor.rs`.
2. In one step, with the R2 credentials in that step's `env` only, it starts the sccache server (`sccache --start-server`) with the bucket, the endpoint, `SCCACHE_REGION=auto`, the key prefix `rust/v1/` (bumped to empty the compile cache), `SCCACHE_IDLE_TIMEOUT=0`, and `SCCACHE_S3_RW_MODE` set to `READ_WRITE` on a push to `main` and `READ_ONLY` everywhere else. The server keeps the credentials, so no later step, build script or proc-macro can read them.
3. It then exports only `RUSTC_WRAPPER=sccache` and `SCCACHE_IGNORE_SERVER_IO_ERROR=1`, so a build falls back to plain `rustc` when sccache's server fails.
4. Without credentials it prints a notice and exports nothing, so fork PRs and a repository without the bucket build as they do today.

Like `.github/actions/cloudflare-secrets`, the action takes the secrets as inputs, because a composite action can't read the `secrets` context. A `stats` input adds the reporting: the job's last step runs the action again with `stats: true`, which writes `sccache --show-stats` (hits, misses, and the reasons calls weren't cacheable) into the job summary.

### 4.2 Two tokens, and only `main` holds the one that writes

The owner creates two R2 API tokens scoped to the one bucket. A token that can only read goes in repository secrets and in Dependabot secrets, where every same-repository PR job can use it. The token that can write goes in a GitHub environment, `rust-compile-cache`, whose deployment rule allows only `main`. A PR job runs on `refs/pull/<n>/merge`, so the environment refuses it, even when the PR edits a workflow. A job picks its token with `environment: ${{ <push to main> && 'rust-compile-cache' || '' }}`. The read-only mode in §4.1 keeps a PR from trying to write, the read-only token makes sure it can't, and sccache itself drops to reading when its start-up test write fails.

### 4.3 What never uses the compile cache

Nothing that is published, deployed or gates a release is built from the compile cache, because a poisoned entry would otherwise reach users or wave a release through. These workflows keep compiling from scratch:

- `release.yml`, `cli-release.yml` and `release-train.yml`;
- `web-deploy.yml`, `web-release-train.yml`, `docs.yml` and `site.yml`;
- `nightly.yml`, because the release guard trusts a green nightly, `release.yml` calls it through `workflow_call`, and a version-only release skips that rerun.

A test in `tools/xtask/src/workflows.rs` fails if one of them uses the action, following `uses: ./.github/workflows/*.yml` from each. The rustdoc that `ci.yml` uploads on `main`, which the docs site links to, is built by a leg that uses the compile cache; it is checked output, not a release artefact, and stays there.

### 4.4 Size and cost

The bucket gets a lifecycle rule that deletes objects 30 days after upload. A dropped entry that is still used is uploaded again by the next push to `main`, and anything poisoned ages out on the same clock. The compile cache should hold 10–20 GB across the OS and target combinations, under $0.30 a month past R2's free 10 GB.

Requests cost more than storage. While the Rust cache stays, sccache sees only the 22 library crates in each build, about a hundred reads per leg. Without the Rust cache, each leg would also look up about 650 dependencies in each of its builds. At about 30 pipeline runs a day across nine Rust jobs, that comes to about 30 million reads a month, near $7 a month past the free 10 million at $0.36 per million; writes happen only on `main` misses and stay within the free million. X8b records the real counts before dropping the Rust cache.

## 5. Rollout

The trial (X8a) runs the compile cache on the Rust legs where compiling costs most and least, macOS x64 and Linux x64 in `ci.yml`, and since 2026-11-12 on macOS Arm64, whose PR runs the rule below reads. For PR runs and pushes to `main`, five of each (on macOS Arm64, five PR runs in each Rust-cache state, however many runs that takes; a state with fewer than five by X8b's start doesn't count toward the rule), it records the hits, misses and non-cacheable calls from `sccache --show-stats`, the minutes of each step against the five runs of each kind before the patch, and the crates each PR touched. Then X8b decides:

- **Keep and spread** if the compile minutes of `cargo xtask check rust` on macOS Arm64 PR runs fall by a third or more in at least one Rust-cache state, compared within that state (exact-key hits to 3.95 minutes or less, misses to 7.57 or less, against §2.1's 5.92 and 11.36), and grow in neither. Until 2026-11-12 the rule read macOS x64 PR runs with a Rust-cache hit (8.6 minutes); §8's work takes that leg off pull requests. The compile cache then goes to every Rust job in `ci.yml` and `full.yml`, and X8b tries one leg without the Rust cache.
- **Remove** otherwise. X8b then takes the action and the secrets out and records the numbers here, so nobody tries the same thing without new evidence.

Each todo's PR carries its workflow edits with the action, the pin, the tests and the docs. If the agent's GitHub access can't push workflow files, the PR gives them as a patch for the owner to commit.

## 6. Risks

- **Clippy.** Cargo runs our crates as `sccache clippy-driver rustc …`, and sccache's docs don't say whether it caches those calls. If it doesn't, clippy, the largest compile in `check rust`, gains nothing. The trial's stats will show it.
- **An empty environment.** GitHub's docs don't say what an `environment:` expression that evaluates to empty does; it is a common pattern that runs the job with no environment. The first PR run shows it. If GitHub rejects it, a second environment, `rust-compile-cache-read`, holds the read token with no branch rule.
- **A server that restarts.** If sccache's server stops partway through a job, the next `rustc` call starts a new one without the credentials, and with `SCCACHE_IGNORE_SERVER_IO_ERROR=1` it should compile without the compile cache. The trial checks the job summary for this.
- **Deployment entries.** A job with an environment records a deployment, so each push to `main` adds a few to the repository's deployments list. If that list becomes noise, X8b moves the writes to one job.
- **A wrong hit.** sccache's keys include the compiler, the target, the flags, each dependency's output and the `CARGO_*` environment, and the runners build at the same paths on every run of an OS. A false hit would fail a test or build that passes from scratch; `rust/v1/` → `rust/v2/` empties the compile cache in one commit.
- **Windows on Arm64.** X8b checks that install-action has an sccache build for `aarch64-pc-windows-msvc` before spreading to that leg.

## 7. Todos

- [x] The setup action, the sccache pin, the workflow checks, ADR 0018's amendment and a trial on two legs ([X8a](../todos/done/X8a-rust-compile-cache-trial.md))
- [ ] Spread it to every Rust job of the fast and full lanes, or remove it, from the trial's numbers ([X8b](../todos/ready/X8b-rust-compile-cache-everywhere.md))

## 8. The waits

The waits in §2 cost more than compiling. macOS jobs on PRs waited 27 to 120 minutes for a runner in §2.1's baseline, against about 13 minutes of compiling, and pushes to `main` waited up to 43 minutes for the run before them. Four todos take them, each in its own PR:

- [ ] Rust on macOS x64 only on pushes to `main` and in the nightly, not on PRs: the leg that waits longest for a runner and finishes last. A break only macOS x64 shows then appears after the merge, and the macOS Arm64 leg still runs on every PR, so the owner accepted the risk (2026-11-12). X8b's PR measurements move to macOS Arm64 first ([X9a](../todos/ready/X9a-macos-x64-rust-off-prs.md))
- [ ] The render goldens, which need a Mac, only when a PR changes what they draw ([X9b](../todos/ready/X9b-render-goldens-when-rendering-changes.md))
- [ ] More of `cargo xtask changes`'s path flags, to skip whole groups of jobs a PR can't affect ([X9c](../todos/ready/X9c-skip-jobs-by-changed-paths.md))
- [ ] A newer push to `main` cancels the older CI run on `main` ([X9d](../todos/ready/X9d-cancel-superseded-main-runs.md))

Turborepo for the TypeScript packages stays as ADR 0015 left it, to revisit once TypeScript time shows on the critical path.
