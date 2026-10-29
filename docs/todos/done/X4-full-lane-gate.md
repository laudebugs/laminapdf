# X4 — Full PR lane with an always-running gate job

| Field | Value |
| --- | --- |
| ID | `X4` |
| Status | `done` |
| Plan | [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package) ("Golden-file and corpus test harness in CI (full lane)"), [§7 Cross-cutting](../../plans/architecture.md#cross-cutting) |
| Round / milestone | Cross-cutting (Round B infrastructure) |
| Depends on | [B2c](./B2c-lamina-model-corpus-fuzz.md) |
| Blocks | [B3c](../ongoing/B3c-save-planner-and-roundtrip.md), [B4a](./B4a-font-loading-and-subsetting.md), [B4b](../ready/B4b-shaping-bidi-line-breaking.md), [B5c](../ready/B5c-lamina-facade-and-examples.md), [B9a](../ongoing/B9a-lamina-content-and-adr-0023.md), [X5](./X5-linux-ci-container.md) |
| ADRs | Implements [0018](../../decisions/0018-test-strategy.md) §2 (full lane: required check, never skipped by a workflow `paths` filter, changed-paths job + gate job), §4 (no retries in PR lanes) |
| Branch | `ci/x4-full-lane-gate` |
| PR title | `ci: full PR lane with an always-running gate job (X4)` |
| Size | M (≈ 300–600 lines of workflow, xtask and docs) |
| PR | [#30](https://github.com/laudebugs/lamina/pull/30) |

## Goal

Add the second PR lane ADR 0018 defines, without blocking docs-only PRs. The full lane runs on every PR and push to `main`: a first job computes changed paths against the merge base; the suites (fast lane on Linux, macOS and Windows; the fixed-seed 500-mutant differential set; each fuzz target for 2 minutes; and hooks for round-trip, render-golden and runtime jobs that later todos fill) run only when code paths changed; a final **gate** job — the only one marked required — succeeds when every suite passed or was not needed. Pushes to `main` always run everything.

## Context

- [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes): lane table, the "Expected — waiting" problem with path filters, the gate design, pushes to `main` run everything.
- Existing workflows: [`.github/workflows/ci.yml`](../../../.github/workflows/ci.yml), `select-runner.yml` (self-hosted preference from PR #11; removed when every job moved to the self-hosted runners).
- Mutant generation with a fixed seed: `tests/common` `Rng` and B2c's `cargo xtask differential`.

## Scope

**In scope**

- `.github/workflows/full.yml`: `changes` job (`git diff --name-only $(git merge-base origin/main HEAD)`) that classifies a PR as **docs-only when every changed path matches a short docs allowlist**, and as code otherwise. The allowlist is `docs/**`, `*.md` files anywhere (READMEs, `CLAUDE.md`, `CHANGELOG.md`), `LICENSE*` and `.github/ISSUE_TEMPLATE/**`/`.github/PULL_REQUEST_TEMPLATE.md`. Inverting the filter this way means root config nobody thought to list — `.github/actions/**`, `.cargo/config.toml` (the `xtask` alias), `.gitattributes` (eol rules that decide whether goldens check out byte-identical on Windows), `biome.json`, `tsconfig.base.json`, `.nvmrc`, `.npmrc`, `Cargo.toml`, `rust-toolchain.toml`, `deny.toml`, `.config/**`, `.github/workflows/**`, lockfiles — always runs the suites. Suite jobs are gated by `if: needs.changes.outputs.code == 'true' || github.event_name == 'push'`, and `gate` (`if: always()`) fails if any needed job failed or was cancelled.
- Suites now: `cargo xtask check` on macOS and Windows (Linux is in the fast lane); `cargo xtask differential --mutants 500` against precomputed expectations; `cargo xtask fuzz <each target> --time 120` (nightly toolchain, cached corpus); empty placeholder jobs are **not** added — later todos add their jobs (B3c round-trip, B5c render goldens/qpdf, B4b/B5b external extraction, B6c runtimes) and wire them into `gate`'s `needs`.
- Branch protection: mark `gate` as required (owner action; documented).
- `nextest` profile `ci` with `retries = 0` and the slow-timeout from B2a.
- Docs: ADR 0018 implementation note; `CLAUDE.md` CI section; how to add a job to the gate.

**Out of scope** (and which todo covers it)

- Nightly lane: [X3a](../ongoing/X3a-nightly-lane.md). Release lane: [B7b](../ready/B7b-versioning-and-0-1-0-alpha.md).

## Tasks

- [x] Write `full.yml` with `changes`, suites and `gate`; reuse `select-runner.yml` for Linux, GitHub-hosted for macOS/Windows
- [x] `cargo xtask differential --mutants 500` (generate with fixed seed; expectations from B2c `--update`). Shipped as `cargo xtask differential --full`: 36 mutants per corpus file, 504 today (see Notes)
- [x] `cargo xtask fuzz --all --time 120` for CI
- [x] Test on a code PR (suites run): #30 itself, where every suite ran. The docs-only PR test moved to Post-merge checks (no separate test PR was opened)
- [x] Owner action documented in the PR description; setting `gate` as required is a Post-merge check
- [x] Document

## Acceptance criteria

- [x] A docs-only PR shows `gate` green within a minute with no suite jobs: covered before merge by the `changes.rs` unit tests (every allowlist pattern) and a dry run of the gate script on sample `needs` results; the live check on the first docs-only PR is in Post-merge checks
- [x] A PR touching `crates/` runs all suites; a failing suite turns `gate` red (#30 ran every suite; the red case was checked by a dry run of the gate script on sample `needs` results)
- [x] A PR touching only root config (e.g. `rust-toolchain.toml`, `.github/workflows/ci.yml`, `.github/actions/<x>/action.yml`, `.cargo/config.toml`, `.gitattributes`, `biome.json`, `tsconfig.base.json`, `.nvmrc`, `.npmrc`) runs all suites; a unit test (or a `changes`-job dry run over sample path lists) covers every allowlist pattern and checks that each of those config paths, plus an unknown new root file, is classified as code
- [x] The full differential set (504 mutants) and 2-minute fuzz runs pass on this PR's own full-lane run
- [x] Full lane completes in ~30 minutes or less (measured times in the ADR 0018 implementation note)

## Post-merge checks

- [ ] The first docs-only PR after merge (only allowlisted paths) shows the full lane's `gate` green within a minute, with every suite job skipped
- [ ] The first push to `main` after merge runs every suite regardless of paths, and it is green
- [ ] The owner has marked `gate` as required in branch protection

## Tests

The workflow on this PR (code) and, after merge, on the first docs-only PR; unit tests for the classifier, path portability, exceptions validation and the differential/fuzz runners.

## Docs to update

- [x] [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package): tick "Golden-file and corpus test harness in CI (full lane)" (fuzz half may already be ticked by B2c)
- [x] ADR 0018 (implementation note), `CLAUDE.md`, [git workflow §5](../../guidelines/git-workflow.md#5-pull-requests) if CI expectations change

## Risks and open questions

- Merge-base computation on shallow clones: fetch with enough depth or use the GitHub API for changed files.
- Windows/macOS minutes cost; consider running them only on `main` pushes if budgets are tight (owner decision; ADR 0018 says every PR).

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [x] Work PR opened from a fresh worktree; **PR** field filled in
- [x] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings): the first high review's 25 findings are fixed and listed in one commit; the second, low review runs on the fixes
- [x] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Notes and deviations

- **Classification lives in xtask.** `cargo xtask changes [--base <rev>]` (`tools/xtask/src/changes.rs`; default base `origin/main`, else `main` in the bare-clone layout) lists the paths changed since the merge base (`--no-renames`, so a rename out of code counts both paths) and writes `code=true|false` to `$GITHUB_OUTPUT`. The `changes` job diffs against the PR's own base branch and runs on `edited` too, so a retargeted PR is re-classified.
- **The allowlist is narrower than the Scope's.** Markdown counts as docs only at the repository root (plus `docs/**` and the PR template). Elsewhere it can be a build input (a crate README included as rustdoc and doc tests, the docs site's pages under `apps/docs/`), so it runs the suites. `LICENSE*` matches at the root only. The allowlist is defined once, in `is_docs`, with unit tests for every pattern and every config path listed in the acceptance criteria.
- **Three OSes on GitHub-hosted runners.** `cargo xtask check` runs on Ubuntu, macOS and Windows in the full lane, whichever runner the fast lane picked. A first version ran only the OS the fast lane didn't use, decided by a second runner lookup that could disagree with ci.yml's; review finding A2 removed that race. Differential and fuzz run on the `select-runner` choice.
- **Path portability** (`portable_paths.rs`, in `check docs`): docs-only PRs skip the Windows job, so the fast lane rejects tracked paths Windows or a case-insensitive file system can't check out.
- **The full differential set** is 36 mutants per corpus file (`FULL_PER_FILE`; 504 for the 14 files today, at least ADR 0018's 500), from its own seed, run by `cargo xtask differential --full` through nextest (`--no-tests fail`, `ci` profile). A fixed count per file means a new corpus file adds only its own mutants. Facts: `fixtures/differential/full.json` (pypdf 6.21.3). 206 of the 504 mutants differ from pypdf on at least one fact. All were sorted by lamina's own warnings and errors into eight ADR 0003 causes, as grouped entries in `full-exceptions.toml`: repair where pypdf gives up (136), lamina repairing where pypdf silently fixes an xref entry (21), lamina opening without repair where pypdf fails (6) or warns (2), no catalog (2), a malformed `/Encrypt` (2), skipped page-tree kids (34 page counts) and skipped outline items (6). No lamina bug turned up. Going from 500 to 504 mutants added 4 (`reportlab-f35`, `signed-f35`, `tampered-f35`, `text-example-f35`) and left the other 500 byte-identical; only `reportlab-f35` disagrees, and it joined the page-tree group. The exceptions parser rejects empty lists, unknown facts and a pair listed twice; the test fails on an exception for a file outside the set.
- **Fuzz** runs the five targets in one job. `--all` runs every target before failing and reports each failure as a crash (new files in `fuzz/artifacts/<target>/`) or a build/setup problem. Runs restore the corpus last saved on `main`; only pushes to `main` save it, after `cargo xtask fuzz cmin`.
- **nextest `ci` profile.** `retries = 0`, the default slow-timeout and `fail-fast = false`. Both PR lanes set `NEXTEST_PROFILE=ci`. The robustness override in the default profile still applies.
- **Windows.** xtask finds `pnpm.cmd` on Windows (`program_path`), because `Command` only finds `.exe` files on `PATH` there.
- **Shared setup.** `.github/actions/host-rust` installs the bare pinned toolchain for jobs that only build xtask (ci.yml's TypeScript job and `changes`).
- **Minutes.** The repository is private, so the macOS and Windows jobs bill at higher rates. ADR 0018 says every code PR, so they run on every code PR; moving them to `main` pushes only is the owner's call (see Risks).
- **Branch protection** is an owner action, documented in the PR description: require the `gate` check. Today no status check is required.
