# X4 — Full PR lane with an always-running gate job

| Field | Value |
| --- | --- |
| ID | `X4` |
| Status | `done` |
| Plan | [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package) ("Golden-file and corpus test harness in CI (full lane)"), [§7 Cross-cutting](../../plans/architecture.md#cross-cutting) |
| Round / milestone | Cross-cutting (Round B infrastructure) |
| Depends on | [B2c](./B2c-lamina-model-corpus-fuzz.md) |
| Blocks | [B3c](../ready/B3c-save-planner-and-roundtrip.md), [B4a](../ready/B4a-font-loading-and-subsetting.md), [B4b](../ready/B4b-shaping-bidi-line-breaking.md), [B5c](../ready/B5c-lamina-facade-and-examples.md), [B9a](../ready/B9a-lamina-content-and-adr-0023.md) |
| ADRs | Implements [0018](../../decisions/0018-test-strategy.md) §2 (full lane: required check, never skipped by a workflow `paths` filter, changed-paths job + gate job), §4 (no retries in PR lanes) |
| Branch | `ci/x4-full-lane-gate` |
| PR title | `ci: full PR lane with an always-running gate job (X4)` |
| Size | M (≈ 300–600 lines of workflow, xtask and docs) |
| PR | — |

## Goal

Add the second PR lane ADR 0018 defines, without blocking docs-only PRs. The full lane runs on every PR and push to `main`: a first job computes changed paths against the merge base; the suites (fast lane on Linux, macOS and Windows; the fixed-seed 500-mutant differential set; each fuzz target for 2 minutes; and hooks for round-trip, render-golden and runtime jobs that later todos fill) run only when code paths changed; a final **gate** job — the only one marked required — succeeds when every suite passed or was not needed. Pushes to `main` always run everything.

## Context

- [ADR 0018 §2](../../decisions/0018-test-strategy.md#2-ci-lanes): lane table, the "Expected — waiting" problem with path filters, the gate design, pushes to `main` run everything.
- Existing workflows: [`.github/workflows/ci.yml`](../../../.github/workflows/ci.yml), [`select-runner.yml`](../../../.github/workflows/select-runner.yml) (self-hosted preference from PR #11).
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
- [x] `cargo xtask differential --mutants 500` (generate with fixed seed; expectations from B2c `--update`)
- [x] `cargo xtask fuzz --all --time 120` for CI
- [ ] Test on a docs-only PR (gate passes in seconds) and a code PR (suites run)
- [ ] Owner: set `gate` as required in branch protection
- [x] Document

## Acceptance criteria

- [ ] A docs-only PR (only allowlisted paths) shows the full lane's `gate` green within a minute and no suite jobs run
- [ ] A PR touching `crates/` runs all suites; a failing suite turns `gate` red
- [ ] A PR touching only root config (e.g. `rust-toolchain.toml`, `.github/workflows/ci.yml`, `.github/actions/<x>/action.yml`, `.cargo/config.toml`, `.gitattributes`, `biome.json`, `tsconfig.base.json`, `.nvmrc`, `.npmrc`) runs all suites; a unit test (or a `changes`-job dry run over sample path lists) covers every allowlist pattern and checks that each of those config paths, plus an unknown new root file, is classified as code
- [ ] The 500-mutant differential and 2-minute fuzz runs pass on this PR's own full-lane run
- [ ] Full lane completes in ~30 minutes or less

## Post-merge checks

- [ ] The first push to `main` after merge runs every suite regardless of paths, and it is green
- [ ] The owner has marked `gate` as required in branch protection

## Tests

The workflow on two test PRs (docs-only and code), linked in the PR description.

## Docs to update

- [x] [architecture §7 Round B](../../plans/architecture.md#round-b--core-model-and-the-first-package): tick "Golden-file and corpus test harness in CI (full lane)" (fuzz half may already be ticked by B2c)
- [x] ADR 0018 (implementation note), `CLAUDE.md`, [git workflow §5](../../guidelines/git-workflow.md#5-pull-requests) if CI expectations change

## Risks and open questions

- Merge-base computation on shallow clones: fetch with enough depth or use the GitHub API for changed files.
- Windows/macOS minutes cost; consider running them only on `main` pushes if budgets are tight (owner decision; ADR 0018 says every PR).

## Definition of done

- [x] Claimed: a small claim PR moved this file `ready/` → `ongoing/` on `main` (Status `ongoing`, links updated) before work started
- [ ] Work PR opened from a fresh worktree; **PR** field filled in
- [ ] `/code-review` process followed (findings and fixes recorded in commit messages; second review if > 6 findings)
- [ ] CI green
- [x] This file moved `ongoing/` → `done/` in the work PR (Status `done`), links to it updated, plan to-dos ticked, board regenerated

## Notes and deviations

- **Classification lives in xtask.** `cargo xtask changes [--base <rev>]` (`tools/xtask/src/changes.rs`) lists the paths changed since the merge base (`--no-renames`, so a rename out of code counts both paths) and writes `code=true|false` to `$GITHUB_OUTPUT`. Its unit tests cover every allowlist pattern, the root config files listed above, `.github/workflows/full.yml`, `.config/nextest.toml`, lockfiles and an unknown root file. `LICENSE*` matches at the root only. The `changes` job builds xtask with the bare pinned toolchain (no cross targets), as ci.yml's TypeScript job does. It diffs against the PR's own base branch (`origin/${{ github.base_ref }}`), not always `main`.
- **Three OSes without duplicating the fast lane.** The fast lane already runs on a self-hosted macOS runner when one is online (else Ubuntu), so the full lane runs `cargo xtask check` on Windows plus whichever of Linux and macOS the fast lane didn't use (`check-unix`, decided from `select-runner.yml`'s answer). Differential and fuzz run on the `select-runner` choice.
- **The 500-mutant set is separate from the fast set.** It has its own seed (`FULL_SEED`) and is spread evenly over the 14 corpus files (`<stem>-f<i>.pdf`). Its facts are in `fixtures/differential/mutants-500.json` (pypdf 6.21.3), and the ignored test `full_mutant_set_agrees_with_pypdf` checks them; `cargo xtask differential --mutants 500` runs it. 205 of the 500 mutants differ from pypdf on at least one fact. Every difference was sorted by lamina's own warnings and errors into eight ADR 0003 causes, listed as grouped entries (`files = [...]`) in `fixtures/differential/mutants-500-exceptions.toml`: repair where pypdf gives up (136), lamina repairing where pypdf silently fixes an xref entry (21), lamina opening without repair where pypdf fails (6) or warns (2), no catalog (2), a malformed `/Encrypt` (2), skipped page-tree kids (33 page counts) and skipped outline items (6). No lamina bug turned up. The test also fails on an exception whose file isn't in the set.
- **Fuzz** runs the five targets one after another in one job. `--all` runs every target before failing, so a crash in one target doesn't hide another. The corpus is cached per OS and architecture (`fuzz-corpus-pr-…`).
- **nextest `ci` profile.** `retries = 0`, the default slow-timeout and `fail-fast = false`. Both PR lanes set `NEXTEST_PROFILE=ci`. The robustness override in the default profile still applies.
- **Windows.** xtask now finds `pnpm.cmd` on Windows (`program_path`), because `Command` only finds `.exe` files on `PATH` there.
- **Minutes.** The repository is private, so the macOS and Windows jobs bill at higher rates. ADR 0018 says every code PR, so they run on every code PR; moving them to `main` pushes only is the owner's call (see Risks).
- **Branch protection** is an owner action, documented in the PR description: require the `gate` check. Today no status check is required.
